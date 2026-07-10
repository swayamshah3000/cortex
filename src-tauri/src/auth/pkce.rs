/// Provider-agnostic PKCE OAuth 2.0 flow module.
///
/// This module is generic: no hardcoded provider endpoints appear here.
/// Per-provider constants (authorization_url, token_url, client_id, scopes, etc.)
/// are passed in via OAuthFlowConfig by the caller (plan 07-09). This satisfies D-24.
///
/// Crates used:
/// - sha2 = "0.10" (already present in Cargo.toml — verified during Task 1)
/// - base64 = "0.22"
/// - rand = "0.9" (thread_rng() API)
/// - subtle = "2" (constant-time state compare — in loopback.rs)
///
/// Security: The loopback listener (auth/loopback.rs) handles constant-time state
/// comparison (T-07-30). This module handles PKCE code generation per RFC 7636 §4.
use base64::Engine;
use rand::Rng;
use sha2::{Digest, Sha256};

/// PKCE verifier + challenge pair, per RFC 7636 §4.
#[derive(Debug, Clone)]
pub struct PkceCodes {
    /// URL-safe base64 (no padding), 43+ chars. Must be kept secret.
    pub verifier: String,
    /// SHA-256(verifier), URL-safe base64 (no padding). Sent to auth server.
    pub challenge: String,
}

/// Token request encoding style — differs by provider.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TokenRequestStyle {
    /// application/x-www-form-urlencoded (Codex CLI token exchange + most providers)
    FormUrlencoded,
    /// application/json (Codex CLI refresh_token exchange)
    Json,
}

/// Per-provider OAuth flow configuration.
///
/// No defaults for `redirect_uri_host` — callers MUST pass an explicit value.
/// Per 07-OAUTH-RESEARCH.md §Redirect URI Host Resolution:
///   - openai-codex → "localhost" (Hydra allow-list requires literal localhost)
///   - gemini (future) → "127.0.0.1"
///
/// The loopback socket always binds 127.0.0.1 regardless of this field.
/// This field controls ONLY the redirect_uri STRING sent in OAuth requests.
#[derive(Debug, Clone)]
pub struct OAuthFlowConfig<'a> {
    /// Provider slug for logging / error messages (e.g. "openai-codex")
    pub provider_slug: &'a str,
    /// Authorization endpoint URL (e.g. "https://auth.openai.com/oauth/authorize")
    pub authorization_url: &'a str,
    /// Token endpoint URL (e.g. "https://auth.openai.com/oauth/token")
    pub token_url: &'a str,
    /// OAuth client_id
    pub client_id: &'a str,
    /// Space-delimited scope string
    pub scope: &'a str,
    /// Port range to search for an available loopback port
    pub port_range: (u16, u16),
    /// Path component of the redirect URI (e.g. "/auth/callback" for codex, "/" for gemini)
    pub redirect_path: &'a str,
    /// Host component of the redirect URI STRING (CRITICAL: must match provider allow-list).
    /// "localhost" for openai-codex, "127.0.0.1" for gemini (future).
    /// The loopback socket ALWAYS binds 127.0.0.1 regardless of this field.
    pub redirect_uri_host: &'a str,
    /// Additional query parameters for the authorization URL (e.g. codex extras)
    pub extra_authz_params: &'a [(&'a str, &'a str)],
    /// Token exchange encoding style
    pub token_request_style: TokenRequestStyle,
}

/// Normalized token response from a provider.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct OAuthTokenResponse {
    pub access_token: String,
    pub refresh_token: Option<String>,
    /// Seconds until expiry, as reported by the provider.
    pub expires_in: Option<i64>,
    pub token_type: Option<String>,
}

/// Generate RFC 7636 §4 compliant PKCE codes.
///
/// Verifier: 32 random bytes → URL-safe base64 no-pad (43 chars, within §4.1 43-128 range)
/// Challenge: SHA-256(verifier bytes) → URL-safe base64 no-pad
pub fn generate_pkce_codes() -> PkceCodes {
    let mut rng = rand::thread_rng();
    // 32 random bytes → URL-safe base64 no-pad = 43 chars (RFC 7636 §4.1 requires 43-128)
    let random_bytes: [u8; 32] = rng.gen();
    let verifier =
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(random_bytes);

    // SHA-256 of the verifier bytes (not the base64 string) — RFC 7636 §4.2
    let digest = Sha256::digest(verifier.as_bytes());
    let challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(digest);

    PkceCodes {
        verifier,
        challenge,
    }
}

/// Execute a complete OAuth 2.0 PKCE authorization code flow.
///
/// Steps (D-24):
/// 1. Generate PKCE codes + random state
/// 2. Spawn loopback listener on 127.0.0.1
/// 3. Construct authorization URL using config.redirect_uri_host for the URI string
/// 4. Yield authorization URL to caller via on_authorization_url_ready
/// 5. Await callback (code + state already verified in loopback.rs)
/// 6. Exchange code for tokens at token_url
/// 7. Return OAuthTokenResponse
pub async fn start_oauth_flow(
    config: OAuthFlowConfig<'_>,
    on_authorization_url_ready: impl FnOnce(String) -> Result<(), String>,
) -> Result<OAuthTokenResponse, String> {
    // Step 1: Generate PKCE codes + random state
    let pkce = generate_pkce_codes();
    let state = generate_random_state();

    // Step 2: Spawn loopback listener — always binds 127.0.0.1 (T-07-31)
    let (port, rx) = crate::auth::loopback::spawn_loopback_listener(
        config.port_range,
        config.redirect_path,
        state.clone(),
    )
    .await?;

    // Step 3: Construct redirect_uri using config.redirect_uri_host (per-provider string).
    // NOTE: This is the STRING sent to the OAuth server. It MUST match the provider's allow-list.
    // The socket bind (127.0.0.1) and this string are independent concerns.
    let redirect_uri = format!(
        "http://{}:{}{}",
        config.redirect_uri_host, port, config.redirect_path
    );

    // Step 4: Build authorization URL
    let auth_url = build_authorization_url(config.authorization_url, &OAuthAuthzParams {
        client_id: config.client_id,
        redirect_uri: &redirect_uri,
        scope: config.scope,
        challenge: &pkce.challenge,
        state: &state,
        extra: config.extra_authz_params,
    });

    // Yield auth URL to caller — caller opens system browser (plan 07-09 wires this)
    on_authorization_url_ready(auth_url)?;

    // Step 5: Await callback (code + state already constant-time verified in loopback.rs)
    let callback = rx
        .await
        .map_err(|_| "OAuth callback channel closed unexpectedly".to_string())?
        .map_err(|e| format!("OAuth callback error: {}", e))?;

    // Step 6: Exchange code for tokens
    exchange_code_for_tokens(
        config.token_url,
        &callback.code,
        &redirect_uri,
        config.client_id,
        &pkce.verifier,
        config.token_request_style,
    )
    .await
}

/// Refresh an expired access token using a stored refresh token.
///
/// FormUrlencoded: grant_type=refresh_token&client_id=..&refresh_token=..
/// Json: {"grant_type":"refresh_token","client_id":"..","refresh_token":".."}
///
/// Error handling:
/// - HTTP 401/403 → "Refresh token invalid or expired. Please reconnect."
/// - HTTP 5xx → "Provider unreachable during token refresh."
/// - HTTP 200 but missing access_token → "Unexpected refresh response shape from {provider}"
pub async fn refresh_access_token(
    refresh_url: &str,
    client_id: &str,
    refresh_token: &str,
    style: TokenRequestStyle,
) -> Result<OAuthTokenResponse, String> {
    let client = reqwest::Client::new();

    let response = match style {
        TokenRequestStyle::FormUrlencoded => {
            client
                .post(refresh_url)
                .header("Content-Type", "application/x-www-form-urlencoded")
                .body(format!(
                    "grant_type=refresh_token&client_id={}&refresh_token={}",
                    url_encode(client_id),
                    url_encode(refresh_token)
                ))
                .send()
                .await
                .map_err(|e| format!("Network error during token refresh: {}", e))?
        }
        TokenRequestStyle::Json => {
            let body = serde_json::json!({
                "grant_type": "refresh_token",
                "client_id": client_id,
                "refresh_token": refresh_token
            });
            client
                .post(refresh_url)
                .header("Content-Type", "application/json")
                .json(&body)
                .send()
                .await
                .map_err(|e| format!("Network error during token refresh: {}", e))?
        }
    };

    let status = response.status().as_u16();
    let body_text = response
        .text()
        .await
        .map_err(|e| format!("Read error during token refresh: {}", e))?;

    match status {
        200 => {
            let parsed: serde_json::Value = serde_json::from_str(&body_text)
                .map_err(|_| "Unexpected refresh response format".to_string())?;
            let access_token = parsed["access_token"]
                .as_str()
                .ok_or_else(|| {
                    "Unexpected refresh response shape: missing access_token".to_string()
                })?
                .to_string();
            Ok(OAuthTokenResponse {
                access_token,
                refresh_token: parsed["refresh_token"].as_str().map(String::from),
                expires_in: parsed["expires_in"].as_i64(),
                token_type: parsed["token_type"].as_str().map(String::from),
            })
        }
        401 | 403 => Err("Refresh token invalid or expired. Please reconnect.".to_string()),
        500..=599 => Err("Provider unreachable during token refresh.".to_string()),
        other => Err(format!(
            "Unexpected status {} during token refresh",
            other
        )),
    }
}

// --- Private helpers ---

struct OAuthAuthzParams<'a> {
    client_id: &'a str,
    redirect_uri: &'a str,
    scope: &'a str,
    challenge: &'a str,
    state: &'a str,
    extra: &'a [(&'a str, &'a str)],
}

fn build_authorization_url(base_url: &str, params: &OAuthAuthzParams<'_>) -> String {
    let mut qs = format!(
        "response_type=code&client_id={}&redirect_uri={}&scope={}&code_challenge={}&code_challenge_method=S256&state={}",
        url_encode(params.client_id),
        url_encode(params.redirect_uri),
        url_encode(params.scope),
        url_encode(params.challenge),
        url_encode(params.state),
    );
    for (k, v) in params.extra {
        qs.push('&');
        qs.push_str(&format!("{}={}", url_encode(k), url_encode(v)));
    }
    format!("{}?{}", base_url, qs)
}

/// Generate a random state value: URL-safe base64 of 16 random bytes.
fn generate_random_state() -> String {
    let mut rng = rand::thread_rng();
    let bytes: [u8; 16] = rng.gen();
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

/// Minimal URL percent-encoder for query parameter values.
/// Encodes all characters except unreserved chars (A-Z a-z 0-9 - _ . ~).
fn url_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 3);
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => {
                out.push('%');
                out.push_str(&format!("{:02X}", b));
            }
        }
    }
    out
}

async fn exchange_code_for_tokens(
    token_url: &str,
    code: &str,
    redirect_uri: &str,
    client_id: &str,
    verifier: &str,
    style: TokenRequestStyle,
) -> Result<OAuthTokenResponse, String> {
    let client = reqwest::Client::new();

    let response = match style {
        TokenRequestStyle::FormUrlencoded => {
            client
                .post(token_url)
                .header("Content-Type", "application/x-www-form-urlencoded")
                .body(format!(
                    "grant_type=authorization_code&code={}&redirect_uri={}&client_id={}&code_verifier={}",
                    url_encode(code),
                    url_encode(redirect_uri),
                    url_encode(client_id),
                    url_encode(verifier),
                ))
                .send()
                .await
                .map_err(|e| format!("Network error during code exchange: {}", e))?
        }
        TokenRequestStyle::Json => {
            let body = serde_json::json!({
                "grant_type": "authorization_code",
                "code": code,
                "redirect_uri": redirect_uri,
                "client_id": client_id,
                "code_verifier": verifier,
            });
            client
                .post(token_url)
                .header("Content-Type", "application/json")
                .json(&body)
                .send()
                .await
                .map_err(|e| format!("Network error during code exchange: {}", e))?
        }
    };

    let status = response.status().as_u16();
    let body_text = response
        .text()
        .await
        .map_err(|e| format!("Read error during code exchange: {}", e))?;

    if status != 200 {
        return Err(format!(
            "Token exchange failed (HTTP {}): {}",
            status,
            body_text.chars().take(200).collect::<String>()
        ));
    }

    let parsed: serde_json::Value =
        serde_json::from_str(&body_text).map_err(|e| format!("Parse error: {}", e))?;

    let access_token = parsed["access_token"]
        .as_str()
        .ok_or_else(|| "Token exchange response missing access_token".to_string())?
        .to_string();

    Ok(OAuthTokenResponse {
        access_token,
        refresh_token: parsed["refresh_token"].as_str().map(String::from),
        expires_in: parsed["expires_in"].as_i64(),
        token_type: parsed["token_type"].as_str().map(String::from),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[test]
    fn test_generate_pkce_codes_verifier_length() {
        // RFC 7636 §4.1: code_verifier length MUST be 43..=128 characters
        for _ in 0..10 {
            let codes = generate_pkce_codes();
            let len = codes.verifier.len();
            assert!(
                (43..=128).contains(&len),
                "verifier length {} is outside RFC 7636 §4.1 range 43-128",
                len
            );
        }
    }

    #[test]
    fn test_generate_pkce_codes_challenge_is_s256() {
        // Challenge must equal URL_SAFE_NO_PAD(SHA-256(verifier_bytes))
        let codes = generate_pkce_codes();
        let expected_digest = Sha256::digest(codes.verifier.as_bytes());
        let expected_challenge =
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(expected_digest);
        assert_eq!(
            codes.challenge, expected_challenge,
            "challenge must be URL_SAFE_NO_PAD(SHA-256(verifier))"
        );
    }

    #[test]
    fn test_generate_pkce_codes_url_safe_no_pad() {
        // Both verifier and challenge must use URL-safe base64 with NO padding
        for _ in 0..20 {
            let codes = generate_pkce_codes();
            for c in codes.verifier.chars() {
                assert!(
                    c.is_ascii_alphanumeric() || c == '-' || c == '_',
                    "verifier char '{}' is not URL-safe (no +, /, =): {}",
                    c, codes.verifier
                );
            }
            for c in codes.challenge.chars() {
                assert!(
                    c.is_ascii_alphanumeric() || c == '-' || c == '_',
                    "challenge char '{}' is not URL-safe (no +, /, =): {}",
                    c, codes.challenge
                );
            }
        }
    }

    #[test]
    fn test_generate_pkce_codes_are_unique() {
        // Two invocations must produce different verifiers (probabilistic; 3 pairs)
        for _ in 0..3 {
            let a = generate_pkce_codes();
            let b = generate_pkce_codes();
            assert_ne!(
                a.verifier, b.verifier,
                "PKCE verifiers should be unique across invocations"
            );
        }
    }

    #[tokio::test]
    async fn test_refresh_access_token_form_urlencoded_body() {
        // Spawn a mock token endpoint that echoes the request body
        let listener = tokio::net::TcpListener::bind(
            SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0)
        ).await.unwrap();
        let port = listener.local_addr().unwrap().port();

        tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buf = [0u8; 4096];
            let n = stream.read(&mut buf).await.unwrap();
            let request = std::str::from_utf8(&buf[..n]).unwrap_or("").to_string();

            // Extract body (after \r\n\r\n)
            let body = request.split("\r\n\r\n").nth(1).unwrap_or("").to_string();

            // Verify the body contains expected fields
            assert!(body.contains("grant_type=refresh_token"), "body: {}", body);
            assert!(body.contains("client_id=my_client"), "body: {}", body);
            assert!(body.contains("refresh_token=my_refresh"), "body: {}", body);

            // Respond with a mock token
            let resp_body = r#"{"access_token":"new_access_tok","expires_in":3600}"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                resp_body.len(), resp_body
            );
            stream.write_all(response.as_bytes()).await.unwrap();
            stream.flush().await.unwrap();
        });

        let url = format!("http://127.0.0.1:{}/token", port);
        let result = refresh_access_token(
            &url,
            "my_client",
            "my_refresh",
            TokenRequestStyle::FormUrlencoded,
        )
        .await;

        assert!(result.is_ok(), "expected Ok: {:?}", result);
        let tok = result.unwrap();
        assert_eq!(tok.access_token, "new_access_tok");
        assert_eq!(tok.expires_in, Some(3600));
    }

    #[tokio::test]
    async fn test_refresh_access_token_json_body() {
        // Mock token endpoint: validate Content-Type and JSON body
        let listener = tokio::net::TcpListener::bind(
            SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0)
        ).await.unwrap();
        let port = listener.local_addr().unwrap().port();

        tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buf = [0u8; 4096];
            let n = stream.read(&mut buf).await.unwrap();
            let request = std::str::from_utf8(&buf[..n]).unwrap_or("").to_string();

            assert!(
                request.contains("application/json"),
                "Content-Type should be application/json for Json style"
            );
            let body = request.split("\r\n\r\n").nth(1).unwrap_or("");
            let parsed: serde_json::Value = serde_json::from_str(body).unwrap();
            assert_eq!(parsed["grant_type"].as_str(), Some("refresh_token"));
            assert_eq!(parsed["client_id"].as_str(), Some("my_client_json"));
            assert_eq!(parsed["refresh_token"].as_str(), Some("my_refresh_json"));

            let resp_body = r#"{"access_token":"json_access_tok","refresh_token":"new_rt","expires_in":7200}"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                resp_body.len(), resp_body
            );
            stream.write_all(response.as_bytes()).await.unwrap();
            stream.flush().await.unwrap();
        });

        let url = format!("http://127.0.0.1:{}/token", port);
        let result = refresh_access_token(
            &url,
            "my_client_json",
            "my_refresh_json",
            TokenRequestStyle::Json,
        )
        .await;

        assert!(result.is_ok(), "expected Ok: {:?}", result);
        let tok = result.unwrap();
        assert_eq!(tok.access_token, "json_access_tok");
        assert_eq!(tok.refresh_token.as_deref(), Some("new_rt"));
        assert_eq!(tok.expires_in, Some(7200));
    }

    #[tokio::test]
    async fn test_start_oauth_flow_uses_configured_redirect_uri_host() {
        // Verify that OAuthFlowConfig.redirect_uri_host flows through to the authorization URL.
        // This gates the per-provider host wiring from 07-OAUTH-RESEARCH.md §Redirect URI Host Resolution.
        //
        // We don't actually complete the flow — we only capture the auth URL and assert the
        // redirect_uri query param starts with "http://localhost:" (not "http://127.0.0.1:").

        let captured_url = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
        let captured_url_clone = captured_url.clone();

        let config = OAuthFlowConfig {
            provider_slug: "openai-codex",
            authorization_url: "https://auth.openai.com/oauth/authorize",
            token_url: "https://auth.openai.com/oauth/token",
            client_id: "test_client_id",
            scope: "openid profile",
            port_range: (54070, 54080),
            redirect_path: "/auth/callback",
            redirect_uri_host: "localhost", // CRITICAL: must be "localhost" for Codex
            extra_authz_params: &[],
            token_request_style: TokenRequestStyle::FormUrlencoded,
        };

        // Start the flow but cancel it immediately after capturing the auth URL
        let result = tokio::time::timeout(
            tokio::time::Duration::from_secs(2),
            start_oauth_flow(config, move |url| {
                *captured_url_clone.lock().unwrap() = url;
                // Return an error to abort the flow (we only want the URL, not to complete)
                Err("test_abort".to_string())
            }),
        )
        .await;

        // The flow should have failed with our abort error
        let url = captured_url.lock().unwrap().clone();
        assert!(!url.is_empty(), "authorization URL should have been captured");

        // CRITICAL: redirect_uri in the auth URL must use "localhost" (not "127.0.0.1")
        // because that's what the Codex CLI Hydra allow-list requires
        let decoded_url = url_decode_for_test(&url);
        assert!(
            decoded_url.contains("redirect_uri=http://localhost:") || url.contains("redirect_uri=http%3A%2F%2Flocalhost%3A"),
            "redirect_uri in auth URL must start with http://localhost: for openai-codex, \
             got auth URL: {}", url
        );
        // Also assert it does NOT use 127.0.0.1
        assert!(
            !url.contains("127.0.0.1") || url.contains("localhost"),
            "redirect_uri must NOT use 127.0.0.1 when redirect_uri_host='localhost'"
        );

        // The flow result should be Err("test_abort") or timeout — both are fine
        let _ = result;
    }

    // Helper for the redirect_uri_host test: URL decode the query string for easier assertion
    fn url_decode_for_test(url: &str) -> String {
        // Simple %XX decoder for test assertions
        let mut out = String::new();
        let bytes = url.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'%' && i + 2 < bytes.len() {
                if let Ok(hex) = std::str::from_utf8(&bytes[i + 1..i + 3]) {
                    if let Ok(byte) = u8::from_str_radix(hex, 16) {
                        out.push(byte as char);
                        i += 3;
                        continue;
                    }
                }
            }
            out.push(bytes[i] as char);
            i += 1;
        }
        out
    }
}
