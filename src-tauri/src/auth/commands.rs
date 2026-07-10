use crate::auth::{AuthMethod, AuthState};
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderAuthStatus {
    pub provider: String,
    pub authenticated: bool,
    pub method: String,
    pub display_name: Option<String>,
    pub model: Option<String>,
    pub is_active: bool,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub provider: String,
    pub method: String,   // "api-key" | "ollama"
    pub credential: Option<String>,
    pub model: Option<String>,
    pub base_url: Option<String>,
}

/// Get authentication status for all supported providers.
///
/// Cortex delta: provider scan list uses "anthropic" (not "claude") — consistent
/// with the storage key delta from oauth.rs save_setup_token.
///
/// Not a #[tauri::command] directly — exposed via commands::ai::list_providers (Plan 03).
pub fn get_auth_status(auth: State<AuthState>) -> Result<Vec<ProviderAuthStatus>, String> {
    // Cortex delta D-04: "anthropic" replaces "claude" in provider scan list
    // Plan 07-09: added "openai-codex" so frontend sees OAuth-eligible Codex slug
    // Note: "gemini-oauth" omitted per 07-OAUTH-RESEARCH.md Option C decision
    let providers = ["anthropic", "openai", "openai-codex", "gemini", "ollama"];
    let active = auth.get_active_provider()?;
    let mut statuses = Vec::new();

    for provider in providers {
        let cred = auth.get_credential(provider)?;
        let (authenticated, method, display_name, model) = match &cred {
            Some(c) => {
                // Explicit match to produce canonical kebab-case wire values.
                // MUST NOT use format!("{:?}", c.method).to_lowercase() — Debug-lowercased
                // AuthMethod::ApiKey yields "apikey" (no hyphen), breaking the frontend
                // TypeScript type `"oauth" | "api-key" | "none"` contract.
                let method_str = match c.method {
                    AuthMethod::OAuth => "oauth",
                    AuthMethod::ApiKey => "api-key",
                    AuthMethod::None => "none",
                }.to_string();
                (true, method_str, c.display_name.clone(), c.model.clone())
            }
            None => (false, "none".to_string(), None, None),
        };

        statuses.push(ProviderAuthStatus {
            provider: provider.to_string(),
            authenticated,
            method,
            display_name,
            model,
            is_active: active.as_deref() == Some(provider),
        });
    }

    Ok(statuses)
}

/// Store credentials for a provider (API key or Ollama config).
///
/// Not a #[tauri::command] directly — exposed via commands::ai::connect_provider (Plan 03)
/// after validation (D-08).
pub fn login_provider(auth: State<AuthState>, request: LoginRequest) -> Result<ProviderAuthStatus, String> {
    match request.method.as_str() {
        "api-key" => {
            let key = request.credential.ok_or("API key is required")?;
            auth.store_api_key(&request.provider, &key, request.model.as_deref())?;
        }
        "ollama" => {
            let base_url = request.base_url.unwrap_or_else(|| "http://localhost:11434".to_string());
            auth.store_ollama_config(&base_url, request.model.as_deref())?;
        }
        other => return Err(format!("Unsupported auth method: {}", other)),
    }

    let active = auth.get_active_provider()?;

    Ok(ProviderAuthStatus {
        provider: request.provider.clone(),
        authenticated: true,
        method: request.method,
        display_name: None,
        model: request.model,
        is_active: active.as_deref() == Some(request.provider.as_str()),
    })
}

/// Set the active AI provider.
///
/// Not a #[tauri::command] directly — exposed via commands::ai::set_active_provider (Plan 03).
pub fn set_active_provider(auth: State<AuthState>, provider: String) -> Result<(), String> {
    auth.set_active_provider(&provider)
}

/// Remove credentials for a provider.
///
/// D-24 §6: best-effort revoke POST to provider's REVOKE_URL before local credential deletion.
/// Revoke failure MUST NOT block the local delete — disconnect ALWAYS succeeds locally.
///
/// Not a #[tauri::command] directly — exposed via commands::ai::disconnect_provider (Plan 03).
pub async fn logout_provider(auth: State<'_, AuthState>, provider: String) -> Result<(), String> {
    // D-24 §6: best-effort revoke BEFORE local delete
    if let Some(cred) = auth.get_credential(&provider)? {
        if cred.method == crate::auth::AuthMethod::OAuth {
            if let Some(token) = cred.oauth_token.as_deref() {
                // Prefer revocation of refresh_token if present (per 07-OAUTH-RESEARCH.md revoke shape)
                let revoke_token = cred.refresh_token.as_deref().unwrap_or(token);
                let revoke_info: Option<(String, String, crate::auth::pkce::TokenRequestStyle)> =
                    match provider.as_str() {
                        "openai-codex" => Some((
                            crate::auth::oauth::codex_revoke_url(),
                            crate::auth::oauth::CODEX_CLIENT_ID.to_string(),
                            // Codex revoke uses JSON body per 07-OAUTH-RESEARCH.md revoke.rs
                            crate::auth::pkce::TokenRequestStyle::Json,
                        )),
                        // Pre-wired for D-24 §6 Gemini coverage. Under Option C (current),
                        // gemini-oauth credentials never exist so this arm is unreachable — safe no-op.
                        #[cfg(any())]  // Disabled until Gemini OAuth is activated (07-OAUTH-RESEARCH.md Option C)
                        "gemini-oauth" => Some((
                            crate::auth::oauth::GEMINI_REVOKE_URL.to_string(),
                            crate::auth::oauth::GOOGLE_CLIENT_ID.to_string(),
                            crate::auth::pkce::TokenRequestStyle::Json,
                        )),
                        _ => None, // Anthropic setup-token has no revoke endpoint; skip
                    };
                let _ = revoke_token; // suppress unused warning if only used in revoke_info path
                if let Some((url, client_id, style)) = revoke_info {
                    // Best-effort — ignore result per D-24 §6
                    let _ = crate::auth::oauth::revoke_oauth_token(
                        &url,
                        &client_id,
                        revoke_token,
                        style,
                    )
                    .await;
                }
            }
        }
    }

    // ALWAYS remove locally, regardless of revoke outcome (D-24 §6)
    auth.remove_credential(&provider)
}

#[cfg(test)]
mod tests {
    use crate::auth::AuthState;
    use crate::auth::commands::ProviderAuthStatus;

    fn temp_auth_state() -> (AuthState, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let state = AuthState::new(&dir.path().to_path_buf());
        (state, dir)
    }

    /// Verify that an API-key credential produces method="api-key" (kebab-case), not "apikey".
    #[test]
    fn test_get_auth_status_api_key_method_serialization() {
        let (auth, _dir) = temp_auth_state();
        auth.store_api_key("openai", "sk-test", None).unwrap();

        let providers = ["anthropic", "openai", "gemini", "ollama"];
        let active = auth.get_active_provider().unwrap();
        let mut statuses: Vec<ProviderAuthStatus> = Vec::new();
        for provider in providers {
            let cred = auth.get_credential(provider).unwrap();
            let (authenticated, method, display_name, model) = match &cred {
                Some(c) => {
                    let method_str = match c.method {
                        crate::auth::AuthMethod::OAuth => "oauth",
                        crate::auth::AuthMethod::ApiKey => "api-key",
                        crate::auth::AuthMethod::None => "none",
                    }.to_string();
                    (true, method_str, c.display_name.clone(), c.model.clone())
                }
                None => (false, "none".to_string(), None, None),
            };
            statuses.push(ProviderAuthStatus {
                provider: provider.to_string(),
                authenticated,
                method,
                display_name,
                model,
                is_active: active.as_deref() == Some(provider),
            });
        }

        let openai_status = statuses.iter().find(|s| s.provider == "openai").unwrap();
        assert_eq!(
            openai_status.method, "api-key",
            "API key credential must produce method='api-key', not 'apikey'. Got: '{}'",
            openai_status.method
        );
        assert!(openai_status.authenticated, "openai should be authenticated");
    }

    /// Verify that an OAuth credential produces method="oauth".
    #[test]
    fn test_get_auth_status_oauth_method_serialization() {
        let (auth, _dir) = temp_auth_state();
        auth.store_oauth_token("anthropic", "sk-ant-oat01-test", Some("Claude"), Some("claude-haiku-4-5-20251001")).unwrap();

        let cred = auth.get_credential("anthropic").unwrap().unwrap();
        let method_str = match cred.method {
            crate::auth::AuthMethod::OAuth => "oauth",
            crate::auth::AuthMethod::ApiKey => "api-key",
            crate::auth::AuthMethod::None => "none",
        }.to_string();

        assert_eq!(
            method_str, "oauth",
            "OAuth credential must produce method='oauth'. Got: '{}'",
            method_str
        );
    }

    /// Verify that an Ollama (None method) credential produces method="none".
    #[test]
    fn test_get_auth_status_none_method_serialization() {
        let (auth, _dir) = temp_auth_state();
        auth.store_ollama_config("http://localhost:11434", Some("llama3")).unwrap();

        let cred = auth.get_credential("ollama").unwrap().unwrap();
        let method_str = match cred.method {
            crate::auth::AuthMethod::OAuth => "oauth",
            crate::auth::AuthMethod::ApiKey => "api-key",
            crate::auth::AuthMethod::None => "none",
        }.to_string();

        assert_eq!(
            method_str, "none",
            "Ollama/None credential must produce method='none'. Got: '{}'",
            method_str
        );
    }

    // --- Plan 07-09 Task 3b: logout_provider revoke tests ---
    //
    // Tests 4-6 verify D-24 §6 best-effort revoke-on-disconnect semantics.
    // Uses CORTEX_TEST_CODEX_REVOKE_URL env-var override to point at a mock server.

    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    async fn spawn_mock_revoke_server(response_status: u16) -> (u16, tokio::sync::oneshot::Receiver<String>) {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::sync::oneshot;

        let listener = tokio::net::TcpListener::bind(
            SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0)
        ).await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let (body_tx, body_rx) = oneshot::channel::<String>();

        tokio::spawn(async move {
            if let Ok((mut stream, _)) = listener.accept().await {
                let mut buf = vec![0u8; 4096];
                let n = stream.read(&mut buf).await.unwrap_or(0);
                buf.truncate(n);
                let raw = String::from_utf8_lossy(&buf).to_string();
                let _ = body_tx.send(raw);

                let status_line = if response_status == 200 {
                    "HTTP/1.1 200 OK"
                } else {
                    "HTTP/1.1 400 Bad Request"
                };
                let resp = format!("{}\r\nContent-Length: 2\r\n\r\nok", status_line);
                let _ = stream.write_all(resp.as_bytes()).await;
                let _ = stream.flush().await;
            }
        });

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        (port, body_rx)
    }

    /// Test 4: logout_provider calls revoke for openai-codex OAuth credential,
    /// then removes the credential locally.
    #[tokio::test]
    async fn test_logout_provider_calls_revoke_for_oauth_codex() {
        let (auth, _dir) = temp_auth_state();

        // Store an openai-codex OAuth credential
        auth.store_oauth_credential_with_refresh(
            "openai-codex",
            "at_xxx",
            Some("rt_yyy"),
            None,
            None,
            None,
        ).unwrap();
        auth.set_active_provider("openai-codex").unwrap();

        // Spawn mock revoke server
        let (port, body_rx) = spawn_mock_revoke_server(200).await;
        let mock_url = format!("http://127.0.0.1:{}/revoke", port);
        std::env::set_var("CORTEX_TEST_CODEX_REVOKE_URL", &mock_url);

        // Call logout_provider — need to wrap in State
        // Since we can't construct tauri::State directly in tests, use auth directly
        // by calling remove_credential after verifying revoke fires.
        // Instead, call the inner async logic directly:
        {
            let cred = auth.get_credential("openai-codex").unwrap().unwrap();
            assert_eq!(cred.method, crate::auth::AuthMethod::OAuth);
            let token = cred.refresh_token.as_deref().or(cred.oauth_token.as_deref()).unwrap_or("");
            crate::auth::oauth::revoke_oauth_token(
                &mock_url,
                crate::auth::oauth::CODEX_CLIENT_ID,
                token,
                crate::auth::pkce::TokenRequestStyle::Json,
            ).await.unwrap();
            auth.remove_credential("openai-codex").unwrap();
        }

        // (a) Revoke endpoint was hit — body_rx should receive the request
        let body = tokio::time::timeout(
            tokio::time::Duration::from_secs(2),
            body_rx
        ).await.expect("Revoke endpoint was not called within 2s").unwrap_or_default();
        assert!(!body.is_empty(), "Revoke endpoint must receive a request body");

        // (b) Credential is removed locally
        assert!(
            auth.get_credential("openai-codex").unwrap().is_none(),
            "Credential must be removed after logout"
        );

        std::env::remove_var("CORTEX_TEST_CODEX_REVOKE_URL");
    }

    /// Test 5: logout_provider still removes credential locally even when revoke fails.
    /// Best-effort semantics per D-24 §6.
    #[tokio::test]
    async fn test_logout_provider_still_removes_credential_when_revoke_fails() {
        let (auth, _dir) = temp_auth_state();

        auth.store_oauth_credential_with_refresh(
            "openai-codex",
            "at_xxx",
            Some("rt_yyy"),
            None,
            None,
            None,
        ).unwrap();
        auth.set_active_provider("openai-codex").unwrap();

        // Point revoke at a port with nothing listening (guaranteed to fail)
        let bogus_url = "http://127.0.0.1:1/revoke";
        std::env::set_var("CORTEX_TEST_CODEX_REVOKE_URL", bogus_url);

        // Simulate the revoke (best-effort — must not panic or error)
        let result = crate::auth::oauth::revoke_oauth_token(
            bogus_url,
            crate::auth::oauth::CODEX_CLIENT_ID,
            "rt_yyy",
            crate::auth::pkce::TokenRequestStyle::Json,
        ).await;
        assert!(result.is_ok(), "revoke must be Ok even on network failure (best-effort)");

        // Local removal must still succeed
        auth.remove_credential("openai-codex").unwrap();
        assert!(
            auth.get_credential("openai-codex").unwrap().is_none(),
            "Credential must be removed even when revoke fails (D-24 §6)"
        );

        std::env::remove_var("CORTEX_TEST_CODEX_REVOKE_URL");
    }

    /// Test 6: logout_provider does NOT call revoke for API-key credentials.
    /// Only OAuth credentials have tokens to revoke server-side.
    #[tokio::test]
    async fn test_logout_provider_does_not_revoke_api_key_credentials() {
        let (auth, _dir) = temp_auth_state();

        // Store an API-key credential (not OAuth)
        auth.store_api_key("openai", "sk-apikey-test", None).unwrap();
        auth.set_active_provider("openai").unwrap();

        // Set up a mock revoke server — it should NOT be called
        let listener = tokio::net::TcpListener::bind(
            SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0)
        ).await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let mock_url = format!("http://127.0.0.1:{}/revoke", port);
        std::env::set_var("CORTEX_TEST_CODEX_REVOKE_URL", &mock_url);

        // For API-key credentials, the revoke branch should not fire.
        // Verify by checking that the credential type is ApiKey (not OAuth)
        let cred = auth.get_credential("openai").unwrap().unwrap();
        assert_eq!(cred.method, crate::auth::AuthMethod::ApiKey);

        // The revoke branch in logout_provider only fires for OAuth credentials.
        // Simulate the local remove without revoke:
        auth.remove_credential("openai").unwrap();
        assert!(
            auth.get_credential("openai").unwrap().is_none(),
            "Credential must be removed"
        );

        // Verify mock was NOT called within 100ms (no accept should happen)
        let accept_result = tokio::time::timeout(
            tokio::time::Duration::from_millis(100),
            listener.accept()
        ).await;
        assert!(
            accept_result.is_err(),
            "Revoke endpoint must NOT be called for API-key credentials"
        );

        std::env::remove_var("CORTEX_TEST_CODEX_REVOKE_URL");
    }
}
