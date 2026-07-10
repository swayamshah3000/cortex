//! RAG orchestration for Chat with Your Docs (Phase 11.7, Plan 05).
//!
//! `ChatEngine::answer` runs the full per-query pipeline:
//! 1. Embed the query via `EmbeddingService`.
//! 2. HNSW top-8 docs from the `documents_384` collection, honoring optional
//!    `SearchFilters` (D-04 filter passthrough).
//! 3. Chunk each candidate doc's excerpt (500 chars / 50-char overlap), embed
//!    each chunk, keep the top-3 per doc by cosine similarity to the query.
//! 4. Rerank the resulting <=24 candidates down to the top-12 (D-01).
//! 5. If the best chunk score is below the 0.35 cosine floor (D-03), return a
//!    canned "not found" answer without calling the LLM.
//! 6. Otherwise build the RAG prompt (D-06/D-07) and stream the LLM response,
//!    forwarding tokens to the Tauri event bus and persisting the assistant
//!    message + citations on completion.
//!
//! The pure helpers below (`chunk_text`, `cosine_sim`, `rerank_chunks`,
//! `build_rag_prompt`, `answer_or_canned`) are unit-tested in isolation —
//! no I/O, no async — so the RAG math can be verified without a live model
//! or network access.

use std::path::PathBuf;
use std::sync::Arc;

use tauri::Emitter;
use tokio::sync::Mutex;

use crate::auth::AuthState;
use crate::chat::session_store::ChatSessionStore;
use crate::engine::CortexEngine;
use crate::graph::entity_store::EntityStore;
use crate::pipeline::embedder::EmbeddingService;
use crate::types::{
    ChatMessage, ChatRole, ChatStreamCompletePayload, ChatStreamErrorPayload,
    ChatStreamTokenPayload, Citation, SearchFilters,
};

/// Cosine-similarity floor below which the engine short-circuits to a canned
/// "not found" answer without calling the LLM (D-03).
const COSINE_FLOOR: f32 = 0.20;

/// The exact system prompt text from D-06 (11.7-CONTEXT.md). Preserve line
/// breaks verbatim — this is sent as-is to every provider.
pub const RAG_SYSTEM_PROMPT: &str = "You are a helpful assistant answering questions about the user's personal documents.\n\nRules:\n- Answer using the numbered document excerpts below. Cite as [1], [2] matching the excerpts.\n- Reasonable inference IS allowed: use context clues (document titles, shared surnames, matching addresses, matching dates of birth, explicit relation words) to draw conclusions the document implies. When you infer rather than quote, say so briefly (\"appears to be...\", \"based on shared surname...\").\n- Extract concrete data (numbers, dates, addresses, amounts, names) directly from the excerpts. Read carefully — sale deeds contain sale prices, tax receipts contain amounts, IDs contain DOBs.\n- If a question spans multiple docs, synthesize across them.\n- Only refuse when the info is genuinely absent from all excerpts. Don't refuse just because it isn't stated in exactly the words the user used.\n- Never cite a source not in the list.";

/// Canned answer returned when no chunk clears the cosine floor (D-03).
const NO_MATCH_ANSWER: &str = "I couldn't find anything relevant in your library.";

/// A single chunk of a document's text, with char offsets so citations can
/// reference back to the exact span (D-02: 500 chars / 50-char overlap).
#[derive(Debug, Clone, PartialEq)]
pub struct Chunk {
    pub text: String,
    pub start: u32,
    pub end: u32,
}

/// Sliding-window chunker over CHARS (not bytes) with overlap.
///
/// Given `text`, produces windows of `size` chars advancing by `size -
/// overlap` chars each step. The final chunk may be shorter than `size`.
/// A text shorter than `size` produces exactly one chunk spanning the whole
/// input.
pub fn chunk_text(text: &str, size: usize, overlap: usize) -> Vec<Chunk> {
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();

    if len == 0 {
        return Vec::new();
    }

    if len <= size {
        return vec![Chunk {
            text: text.to_string(),
            start: 0,
            end: len as u32,
        }];
    }

    let stride = size.saturating_sub(overlap).max(1);
    let mut chunks = Vec::new();
    let mut start = 0usize;

    while start < len {
        let end = (start + size).min(len);
        let slice: String = chars[start..end].iter().collect();
        chunks.push(Chunk {
            text: slice,
            start: start as u32,
            end: end as u32,
        });
        if end == len {
            break;
        }
        start += stride;
    }

    chunks
}

/// Cosine similarity between two vectors. Returns 0.0 on zero norm (avoids
/// division by zero / NaN propagation).
pub fn cosine_sim(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    dot / (norm_a * norm_b)
}

/// Sort `candidates` descending by score and keep the top `top_k`.
pub fn rerank_chunks(
    mut candidates: Vec<(String, Chunk, f32)>,
    top_k: usize,
) -> Vec<(String, Chunk, f32)> {
    candidates.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
    candidates.truncate(top_k);
    candidates
}

/// Build the D-07 user-message shape:
///
/// ```text
/// Documents in your library:
/// [1] {title_1}: {chunk_1_text}
/// [2] {title_2}: {chunk_2_text}
/// ...
///
/// Question: {user_query}
/// ```
///
/// `system` is accepted for symmetry with the RAG pipeline's call site but is
/// NOT embedded in the returned string — it is sent as a separate
/// `system_prompt` field on `AIServiceRequest`. Kept as a parameter so the
/// function signature documents the full prompt-assembly contract.
pub fn build_rag_prompt(_system: &str, numbered_docs: &[(u32, &str, &str)], query: &str) -> String {
    let mut out = String::from("Documents in your library:\n");
    for (index, title, chunk_text) in numbered_docs {
        out.push_str(&format!("[{}] {}: {}\n", index, title, chunk_text));
    }
    out.push('\n');
    out.push_str(&format!("Question: {}", query));
    out
}

/// Returns the canned "not found" answer when `best_score` is below the
/// cosine floor (D-03); `None` when the score clears the floor and the LLM
/// should be called.
pub fn answer_or_canned(best_score: f32) -> Option<&'static str> {
    if best_score < COSINE_FLOOR {
        Some(NO_MATCH_ANSWER)
    } else {
        None
    }
}

/// Facade for the RAG pipeline. Stateless — all dependencies are passed into
/// `answer`.
pub struct ChatEngine;

impl ChatEngine {
    /// Runs the full RAG pipeline for a single query and persists the
    /// assistant's response (and citations, if any) to `chat_store`.
    ///
    /// See module doc comment for the full step-by-step description.
    #[allow(clippy::too_many_arguments)]
    pub async fn answer(
        app: tauri::AppHandle,
        auth: Arc<AuthState>,
        engine: Arc<Mutex<CortexEngine>>,
        embedding_service: Arc<EmbeddingService>,
        entity_store: Arc<std::sync::Mutex<EntityStore>>,
        chat_store: Arc<Mutex<ChatSessionStore>>,
        app_data_dir: PathBuf,
        session_id: String,
        _user_message_id: String,
        assistant_message_id: String,
        query: String,
        filters: Option<SearchFilters>,
    ) -> Result<(), String> {
        // ── Retrieval (spawn_blocking: std::sync::Mutex guards must not cross .await) ──
        let retrieval_query = query.clone();
        let retrieval = tokio::task::spawn_blocking(move || {
            Self::retrieve_and_rerank(
                &retrieval_query,
                filters.as_ref(),
                &engine,
                &embedding_service,
                &entity_store,
            )
        })
        .await
        .map_err(|e| format!("retrieval task panicked: {}", e))??;

        let best_score = retrieval
            .iter()
            .map(|(_, _, _, score)| *score)
            .fold(f32::MIN, f32::max);
        let best_score = if retrieval.is_empty() { 0.0 } else { best_score };

        // ── Below-floor branch (D-03): canned answer, no LLM call ──
        if let Some(canned) = answer_or_canned(best_score) {
            let _ = app.emit(
                "chat-stream-token",
                ChatStreamTokenPayload {
                    session_id: session_id.clone(),
                    message_id: assistant_message_id.clone(),
                    token: canned.to_string(),
                    cumulative_index: 1,
                },
            );
            let _ = app.emit(
                "chat-stream-complete",
                ChatStreamCompletePayload {
                    session_id: session_id.clone(),
                    message_id: assistant_message_id.clone(),
                    citations: vec![],
                    input_tokens: None,
                    output_tokens: None,
                },
            );

            Self::persist_assistant_message(
                &chat_store,
                &app_data_dir,
                &session_id,
                &assistant_message_id,
                canned,
                None,
            )
            .await;

            return Ok(());
        }

        // ── Build citations + numbered docs for the prompt ──
        let mut citations: Vec<Citation> = Vec::new();
        let mut numbered_docs: Vec<(u32, String, String)> = Vec::new();
        for (index, (doc_id, doc_title, chunk, _score)) in retrieval.iter().enumerate() {
            let doc_index = (index + 1) as u32;
            citations.push(Citation {
                index: doc_index,
                doc_id: doc_id.clone(),
                doc_title: doc_title.clone(),
                chunk_start: chunk.start,
                chunk_end: chunk.end,
            });
            numbered_docs.push((doc_index, doc_title.clone(), chunk.text.clone()));
        }

        let numbered_docs_refs: Vec<(u32, &str, &str)> = numbered_docs
            .iter()
            .map(|(i, t, c)| (*i, t.as_str(), c.as_str()))
            .collect();

        let prompt = build_rag_prompt(RAG_SYSTEM_PROMPT, &numbered_docs_refs, &query);

        let request = crate::ai::AIServiceRequest {
            system_prompt: RAG_SYSTEM_PROMPT.to_string(),
            messages: vec![crate::ai::ServiceMessage {
                role: "user".to_string(),
                content: prompt,
            }],
            max_tokens: Some(4096),
            temperature: None,
            response_format: None,
            model_override: None,
        };

        let mut stream = match crate::ai::ai_request_stream(&auth, request).await {
            Ok(s) => s,
            Err(message) => {
                let _ = app.emit(
                    "chat-stream-error",
                    ChatStreamErrorPayload {
                        session_id: session_id.clone(),
                        message_id: assistant_message_id.clone(),
                        error: message.clone(),
                    },
                );
                Self::persist_assistant_message(
                    &chat_store,
                    &app_data_dir,
                    &session_id,
                    &assistant_message_id,
                    &message,
                    None,
                )
                .await;
                return Err(message);
            }
        };

        use futures::StreamExt;
        let mut content_buffer = String::new();

        while let Some(chunk) = stream.next().await {
            match chunk {
                crate::ai::StreamChunk::Token {
                    token,
                    cumulative_index,
                } => {
                    content_buffer.push_str(&token);
                    let _ = app.emit(
                        "chat-stream-token",
                        ChatStreamTokenPayload {
                            session_id: session_id.clone(),
                            message_id: assistant_message_id.clone(),
                            token,
                            cumulative_index,
                        },
                    );
                }
                crate::ai::StreamChunk::Done {
                    input_tokens,
                    output_tokens,
                    ..
                } => {
                    let _ = app.emit(
                        "chat-stream-complete",
                        ChatStreamCompletePayload {
                            session_id: session_id.clone(),
                            message_id: assistant_message_id.clone(),
                            citations: citations.clone(),
                            input_tokens,
                            output_tokens,
                        },
                    );
                    Self::persist_assistant_message(
                        &chat_store,
                        &app_data_dir,
                        &session_id,
                        &assistant_message_id,
                        &content_buffer,
                        Some(citations.clone()),
                    )
                    .await;
                    return Ok(());
                }
                crate::ai::StreamChunk::Error { message } => {
                    let _ = app.emit(
                        "chat-stream-error",
                        ChatStreamErrorPayload {
                            session_id: session_id.clone(),
                            message_id: assistant_message_id.clone(),
                            error: message.clone(),
                        },
                    );
                    // Persist a truncated assistant message with the error so
                    // the user can see what failed after reload.
                    Self::persist_assistant_message(
                        &chat_store,
                        &app_data_dir,
                        &session_id,
                        &assistant_message_id,
                        &message,
                        None,
                    )
                    .await;
                    return Err(message);
                }
            }
        }

        // Stream ended without an explicit Done — treat as a soft success so
        // the user still sees whatever content arrived.
        let _ = app.emit(
            "chat-stream-complete",
            ChatStreamCompletePayload {
                session_id: session_id.clone(),
                message_id: assistant_message_id.clone(),
                citations: citations.clone(),
                input_tokens: None,
                output_tokens: None,
            },
        );
        Self::persist_assistant_message(
            &chat_store,
            &app_data_dir,
            &session_id,
            &assistant_message_id,
            &content_buffer,
            Some(citations),
        )
        .await;

        Ok(())
    }

    /// Blocking retrieval segment: embed query, HNSW top-8, chunk + embed +
    /// score, rerank to top-12. Runs inside `spawn_blocking` so the
    /// std::sync::Mutex guards on `engine`/`entity_store` never cross an
    /// `.await`.
    #[allow(clippy::type_complexity)]
    fn retrieve_and_rerank(
        query: &str,
        filters: Option<&SearchFilters>,
        engine: &Arc<Mutex<CortexEngine>>,
        embedding_service: &Arc<EmbeddingService>,
        entity_store: &Arc<std::sync::Mutex<EntityStore>>,
    ) -> Result<Vec<(String, String, Chunk, f32)>, String> {
        let query_vec = embedding_service
            .embed_text(query)
            .map_err(|e| e.to_string())?;

        let engine_guard = engine.blocking_lock();

        // Optional filter narrowing (D-04), mirroring search_documents_impl's
        // candidate-set combine truth table.
        let mut candidate_set: Option<std::collections::HashSet<String>> = None;
        if let Some(filters) = filters {
            let metadata_candidates =
                crate::search::filters::apply_metadata_filters(filters, &engine_guard)
                    .map_err(|e| e.to_string())?;
            let entity_candidates = {
                let entity_guard = entity_store
                    .lock()
                    .map_err(|e| format!("entity_store lock poisoned: {}", e))?;
                crate::search::filters::apply_entity_class_filters(
                    filters.entity_filters.as_deref().unwrap_or(&[]),
                    &entity_guard,
                )
            };
            candidate_set = match (metadata_candidates, entity_candidates) {
                (None, None) => None,
                (Some(a), None) => Some(a),
                (None, Some(b)) => Some(b),
                (Some(a), Some(b)) => Some(a.intersection(&b).cloned().collect()),
            };
        }

        let collection_arc = engine_guard
            .collections
            .get_collection("documents_384")
            .ok_or_else(|| "documents_384 collection not found".to_string())?;

        let search_query = ruvector_core::types::SearchQuery {
            vector: query_vec.clone(),
            k: 12,
            filter: None,
            ef_search: None,
        };

        let raw_results = {
            let collection = collection_arc.read();
            collection.db.search(search_query).map_err(|e| e.to_string())?
        };

        let mut all_candidates: Vec<(String, String, Chunk, f32)> = Vec::new();

        for raw in raw_results {
            if let Some(ref candidates) = candidate_set {
                if !candidates.contains(&raw.id) {
                    continue;
                }
            }

            let metadata = match raw.metadata {
                Some(ref m) => m,
                None => continue,
            };

            let doc_title = metadata
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown")
                .to_string();

            // Read FULL document text from disk (Fix: 200-char excerpts starve LLM).
            // Falls back to metadata.excerpt if disk parse fails.
            let full_text: String = metadata
                .get("path")
                .and_then(|v| v.as_str())
                .and_then(|p| {
                    crate::pipeline::parser::parse_document(std::path::Path::new(p))
                        .ok()
                        .map(|pd| pd.text)
                })
                .or_else(|| {
                    metadata
                        .get("excerpt")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                })
                .unwrap_or_default();

            if full_text.is_empty() {
                continue;
            }

            // Chunk at 1500 chars / 200 overlap — big enough to carry
            // dollar figures, dates, party names in a single window.
            let doc_chunks = chunk_text(&full_text, 1500, 200);
            let mut scored_chunks: Vec<(String, String, Chunk, f32)> = Vec::new();

            for chunk in doc_chunks {
                let chunk_vec = match embedding_service.embed_text(&chunk.text) {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                let score = cosine_sim(&query_vec, &chunk_vec);
                scored_chunks.push((raw.id.clone(), doc_title.clone(), chunk, score));
            }

            scored_chunks.sort_by(|a, b| b.3.partial_cmp(&a.3).unwrap_or(std::cmp::Ordering::Equal));
            scored_chunks.truncate(5);
            all_candidates.extend(scored_chunks);
        }

        all_candidates.sort_by(|a, b| b.3.partial_cmp(&a.3).unwrap_or(std::cmp::Ordering::Equal));
        all_candidates.truncate(15);
        Ok(all_candidates)
    }

    /// Persist the assistant's `ChatMessage` (with optional citations) to the
    /// session store and flush to disk. Failures are logged, not propagated —
    /// the streaming events have already reached the frontend by this point.
    async fn persist_assistant_message(
        chat_store: &Arc<Mutex<ChatSessionStore>>,
        app_data_dir: &PathBuf,
        session_id: &str,
        assistant_message_id: &str,
        content: &str,
        citations: Option<Vec<Citation>>,
    ) {
        let now = chrono::Utc::now().to_rfc3339();
        let msg = ChatMessage {
            id: assistant_message_id.to_string(),
            role: ChatRole::Assistant,
            content: content.to_string(),
            citations,
            created_at: now.clone(),
        };

        let mut store = chat_store.lock().await;
        store.append_message(session_id, msg, now);
        if let Err(e) = store.save(app_data_dir) {
            eprintln!("Warning: failed to persist chat session {}: {}", session_id, e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Test 1: chunk_text 500/50 overlap ──
    #[test]
    fn test_chunk_text_500_char_50_overlap() {
        let text: String = "a".repeat(1200);
        let chunks = chunk_text(&text, 500, 50);
        assert_eq!(chunks.len(), 3, "1200-char input must produce 3 chunks");

        assert_eq!(chunks[0].start, 0);
        assert_eq!(chunks[0].end, 500);
        assert_eq!(chunks[1].start, 450);
        assert_eq!(chunks[1].end, 950);
        assert_eq!(chunks[2].start, 900);
        assert_eq!(chunks[2].end, 1200);
    }

    // ── Test 2: short input → single chunk ──
    #[test]
    fn test_chunk_text_short_input_single_chunk() {
        let text = "short text under 500 chars";
        let chunks = chunk_text(text, 500, 50);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].start, 0);
        assert_eq!(chunks[0].end, text.chars().count() as u32);
    }

    // ── Test 3: cosine_sim ──
    #[test]
    fn test_cosine_similarity() {
        let identical = vec![1.0, 2.0, 3.0];
        assert!((cosine_sim(&identical, &identical) - 1.0).abs() < 1e-6);

        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        assert!((cosine_sim(&a, &b) - 0.0).abs() < 1e-6);

        // Known small pair: [1,0,0] vs [1,1,0] -> cos = 1/sqrt(2)
        let x = vec![1.0, 0.0, 0.0];
        let y = vec![1.0, 1.0, 0.0];
        let expected = 1.0 / std::f32::consts::SQRT_2;
        assert!((cosine_sim(&x, &y) - expected).abs() < 1e-5);
    }

    // ── Test 4: rerank_chunks top 12 of 24 ──
    #[test]
    fn test_rerank_top_12() {
        let mut candidates = Vec::new();
        for i in 0..24 {
            candidates.push((
                format!("doc-{}", i),
                Chunk {
                    text: format!("chunk-{}", i),
                    start: 0,
                    end: 10,
                },
                i as f32,
            ));
        }

        let top = rerank_chunks(candidates, 12);
        assert_eq!(top.len(), 12);
        // Descending order, highest score (23.0) first.
        for i in 0..12 {
            assert_eq!(top[i].2, (23 - i) as f32);
        }
    }

    // ── Test 5: build_rag_prompt exact shape ──
    #[test]
    fn test_build_rag_prompt() {
        let docs = vec![(1u32, "Doc A title", "chunk text 1"), (2u32, "Doc B title", "chunk text 2")];
        let prompt = build_rag_prompt(RAG_SYSTEM_PROMPT, &docs, "What happened?");
        let expected = "Documents in your library:\n[1] Doc A title: chunk text 1\n[2] Doc B title: chunk text 2\n\nQuestion: What happened?";
        assert_eq!(prompt, expected);
    }

    // ── Test 6: below-floor canned answer ──
    #[test]
    fn test_below_cosine_floor_returns_canned_answer() {
        assert_eq!(
            answer_or_canned(0.05),
            Some("I couldn't find anything relevant in your library.")
        );
        assert_eq!(answer_or_canned(0.19), Some(NO_MATCH_ANSWER));
        assert_eq!(answer_or_canned(0.20), None);
        assert_eq!(answer_or_canned(0.9), None);
    }
}
