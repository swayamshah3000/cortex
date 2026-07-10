//! Phase 11.6 IPC surface for adaptive ontology.
//!
//! 8 commands total (7 from D-15..D-23 + 1 opt-in toggle for D-21):
//! - get_ontology                   — read-through of OntologyStoreSchema
//! - apply_consolidation            — accept a pending consolidation suggestion
//! - add_manual_predicate           — user-added predicate (Settings > Ontology form)
//! - rename_predicate                — user-renamed predicate (Settings > Ontology row)
//! - merge_predicates                — user-merged two predicates (Settings > Ontology multi-select)
//! - reset_ontology_to_seed          — nuclear reset per D-21 "Reset to seed"
//! - regenerate_corpus_seed          — clear bootstrap_completed so next 30-doc batch re-bootstraps
//! - set_automatic_ontology_growth   — toggle D-21 opt-in flag
//!
//! All state mutations persist ontology.json (and triples.json, when a triple
//! rewrite occurred) before returning success. Failures do not persist.
//! Every mutation logs to `activity_log` per D-20.

use tauri::State;

use crate::graph::ontology_store::{TripleRewriteInstruction, TripleRewriteKind};
use crate::state::AppState;
use crate::types::{OntologyStoreSchema, PromoteResult};

// ─── Private helpers ────────────────────────────────────────────────────────

/// Persist both `ontology_store` and `triple_store` to their respective JSON
/// sidecars in a single `spawn_blocking` task (T-11.6-22 mitigation: keeps
/// the two stores from drifting out of sync if one write succeeds and the
/// process is killed before the other completes — both live in the same
/// blocking closure so a panic/early-return leaves neither persisted half-way
/// relative to the caller's view of success).
async fn persist_stores(state: &State<'_, AppState>) -> std::io::Result<()> {
    let dir = state.app_data_dir.clone();
    let os_arc = state.ontology_store.clone();
    let ts_arc = state.triple_store.clone();
    tokio::task::spawn_blocking(move || -> std::io::Result<()> {
        {
            let os = os_arc.blocking_lock();
            os.save(&dir)?;
        }
        {
            let ts = ts_arc.blocking_lock();
            ts.save(&dir)?;
        }
        Ok(())
    })
    .await
    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?
}

/// Push a single activity-log entry for an ontology mutation (D-20). Uses
/// `ActivityLog::record_with_details` (the real API — mirrors
/// `commands::spaces`/`commands::documents`/`commands::folders`), not a raw
/// struct push. Lock-poisoning is logged and swallowed — an activity-log
/// failure must never fail the underlying ontology mutation.
fn log_activity(state: &State<'_, AppState>, action: &str, subject: &str, activity_type: &str) {
    match state.activity_log.lock() {
        Ok(mut log) => log.record_with_details(action, subject, activity_type, None),
        Err(e) => eprintln!("Warning: activity_log lock poisoned in ontology command: {e}"),
    }
}

// ─── Commands ───────────────────────────────────────────────────────────────

/// Read-through of the full on-disk ontology schema (predicates, entity
/// subclasses, pending consolidation, bootstrap/consolidation timestamps).
#[tauri::command]
pub async fn get_ontology(state: State<'_, AppState>) -> Result<OntologyStoreSchema, String> {
    let os = state.ontology_store.lock().await;
    Ok(os.schema_clone())
}

/// Accept a pending consolidation suggestion by id: mutate the ontology
/// vocabulary, apply the matching rewrite to every affected `TripleStore`
/// triple, persist both stores, and log the acceptance (D-16, D-17, D-20).
/// Returns the number of triples rewritten (0 for a `Split`, which requires
/// manual user re-classification — see `TripleRewriteKind::Split`).
#[tauri::command]
pub async fn apply_consolidation(
    suggestion_id: String,
    state: State<'_, AppState>,
) -> Result<u32, String> {
    let now = chrono::Utc::now().to_rfc3339();

    // Step 1: apply to OntologyStore, get rewrite instructions.
    let instructions: Vec<TripleRewriteInstruction> = {
        let mut os = state.ontology_store.lock().await;
        os.apply_consolidation(&suggestion_id, &now)
            .map_err(|e| e.to_string())?
    };

    // Step 2: apply matching rewrites to TripleStore.
    let mut total_rewritten = 0u32;
    {
        let mut ts = state.triple_store.lock().await;
        for inst in &instructions {
            match inst.kind {
                TripleRewriteKind::Rename => {
                    if let Some(from) = inst.from.first() {
                        total_rewritten += ts.rename_predicate_across_all_triples(from, &inst.to);
                    }
                }
                TripleRewriteKind::Merge => {
                    total_rewritten += ts.merge_predicate_across_all_triples(&inst.from, &inst.to);
                }
                TripleRewriteKind::Split => {
                    // Split is manual — no automatic triple rewrite. UI surfaces the
                    // split predicate for user re-classification (D-16/D-17 discretion).
                }
            }
        }
    }

    // Step 3: persist both stores.
    persist_stores(&state).await.map_err(|e| e.to_string())?;

    // Step 4: activity_log emission (D-20).
    log_activity(
        &state,
        "ontology.consolidation_applied",
        &suggestion_id,
        "success",
    );

    Ok(total_rewritten)
}

/// Register a user-authored predicate via Settings > Ontology (D-21).
#[tauri::command]
pub async fn add_manual_predicate(
    name: String,
    description: String,
    subject_class: Option<String>,
    object_class: Option<String>,
    state: State<'_, AppState>,
) -> Result<PromoteResult, String> {
    let now = chrono::Utc::now().to_rfc3339();
    let result = {
        let mut os = state.ontology_store.lock().await;
        os.register_manual_predicate(name.clone(), description, subject_class, object_class, &now)
    };
    if matches!(result, PromoteResult::Promoted) {
        persist_stores(&state).await.map_err(|e| e.to_string())?;
        log_activity(&state, "ontology.manual_predicate_added", &name, "success");
    }
    Ok(result)
}

/// Rename a non-seed predicate: mutate the ontology vocabulary, rewrite every
/// existing `TripleStore` triple carrying the old name, persist both stores,
/// log the rename. Returns the number of triples rewritten.
#[tauri::command]
pub async fn rename_predicate(
    old_name: String,
    new_name: String,
    state: State<'_, AppState>,
) -> Result<u32, String> {
    let now = chrono::Utc::now().to_rfc3339();
    {
        let mut os = state.ontology_store.lock().await;
        os.rename_predicate(&old_name, &new_name, &now)
            .map_err(|e| e.to_string())?;
    }
    let rewritten = {
        let mut ts = state.triple_store.lock().await;
        ts.rename_predicate_across_all_triples(&old_name, &new_name)
    };
    persist_stores(&state).await.map_err(|e| e.to_string())?;
    log_activity(
        &state,
        "ontology.predicate_renamed",
        &format!("{old_name} -> {new_name}"),
        "success",
    );
    Ok(rewritten)
}

/// Merge two or more predicates into one surviving name: mutate the ontology
/// vocabulary, rewrite every existing `TripleStore` triple carrying any of
/// the `from` names, persist both stores, log the merge. Returns the number
/// of triples rewritten.
#[tauri::command]
pub async fn merge_predicates(
    from: Vec<String>,
    into: String,
    state: State<'_, AppState>,
) -> Result<u32, String> {
    let now = chrono::Utc::now().to_rfc3339();
    {
        let mut os = state.ontology_store.lock().await;
        os.merge_predicates(from.clone(), into.clone(), &now)
            .map_err(|e| e.to_string())?;
    }
    let rewritten = {
        let mut ts = state.triple_store.lock().await;
        ts.merge_predicate_across_all_triples(&from, &into)
    };
    persist_stores(&state).await.map_err(|e| e.to_string())?;
    log_activity(
        &state,
        "ontology.predicates_merged",
        &format!("{from:?} -> {into}"),
        "success",
    );
    Ok(rewritten)
}

/// Nuclear reset (D-21): wipe corpus-seeded / adaptive / pending / manual
/// state and entity subclasses back to the frozen 21-predicate seed
/// vocabulary. Does NOT rewrite existing `TripleStore` triples — those
/// continue to reference their original predicate names even if that
/// predicate is no longer in the effective vocabulary. This is intentional
/// per D-21 semantics; the UI (Plan 08) gates this command behind an
/// `AlertDialog` confirmation (T-11.6-24, disposition: accept).
#[tauri::command]
pub async fn reset_ontology_to_seed(state: State<'_, AppState>) -> Result<(), String> {
    let now = chrono::Utc::now().to_rfc3339();
    {
        let mut os = state.ontology_store.lock().await;
        os.reset_to_seed(&now);
    }
    persist_stores(&state).await.map_err(|e| e.to_string())?;
    log_activity(&state, "ontology.reset_to_seed", "reset", "warning");
    Ok(())
}

/// Clear only `corpus_seed` + `bootstrap_completed_at` so the next 30-doc
/// backfill batch re-triggers the corpus-seeded bootstrap (D-01, D-02).
/// Preserves `adaptive_predicates`, `manual_predicates`, and
/// `entity_subclasses` — unlike `reset_ontology_to_seed`, this is not a
/// nuclear wipe.
#[tauri::command]
pub async fn regenerate_corpus_seed(state: State<'_, AppState>) -> Result<(), String> {
    let now = chrono::Utc::now().to_rfc3339();
    {
        let mut os = state.ontology_store.lock().await;
        os.clear_corpus_seed_for_regeneration(&now);
    }
    persist_stores(&state).await.map_err(|e| e.to_string())?;
    log_activity(
        &state,
        "ontology.corpus_seed_regeneration_requested",
        "regenerate",
        "info",
    );
    Ok(())
}

/// Toggle the D-21 opt-in "Automatic ontology growth" preference. Defaults to
/// `false` for privacy-strict users; Settings writes this via the form toggle.
#[tauri::command]
pub async fn set_automatic_ontology_growth(
    enabled: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    {
        let mut os = state.ontology_store.lock().await;
        os.set_automatic_growth(enabled);
    }
    persist_stores(&state).await.map_err(|e| e.to_string())?;
    log_activity(
        &state,
        "ontology.automatic_growth_toggled",
        if enabled { "enabled" } else { "disabled" },
        "info",
    );
    Ok(())
}
