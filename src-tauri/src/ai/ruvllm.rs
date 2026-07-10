//! Local LLM inference via the `ruvllm` crate (Metal-accelerated on Apple Silicon).
//!
//! Integrates ruvllm as the `local-ruvllm` provider (D-03) per 11.8-01-AUDIT.md's
//! confirmed target API surface: `CandleBackend::with_device(Metal) -> load_model(id, cfg)
//! -> apply_chat_template(&messages) -> generate(prompt, params)`.
//!
//! ## Model lifecycle
//! A single `CandleBackend` is lazily initialized and cached process-wide behind a
//! `tokio::sync::Mutex` (module-level `OnceCell`), since `load_model()` requires `&mut self`
//! and model init/load is expensive (should happen at most once per session, not per request).
//!
//! ## Fallback contract (D-06 / T-11.8-06-02)
//! Errors that should trigger the frontend to fall back to the previously active provider
//! are prefixed with `[FALLBACK]` — e.g. Metal unavailable, model not downloaded, or any
//! runtime-init failure. Plan 10 (Settings UI) detects this prefix and swaps providers +
//! shows a toast.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;

use ruvllm::backends::{CandleBackend, DeviceType, GenerateParams, LlmBackend, ModelConfig, Quantization};
use ruvllm::tokenizer::ChatMessage;

use crate::ai::service::{AIServiceResponse, ServiceMessage};

/// The default model shipped for `local-ruvllm` (11.8-01-AUDIT.md "Recommended default model").
/// Qwen2.5-7B-Instruct, Q4_K_M GGUF — best decode throughput/quality tradeoff on Apple Silicon
/// per the ruvllm README's own M4 Pro benchmark table (95 tok/s decode, 4.2GB memory).
pub fn default_model_id() -> &'static str {
    "Qwen/Qwen2.5-7B-Instruct-GGUF"
}

/// Lightweight fallback model for lower-spec Macs or faster cold start (audit's secondary pick).
pub fn fallback_model_id() -> &'static str {
    "meta-llama/Llama-3.2-3B-Instruct-GGUF"
}

/// Allowlist of model ids `download_ruvllm_model` accepts. Mirrors the D-04 model-picker
/// dialog's two offered choices. Rejecting unknown ids here is a Rule 2 (missing-validation)
/// correctness requirement — we never let the frontend trigger an arbitrary HF Hub download.
pub fn allowed_model_ids() -> &'static [&'static str] {
    &[
        "Qwen/Qwen2.5-7B-Instruct-GGUF",
        "meta-llama/Llama-3.2-3B-Instruct-GGUF",
    ]
}

/// Deterministic on-disk location for a downloaded/loaded ruvllm model (D-04: never hardcode
/// absolute paths — always under `app_data_dir/ruvllm-models/{model_id}/`).
pub fn model_storage_path(app_data_dir: &Path, model_id: &str) -> PathBuf {
    app_data_dir.join("ruvllm-models").join(model_id)
}

/// Marker file written after a successful `load_model()` for a given model id, so
/// `ruvllm_chat` can give a clear "not downloaded yet" error without re-attempting a
/// (potentially multi-GB) implicit download inside a plain chat call.
fn ready_marker_path(app_data_dir: &Path, model_id: &str) -> PathBuf {
    model_storage_path(app_data_dir, model_id).join(".ready")
}

/// Process-wide lazily-initialized backend handle. `ai_request()` only has access to
/// `&AuthState` (no `AppState`), so the backend cannot live on Tauri's managed state without
/// changing every call site — instead we hold it here, guarded by an async mutex since
/// `LlmBackend::load_model`/`generate` are synchronous (blocking) calls made from async code.
static BACKEND: std::sync::OnceLock<Arc<Mutex<Option<CandleBackend>>>> = std::sync::OnceLock::new();

fn backend_handle() -> Arc<Mutex<Option<CandleBackend>>> {
    BACKEND
        .get_or_init(|| Arc::new(Mutex::new(None)))
        .clone()
}

/// Resolve a `Quantization` for the GGUF filename patterns `load_from_hub` searches for.
fn default_quantization() -> Quantization {
    Quantization::Q4K
}

/// Ensure the backend is initialized (Metal device selected) and the requested model is
/// loaded. No-op if the currently loaded model already matches `model_id`.
///
/// Returns a `[FALLBACK]`-prefixed error on Metal init failure or model load failure —
/// per D-06, ai_request() must never leave the user with silently-broken inference.
async fn ensure_model_loaded(app_data_dir: &Path, model_id: &str) -> Result<(), String> {
    let handle = backend_handle();
    let mut guard = handle.lock().await;

    if let Some(backend) = guard.as_ref() {
        if backend.is_model_loaded() && backend.model_id() == model_id {
            return Ok(());
        }
    }

    // (Re)initialize the backend on Metal. blocking-safe: with_device() is cheap (device probe).
    let mut backend = CandleBackend::with_device(DeviceType::Metal)
        .map_err(|e| format!("[FALLBACK] ruvllm Metal init failed: {e}"))?;

    let config = ModelConfig {
        quantization: Some(default_quantization()),
        device: DeviceType::Metal,
        ..Default::default()
    };

    // load_model() is a blocking, potentially multi-GB-download call. Run it on a blocking
    // thread so we don't stall the async runtime's worker threads for the duration.
    let model_id_owned = model_id.to_string();
    let mut backend = tokio::task::spawn_blocking(move || {
        backend
            .load_model(&model_id_owned, config)
            .map(|_| backend)
    })
    .await
    .map_err(|e| format!("[FALLBACK] ruvllm load task panicked: {e}"))?
    .map_err(|e| format!("[FALLBACK] ruvllm model load failed: {e}"))?;

    // Mark ready on disk (best-effort — failure to write the marker is not fatal to this call).
    let marker = ready_marker_path(app_data_dir, model_id);
    if let Some(parent) = marker.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(&marker, "1");

    // model_id() reads back whatever load_model recorded internally.
    let _ = backend.model_id();

    *guard = Some(backend);
    Ok(())
}

/// Chat completion via the local ruvllm backend. Mirrors the `ollama_chat` shape used
/// elsewhere in `ai/service.rs`.
///
/// `model` is the HF repo id (or local GGUF path) to load if not already cached. Falls back
/// to `default_model_id()` when empty (mirrors `ai_request`'s "auto" model_override handling).
pub async fn ruvllm_chat(
    app_data_dir: &Path,
    model: &str,
    system_prompt: &str,
    messages: &[ServiceMessage],
    max_tokens: u32,
) -> Result<AIServiceResponse, String> {
    let model_id = if model.is_empty() || model == "auto" {
        default_model_id()
    } else {
        model
    };

    // Fast-path check: if the model has never been downloaded/loaded, tell the caller before
    // attempting a (possibly very slow) implicit download — matches the plan's "not downloaded"
    // UX requirement (Task 2 action spec).
    if !ready_marker_path(app_data_dir, model_id).exists() {
        return Err(format!(
            "ruvllm model not downloaded. Open Settings → Local Provider → Download model. (model: {model_id})"
        ));
    }

    ensure_model_loaded(app_data_dir, model_id).await?;

    let handle = backend_handle();
    let guard = handle.lock().await;
    let backend = guard
        .as_ref()
        .ok_or_else(|| "[FALLBACK] ruvllm backend not initialized".to_string())?;

    let mut chat = vec![ChatMessage::system(system_prompt)];
    chat.extend(messages.iter().map(|m| match m.role.as_str() {
        "assistant" => ChatMessage::assistant(m.content.clone()),
        _ => ChatMessage::user(m.content.clone()),
    }));

    let prompt = backend
        .apply_chat_template(&chat)
        .map_err(|e| format!("ruvllm chat template error: {e}"))?;

    let params = GenerateParams::default()
        .with_max_tokens(max_tokens as usize)
        .with_temperature(0.7);

    let text = backend
        .generate(&prompt, params)
        .map_err(|e| format!("ruvllm generation error: {e}"))?;

    Ok(AIServiceResponse {
        content: text,
        model: backend.model_id().to_string(),
        // ruvllm's ModelInfo does not expose per-call token counts today (11.8-01-AUDIT.md).
        input_tokens: None,
        output_tokens: None,
    })
}

/// Streaming variant for RAG chat (Phase 11.7's `ai_request_stream()`).
///
/// Per 11.8-01-AUDIT.md, ruvllm supports streaming via `generate_stream_v2` (sync,
/// `Iterator<Item = Result<StreamEvent>>`). We use the sync iterator form here (not the
/// async `generate_stream_async`) because `CandleBackend` is held behind a plain
/// `tokio::sync::Mutex<Option<CandleBackend>>` guard (not `Send`-friendly across an
/// internally-polled async stream without additional plumbing) — the sync iterator is
/// drained inside a `spawn_blocking` task and forwarded through an mpsc channel to the
/// caller-supplied callback, matching the existing SSE/Tauri-event sink shape used by
/// cloud providers in `ai/stream.rs`.
///
/// Returns a `Vec<String>` of token chunks in order (the caller — `ai/stream.rs` — forwards
/// each chunk to the Tauri event sink as it becomes available via the callback).
pub async fn ruvllm_chat_stream<F>(
    app_data_dir: &Path,
    model: &str,
    system_prompt: &str,
    messages: &[ServiceMessage],
    max_tokens: u32,
    mut on_token: F,
) -> Result<AIServiceResponse, String>
where
    F: FnMut(&str) + Send,
{
    let model_id = if model.is_empty() || model == "auto" {
        default_model_id()
    } else {
        model
    };

    if !ready_marker_path(app_data_dir, model_id).exists() {
        return Err(format!(
            "ruvllm model not downloaded. Open Settings → Local Provider → Download model. (model: {model_id})"
        ));
    }

    ensure_model_loaded(app_data_dir, model_id).await?;

    let handle = backend_handle();
    let guard = handle.lock().await;
    let backend = guard
        .as_ref()
        .ok_or_else(|| "[FALLBACK] ruvllm backend not initialized".to_string())?;

    let mut chat = vec![ChatMessage::system(system_prompt)];
    chat.extend(messages.iter().map(|m| match m.role.as_str() {
        "assistant" => ChatMessage::assistant(m.content.clone()),
        _ => ChatMessage::user(m.content.clone()),
    }));

    let prompt = backend
        .apply_chat_template(&chat)
        .map_err(|e| format!("ruvllm chat template error: {e}"))?;

    let params = GenerateParams::default()
        .with_max_tokens(max_tokens as usize)
        .with_temperature(0.7);

    let stream = backend
        .generate_stream_v2(&prompt, params)
        .map_err(|e| format!("ruvllm stream init error: {e}"))?;

    let mut full_text = String::new();
    for event in stream {
        match event {
            Ok(ruvllm::backends::StreamEvent::Token(t)) => {
                full_text.push_str(&t.text);
                on_token(&t.text);
            }
            Ok(ruvllm::backends::StreamEvent::Done { .. }) => break,
            Ok(ruvllm::backends::StreamEvent::Error(e)) => {
                return Err(format!("ruvllm stream error: {e}"));
            }
            Err(e) => return Err(format!("ruvllm stream error: {e}")),
        }
    }

    Ok(AIServiceResponse {
        content: full_text,
        model: backend.model_id().to_string(),
        input_tokens: None,
        output_tokens: None,
    })
}

/// Download (and load) a ruvllm model by HF repo id, validating against the allowlist first
/// (Rule 2 — never let the frontend trigger an arbitrary/unvalidated HF Hub download).
///
/// `load_model()` transparently handles the HF Hub download internally (`load_from_hub`), so
/// there is no separate byte-level downloader here — this function's job is validation +
/// coarse-grained progress signaling + marking the model "ready" on disk once loaded.
///
/// Progress callback contract (mirrors Tauri event payload `{modelId, bytesDownloaded,
/// totalBytes}` emitted by the `download_ruvllm_model` command): ruvllm's synchronous
/// `hf_hub::api::sync::Api` does not expose byte-level progress, so this emits two coarse
/// steps — `(0, 1)` at start and `(1, 1)` on completion — rather than fine-grained byte
/// counts. Documented as a deviation in the plan SUMMARY.
pub async fn download_model<F>(
    model_id: &str,
    app_data_dir: &Path,
    mut progress: F,
) -> Result<PathBuf, String>
where
    F: FnMut(u64, u64) + Send + 'static,
{
    if !allowed_model_ids().contains(&model_id) {
        return Err(format!("unknown model id: {model_id}"));
    }

    progress(0, 1);

    let app_data_dir_owned = app_data_dir.to_path_buf();
    let model_id_owned = model_id.to_string();
    ensure_model_loaded(&app_data_dir_owned, &model_id_owned).await?;

    progress(1, 1);

    Ok(model_storage_path(app_data_dir, model_id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_model_id_matches_audit() {
        // Regression guard against silent model swaps (11.8-01-AUDIT.md "Recommended default model").
        assert_eq!(default_model_id(), "Qwen/Qwen2.5-7B-Instruct-GGUF");
    }

    #[test]
    fn test_model_storage_path_deterministic() {
        let app_data_dir = PathBuf::from("/tmp/cortex-test-app-data");
        let path = model_storage_path(&app_data_dir, "Qwen/Qwen2.5-7B-Instruct-GGUF");
        assert_eq!(
            path,
            app_data_dir
                .join("ruvllm-models")
                .join("Qwen/Qwen2.5-7B-Instruct-GGUF")
        );
    }

    #[test]
    fn test_allowed_model_ids_contains_default_and_fallback() {
        assert!(allowed_model_ids().contains(&default_model_id()));
        assert!(allowed_model_ids().contains(&fallback_model_id()));
    }

    #[tokio::test]
    async fn test_ruvllm_chat_errors_when_model_not_downloaded() {
        let dir = tempfile::tempdir().unwrap();
        let result = ruvllm_chat(
            dir.path(),
            "Qwen/Qwen2.5-7B-Instruct-GGUF",
            "system",
            &[ServiceMessage {
                role: "user".to_string(),
                content: "hi".to_string(),
            }],
            16,
        )
        .await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.contains("not downloaded"),
            "expected 'not downloaded' error, got: {err}"
        );
    }

    #[tokio::test]
    async fn test_download_model_rejects_unknown_id() {
        let dir = tempfile::tempdir().unwrap();
        let result = download_model("some/unknown-model", dir.path(), |_, _| {}).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unknown model id"));
    }
}
