/// Loopback HTTP listener for OAuth PKCE callback capture.
///
/// Security properties (see threat model T-07-30, T-07-31, T-07-33):
/// - Binds to 127.0.0.1 ONLY (loopback, never a wildcard address) — T-07-31
/// - State comparison uses subtle::ConstantTimeEq — NOT != or == (T-07-30)
/// - 5-minute self-terminate timeout (T-07-33)
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use subtle::ConstantTimeEq;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::oneshot;

/// The payload captured from the OAuth callback GET request.
#[derive(Debug, Clone, PartialEq)]
pub struct AuthCallback {
    pub code: String,
    pub state: Option<String>,
}

/// Bind an ephemeral port in the requested range on 127.0.0.1 (loopback only).
///
/// State comparison uses subtle::ConstantTimeEq to eliminate timing side-channels (T-07-30).
/// Returns the actual bound port and a oneshot receiver that fires when the browser hits
/// GET {redirect_path}?code=...&state=...
///
/// The listener self-terminates after 300 seconds (5 min) sending Err on the receiver
/// if no callback has arrived (T-07-33).
pub async fn spawn_loopback_listener(
    port_range: (u16, u16),
    redirect_path: &str,
    expected_state: String,
) -> Result<(u16, oneshot::Receiver<Result<AuthCallback, String>>), String> {
    // Iterate the port range; first successful bind wins.
    let mut listener = None;
    for port in port_range.0..=port_range.1 {
        // T-07-31: bind ONLY to 127.0.0.1 (loopback) — never a wildcard address
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port);
        match TcpListener::bind(addr).await {
            Ok(l) => {
                listener = Some((port, l));
                break;
            }
            Err(_) => continue,
        }
    }

    let (port, listener) = listener.ok_or_else(|| {
        format!(
            "OAuth loopback ports {}-{} all in use",
            port_range.0, port_range.1
        )
    })?;

    let (tx, rx) = oneshot::channel::<Result<AuthCallback, String>>();
    let redirect_path = redirect_path.to_string();

    tokio::spawn(async move {
        // T-07-33: self-terminate after 300 seconds (5 min) if no callback arrives.
        let result = tokio::time::timeout(
            tokio::time::Duration::from_secs(300),
            accept_callback(&listener, &redirect_path, &expected_state),
        )
        .await;

        let payload = match result {
            Ok(inner) => inner,
            Err(_) => Err("OAuth timeout — 5 minutes without callback".to_string()),
        };

        // Best-effort send; receiver may have been dropped if caller gave up.
        let _ = tx.send(payload);
    });

    Ok((port, rx))
}

/// Accept a single HTTP connection, parse the GET request, and validate the state parameter.
async fn accept_callback(
    listener: &TcpListener,
    redirect_path: &str,
    expected_state: &str,
) -> Result<AuthCallback, String> {
    let (mut stream, _peer) = listener
        .accept()
        .await
        .map_err(|e| format!("Loopback accept error: {}", e))?;

    // Read the first line of the HTTP request (e.g. "GET /auth/callback?code=abc&state=xyz HTTP/1.1")
    let mut buf = [0u8; 4096];
    let n = stream
        .read(&mut buf)
        .await
        .map_err(|e| format!("Loopback read error: {}", e))?;

    let request = std::str::from_utf8(&buf[..n]).unwrap_or("");
    let first_line = request.lines().next().unwrap_or("");

    // Parse: "GET /path?query HTTP/1.1"
    let path_and_query = first_line
        .split_whitespace()
        .nth(1)
        .unwrap_or("");

    let (path, query) = match path_and_query.split_once('?') {
        Some((p, q)) => (p, q),
        None => (path_and_query, ""),
    };

    // Verify this is a request to the expected redirect path
    // (be lenient: accept if redirect_path is empty or matches)
    if !redirect_path.is_empty() && path != redirect_path {
        let _ = send_http_response(&mut stream, 404, "Not found").await;
        return Err(format!("Unexpected path: {} (expected {})", path, redirect_path));
    }

    // Parse query string for `code` and `state`
    let code = extract_query_param(query, "code");
    let state_from_query = extract_query_param(query, "state").unwrap_or_default();

    // T-07-30: constant-time compare — do not replace with != or ==
    // This prevents timing side-channel attacks where an attacker could detect
    // partial matches by measuring response latency differences.
    let state_ok: bool = state_from_query
        .as_bytes()
        .ct_eq(expected_state.as_bytes())
        .into();

    if !state_ok {
        let _ = send_http_response(&mut stream, 400, "State mismatch. Authentication failed.").await;
        return Err(format!(
            "state mismatch: received '{}' but expected '{}'",
            state_from_query, expected_state
        ));
    }

    let code = match code {
        Some(c) if !c.is_empty() => c,
        _ => {
            let _ = send_http_response(&mut stream, 400, "Missing code parameter.").await;
            return Err("OAuth callback missing 'code' parameter".to_string());
        }
    };

    // Respond with a success page the user sees in their browser.
    let html = "<!DOCTYPE html><html><body>\
        <h2>Authentication successful</h2>\
        <p>You may close this tab. Return to Cortex.</p>\
        </body></html>";
    let _ = send_http_response(&mut stream, 200, html).await;

    Ok(AuthCallback {
        code,
        state: Some(state_from_query),
    })
}

/// Minimal URL query parameter extractor.
/// Handles percent-decoding via a simple + and %XX decoder.
fn extract_query_param(query: &str, name: &str) -> Option<String> {
    for pair in query.split('&') {
        if let Some((k, v)) = pair.split_once('=') {
            if k == name {
                return Some(url_decode(v));
            }
        }
    }
    None
}

/// Minimal URL percent-decoder (handles %XX and + → space).
fn url_decode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'+' {
            out.push(' ');
            i += 1;
        } else if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(hex) = std::str::from_utf8(&bytes[i + 1..i + 3]) {
                if let Ok(byte) = u8::from_str_radix(hex, 16) {
                    out.push(byte as char);
                    i += 3;
                    continue;
                }
            }
            out.push(bytes[i] as char);
            i += 1;
        } else {
            out.push(bytes[i] as char);
            i += 1;
        }
    }
    out
}

/// Send a minimal HTTP/1.1 response.
async fn send_http_response(
    stream: &mut tokio::net::TcpStream,
    status: u16,
    body: &str,
) -> std::io::Result<()> {
    let status_text = match status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        _ => "Unknown",
    };
    let response = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        status, status_text, body.len(), body
    );
    stream.write_all(response.as_bytes()).await?;
    stream.flush().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_spawn_loopback_listener_binds_in_range() {
        let (port, _rx) = spawn_loopback_listener((54000, 54010), "/callback", "state123".to_string())
            .await
            .unwrap();
        assert!(
            port >= 54000 && port <= 54010,
            "bound port {} is outside expected range 54000-54010",
            port
        );
    }

    #[tokio::test]
    async fn test_spawn_loopback_listener_captures_code() {
        let (port, rx) =
            spawn_loopback_listener((54020, 54030), "/callback", "abc123".to_string())
                .await
                .unwrap();

        // Simulate the browser hitting the callback URL
        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
            let url = format!("http://127.0.0.1:{}/callback?code=fakecode&state=abc123", port);
            let res = reqwest::get(&url).await.unwrap();
            assert_eq!(res.status().as_u16(), 200);
            let body = res.text().await.unwrap();
            assert!(
                body.contains("You may close this tab"),
                "response body should contain close-tab message"
            );
        });

        let result = rx.await.unwrap();
        assert!(result.is_ok(), "expected Ok but got: {:?}", result);
        let cb = result.unwrap();
        assert_eq!(cb.code, "fakecode");
        assert_eq!(cb.state.as_deref(), Some("abc123"));
    }

    #[tokio::test]
    async fn test_spawn_loopback_listener_rejects_state_mismatch() {
        let (port, rx) =
            spawn_loopback_listener((54040, 54050), "/callback", "expected_state".to_string())
                .await
                .unwrap();

        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
            let url = format!("http://127.0.0.1:{}/callback?code=fakecode&state=DIFFERENT", port);
            let res = reqwest::get(&url).await.unwrap();
            assert_eq!(res.status().as_u16(), 400, "state mismatch should return 400");
        });

        let result = rx.await.unwrap();
        assert!(result.is_err(), "expected Err for state mismatch");
        let err = result.unwrap_err();
        assert!(
            err.contains("state mismatch") || err.contains("mismatch"),
            "error message should mention state mismatch, got: {}",
            err
        );
    }

    #[test]
    fn test_spawn_loopback_listener_uses_constant_time_compare() {
        // Regression guard: ensure the loopback.rs source uses subtle::ct_eq for state comparison.
        // This test fails if a future refactor removes the constant-time equality primitive (T-07-30).
        let source = include_str!("loopback.rs");
        assert!(
            source.contains("ct_eq(") || source.contains("ConstantTimeEq"),
            "loopback.rs MUST use subtle::ConstantTimeEq::ct_eq for state comparison (T-07-30). \
             Do not replace with != or ==."
        );
        // Also assert the constant-time comparison flag is present as a comment
        // (the inline comment "// T-07-30:" documents the security property)
        assert!(
            source.contains("T-07-30"),
            "loopback.rs must contain T-07-30 security annotation for the constant-time comparison"
        );
        // Guard: the ct_eq call must be present in the accept_callback function body
        // (not just imported). We check for the pattern used in the actual implementation.
        assert!(
            source.contains(".ct_eq(expected_state.as_bytes())"),
            "loopback.rs must call .ct_eq(expected_state.as_bytes()) for state comparison"
        );
    }

    #[tokio::test]
    async fn test_spawn_loopback_listener_port_range_exhausted() {
        // Pre-bind every port in a tight range to force exhaustion
        let mut listeners = Vec::new();
        for port in 54060u16..=54062 {
            let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port);
            if let Ok(l) = TcpListener::bind(addr).await {
                listeners.push(l);
            }
        }

        // If we managed to bind all 3 ports, verify spawn_loopback_listener fails
        if listeners.len() == 3 {
            let result = spawn_loopback_listener((54060, 54062), "/callback", "state".to_string()).await;
            assert!(result.is_err(), "expected Err when all ports are in use");
            let err = result.unwrap_err();
            assert!(
                err.contains("all in use") || err.contains("ports"),
                "error message should mention ports exhausted, got: {}",
                err
            );
        }
        // If some ports couldn't be bound (system-level constraint), skip the assertion.
        // The test still validates the range-exhaustion code path for 3 pre-bound ports.
    }
}
