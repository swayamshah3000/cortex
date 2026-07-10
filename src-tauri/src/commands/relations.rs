//! Phase 11.5 relation IPC surface. 7 commands: 5 read-only queries + 2 mutating
//! overrides, operating on the `TripleStore` sidecar (Plan 02) wired into
//! `AppState` (Plan 04).
//!
//! Read commands acquire `state.triple_store` + `state.entity_store` inside
//! `spawn_blocking` (mirrors `entities.rs` pattern). Mutating commands
//! (`add_manual_triple`, `delete_triple`) persist to disk immediately via
//! `TripleStore::save` — unlike backfill's `upsert_from_doc`, which batches
//! saves at the end of the run.

use std::collections::HashMap;

use tauri::State;

use crate::error::AppError;
use crate::graph::entity_store::EntityStore;
use crate::graph::triple_store::TripleStore;
use crate::state::AppState;
use crate::types::{
    is_valid_predicate, AssetType, CanonicalEntity, OwnershipPageData, PredicateObjectPair,
    PredicateSubjectPair, RelationsPageData, Triple, TripleWithEntities,
};

// ─── Private helpers ────────────────────────────────────────────────────────

/// Resolve a `Triple`'s subject + object `CanonicalEntity` records from the
/// `EntityStore` and bundle them into a `TripleWithEntities`. Returns `None`
/// (with a logged warning) if either side is missing — this can happen if a
/// canonical was deleted/merged after the triple was extracted.
fn expand_triple(triple: &Triple, entity_store: &EntityStore) -> Option<TripleWithEntities> {
    let subject = entity_store.get_canonical(&triple.subject_id);
    let object = entity_store.get_canonical(&triple.object_id);

    match (subject, object) {
        (Some(subject), Some(object)) => Some(TripleWithEntities {
            triple: triple.clone(),
            subject,
            object,
        }),
        _ => {
            eprintln!(
                "Warning: expand_triple could not resolve subject/object for triple {} (subject={}, object={}) — skipping",
                triple.id, triple.subject_id, triple.object_id
            );
            None
        }
    }
}

/// Pure classifier — no engine/state dependency. Implements D-14 / D-18 asset
/// classification from `entity_type` + optional `subclass` (Identifier only) +
/// optional doc `topic`. Topic-based rules take priority for Investment
/// (overrides entity_type per plan), then entity_type-specific rules, then a
/// name-hint fallback for Property/Vehicle, defaulting to `Other`.
pub(crate) fn asset_type_from_signal(
    entity_type: &str,
    subclass: Option<&str>,
    topic: Option<&str>,
    canonical_name: Option<&str>,
) -> AssetType {
    let entity_type_lc = entity_type.to_lowercase();
    let topic_lc = topic.map(|t| t.to_lowercase());

    // Investment/insurance topic overrides entity_type entirely.
    if let Some(t) = topic_lc.as_deref() {
        if t == "investment" || t == "insurance" {
            return AssetType::Investment;
        }
    }

    match entity_type_lc.as_str() {
        "identifier" => {
            let sub = subclass.map(|s| s.to_lowercase()).unwrap_or_default();
            if matches!(
                sub.as_str(),
                "bank_account" | "iban" | "policy_number" | "folio_number" | "credit_card"
            ) {
                AssetType::Financial
            } else {
                AssetType::Other
            }
        }
        "location" => {
            match topic_lc.as_deref() {
                Some("vehicle") => AssetType::Vehicle,
                Some("property") | Some("real_estate") => AssetType::Property,
                _ => {
                    // Fallback: inspect canonical_name for hints.
                    let name_lc = canonical_name.map(|n| n.to_lowercase()).unwrap_or_default();
                    if name_lc.contains("plot")
                        || name_lc.contains("land")
                        || name_lc.contains("property")
                    {
                        AssetType::Property
                    } else if name_lc.contains("car")
                        || name_lc.contains("vehicle")
                        || name_lc.contains("bike")
                        || name_lc.contains("scooter")
                    {
                        AssetType::Vehicle
                    } else {
                        AssetType::Other
                    }
                }
            }
        }
        "organization" => {
            if topic_lc.as_deref() == Some("business") {
                AssetType::Business
            } else {
                AssetType::Other
            }
        }
        _ => AssetType::Other,
    }
}

/// Look up a representative doc topic for a triple's first `doc_ids` entry (if
/// any) via the `documents_384` collection metadata. Returns `None` when the
/// triple has no doc provenance or the doc/topic cannot be resolved.
fn lookup_doc_topic(triple: &Triple, engine: &crate::engine::CortexEngine) -> Option<String> {
    let doc_id = triple.doc_ids.first()?;
    let collection_arc = engine.collections.get_collection("documents_384")?;
    let collection = collection_arc.read();
    let entry = collection.db.get(doc_id).ok().flatten()?;
    let metadata = entry.metadata?;
    metadata
        .get("topic")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

/// Look up a subclass hint for an identifier-class object canonical by
/// scanning its aliases against the stored `ExtractedEntity.subclass` values
/// recorded in any doc's `extracted_entities` metadata. Best-effort: returns
/// the first matching subclass, or `None`.
fn lookup_identifier_subclass(
    object: &CanonicalEntity,
    entity_store: &EntityStore,
    engine: &crate::engine::CortexEngine,
) -> Option<String> {
    let doc_ids = entity_store.doc_index.get(&object.id)?;
    let collection_arc = engine.collections.get_collection("documents_384")?;
    let collection = collection_arc.read();

    for doc_id in doc_ids {
        let Ok(Some(entry)) = collection.db.get(doc_id) else {
            continue;
        };
        let Some(metadata) = entry.metadata else {
            continue;
        };
        let Some(arr) = metadata.get("extracted_entities").and_then(|v| v.as_array()) else {
            continue;
        };
        for e in arr {
            let value = e.get("value").and_then(|v| v.as_str()).unwrap_or("");
            if object.aliases.iter().any(|a| a.eq_ignore_ascii_case(value)) {
                if let Some(sub) = e.get("subclass").and_then(|v| v.as_str()) {
                    return Some(sub.to_string());
                }
            }
        }
    }
    None
}

/// Full classifier — reads doc topic (and, for Identifier objects, a subclass
/// hint) via `engine`, then delegates to the pure `asset_type_from_signal`.
fn classify_asset_type(
    object: &CanonicalEntity,
    triple: &Triple,
    entity_store: &EntityStore,
    engine: &crate::engine::CortexEngine,
) -> AssetType {
    let topic = lookup_doc_topic(triple, engine);
    let subclass = if object.entity_type.eq_ignore_ascii_case("identifier") {
        lookup_identifier_subclass(object, entity_store, engine)
    } else {
        None
    };
    asset_type_from_signal(
        &object.entity_type,
        subclass.as_deref(),
        topic.as_deref(),
        Some(object.canonical_name.as_str()),
    )
}

// ─── IPC commands ───────────────────────────────────────────────────────────

/// `get_entity_relations(entity_id) -> RelationsPageData` (D-13).
/// All triples touching `entity_id`, partitioned into outgoing (entity is
/// subject) and incoming (entity is object), each expanded with resolved
/// subject/object CanonicalEntity records.
#[tauri::command]
pub async fn get_entity_relations(
    entity_id: String,
    state: State<'_, AppState>,
) -> Result<RelationsPageData, AppError> {
    let triple_store = state.triple_store.clone();
    let entity_store = state.entity_store.clone();

    let result = tokio::task::spawn_blocking(move || {
        let entity_store_guard = entity_store
            .lock()
            .map_err(|e| AppError::Internal(e.to_string()))?;

        let entity = entity_store_guard
            .get_canonical(&entity_id)
            .ok_or_else(|| AppError::NotFound(format!("entity not found: {}", entity_id)))?;

        let triple_store_guard = triple_store.blocking_lock();
        let triples = triple_store_guard.get_by_entity(&entity_id);

        let mut outgoing = Vec::new();
        let mut incoming = Vec::new();

        for triple in triples {
            if triple.subject_id == entity_id {
                if let Some(expanded) = expand_triple(triple, &entity_store_guard) {
                    outgoing.push(expanded);
                }
            } else if triple.object_id == entity_id {
                if let Some(expanded) = expand_triple(triple, &entity_store_guard) {
                    incoming.push(expanded);
                }
            }
        }

        Ok::<RelationsPageData, AppError>(RelationsPageData {
            entity,
            outgoing,
            incoming,
        })
    })
    .await??;

    Ok(result)
}

/// `get_all_owned_by(person_id) -> OwnershipPageData` (D-14).
/// Follows the `owns` forward index from `person_id`, groups the resulting
/// assets by `AssetType` derived from each object entity's class + topic.
#[tauri::command]
pub async fn get_all_owned_by(
    person_id: String,
    state: State<'_, AppState>,
) -> Result<OwnershipPageData, AppError> {
    let triple_store = state.triple_store.clone();
    let entity_store = state.entity_store.clone();
    let engine = state.engine.clone();

    let result = tokio::task::spawn_blocking(move || {
        let entity_store_guard = entity_store
            .lock()
            .map_err(|e| AppError::Internal(e.to_string()))?;

        let person = entity_store_guard
            .get_canonical(&person_id)
            .ok_or_else(|| AppError::NotFound(format!("entity not found: {}", person_id)))?;

        let triple_store_guard = triple_store.blocking_lock();
        let object_ids = triple_store_guard.get_objects_for(&person_id, "owns");

        let engine_guard = engine.blocking_lock();

        let mut assets_by_type: HashMap<AssetType, Vec<TripleWithEntities>> = HashMap::new();
        let mut total_assets: u32 = 0;

        for object_id in object_ids {
            // Find the underlying (person_id, owns, object_id) triple via entity_touches.
            let matching_triple = triple_store_guard
                .get_by_entity(&person_id)
                .into_iter()
                .find(|t| {
                    t.subject_id == person_id && t.predicate == "owns" && t.object_id == object_id
                });

            let Some(triple) = matching_triple else {
                continue;
            };

            let Some(object) = entity_store_guard.get_canonical(object_id) else {
                continue;
            };

            let asset_type =
                classify_asset_type(&object, triple, &entity_store_guard, &engine_guard);

            if let Some(expanded) = expand_triple(triple, &entity_store_guard) {
                assets_by_type.entry(asset_type).or_default().push(expanded);
                total_assets += 1;
            }
        }

        Ok::<OwnershipPageData, AppError>(OwnershipPageData {
            person,
            assets_by_type,
            total_assets,
        })
    })
    .await??;

    Ok(result)
}

/// `get_all_related_to(entity_id) -> Vec<(String, CanonicalEntity)>` (D-15).
/// Flat, deduplicated (predicate, other-entity) list for every triple touching
/// `entity_id` in either direction.
#[tauri::command]
pub async fn get_all_related_to(
    entity_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<(String, CanonicalEntity)>, AppError> {
    let triple_store = state.triple_store.clone();
    let entity_store = state.entity_store.clone();

    let result = tokio::task::spawn_blocking(move || {
        let entity_store_guard = entity_store
            .lock()
            .map_err(|e| AppError::Internal(e.to_string()))?;

        // Validate the entity exists.
        entity_store_guard
            .get_canonical(&entity_id)
            .ok_or_else(|| AppError::NotFound(format!("entity not found: {}", entity_id)))?;

        let triple_store_guard = triple_store.blocking_lock();
        let triples = triple_store_guard.get_by_entity(&entity_id);

        let mut seen: std::collections::HashSet<(String, String)> = std::collections::HashSet::new();
        let mut results: Vec<(String, CanonicalEntity)> = Vec::new();

        for triple in triples {
            let (other_id, predicate) = if triple.subject_id == entity_id {
                (triple.object_id.clone(), triple.predicate.clone())
            } else {
                (triple.subject_id.clone(), triple.predicate.clone())
            };

            let key = (predicate.clone(), other_id.clone());
            if seen.contains(&key) {
                continue;
            }

            if let Some(other) = entity_store_guard.get_canonical(&other_id) {
                seen.insert(key);
                results.push((predicate, other));
            }
        }

        Ok::<Vec<(String, CanonicalEntity)>, AppError>(results)
    })
    .await??;

    Ok(result)
}

/// `get_subjects_by_predicate_object(predicate, object_id) -> Vec<PredicateSubjectPair>`.
/// "Who --predicate--> object_id?" — reverse-index lookup.
#[tauri::command]
pub async fn get_subjects_by_predicate_object(
    predicate: String,
    object_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<PredicateSubjectPair>, AppError> {
    if !is_valid_predicate(&predicate) {
        return Err(AppError::Internal(format!(
            "invalid predicate: {}",
            predicate
        )));
    }

    let triple_store = state.triple_store.clone();
    let entity_store = state.entity_store.clone();

    let result = tokio::task::spawn_blocking(move || {
        let entity_store_guard = entity_store
            .lock()
            .map_err(|e| AppError::Internal(e.to_string()))?;

        let triple_store_guard = triple_store.blocking_lock();
        let subject_ids = triple_store_guard.get_subjects_for(&predicate, &object_id);

        let mut results = Vec::new();
        for subject_id in subject_ids {
            let Some(subject) = entity_store_guard.get_canonical(subject_id) else {
                continue;
            };
            let Some(triple) = triple_store_guard.all().find(|t| {
                t.subject_id == subject_id && t.predicate == predicate && t.object_id == object_id
            }) else {
                continue;
            };
            results.push(PredicateSubjectPair {
                subject,
                triple: triple.clone(),
            });
        }

        Ok::<Vec<PredicateSubjectPair>, AppError>(results)
    })
    .await??;

    Ok(result)
}

/// `get_objects_by_subject_predicate(subject_id, predicate) -> Vec<PredicateObjectPair>`.
/// "subject_id --predicate--> who?" — forward-index lookup.
#[tauri::command]
pub async fn get_objects_by_subject_predicate(
    subject_id: String,
    predicate: String,
    state: State<'_, AppState>,
) -> Result<Vec<PredicateObjectPair>, AppError> {
    if !is_valid_predicate(&predicate) {
        return Err(AppError::Internal(format!(
            "invalid predicate: {}",
            predicate
        )));
    }

    let triple_store = state.triple_store.clone();
    let entity_store = state.entity_store.clone();

    let result = tokio::task::spawn_blocking(move || {
        let entity_store_guard = entity_store
            .lock()
            .map_err(|e| AppError::Internal(e.to_string()))?;

        let triple_store_guard = triple_store.blocking_lock();
        let object_ids = triple_store_guard.get_objects_for(&subject_id, &predicate);

        let mut results = Vec::new();
        for object_id in object_ids {
            let Some(object) = entity_store_guard.get_canonical(object_id) else {
                continue;
            };
            let Some(triple) = triple_store_guard.all().find(|t| {
                t.subject_id == subject_id && t.predicate == predicate && t.object_id == object_id
            }) else {
                continue;
            };
            results.push(PredicateObjectPair {
                object,
                triple: triple.clone(),
            });
        }

        Ok::<Vec<PredicateObjectPair>, AppError>(results)
    })
    .await??;

    Ok(result)
}

/// `add_manual_triple(subject_id, predicate, object_id, doc_id?) -> Triple` (D-16).
/// User override — validates subject != object, predicate vocabulary, and that
/// both entities exist, then inserts + persists immediately.
#[tauri::command]
pub async fn add_manual_triple(
    subject_id: String,
    predicate: String,
    object_id: String,
    doc_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<Triple, AppError> {
    if subject_id == object_id {
        return Err(AppError::Internal(
            "subject_id must differ from object_id".to_string(),
        ));
    }
    if !is_valid_predicate(&predicate) {
        return Err(AppError::Internal(format!(
            "invalid predicate: {}",
            predicate
        )));
    }

    let triple_store = state.triple_store.clone();
    let entity_store = state.entity_store.clone();
    let app_data_dir = state.app_data_dir.clone();

    let result = tokio::task::spawn_blocking(move || {
        {
            let entity_store_guard = entity_store
                .lock()
                .map_err(|e| AppError::Internal(e.to_string()))?;
            if entity_store_guard.get_canonical(&subject_id).is_none() {
                return Err(AppError::NotFound(format!(
                    "subject entity not found: {}",
                    subject_id
                )));
            }
            if entity_store_guard.get_canonical(&object_id).is_none() {
                return Err(AppError::NotFound(format!(
                    "object entity not found: {}",
                    object_id
                )));
            }
        }

        let mut triple_store_guard = triple_store.blocking_lock();
        let new_id = triple_store_guard
            .add_manual(subject_id, predicate, object_id, doc_id)
            .map_err(AppError::Internal)?;

        triple_store_guard
            .save(&app_data_dir)
            .map_err(|e| AppError::Internal(e.to_string()))?;

        triple_store_guard
            .get(&new_id)
            .cloned()
            .ok_or_else(|| AppError::Internal(format!("triple not found after insert: {}", new_id)))
    })
    .await??;

    Ok(result)
}

/// `delete_triple(triple_id) -> ()` (D-16).
/// Removes the triple (and its auto-inverse/symmetric partner, if any) and
/// persists immediately. Returns `NotFound` if the id does not exist.
#[tauri::command]
pub async fn delete_triple(triple_id: String, state: State<'_, AppState>) -> Result<(), AppError> {
    let triple_store = state.triple_store.clone();
    let app_data_dir = state.app_data_dir.clone();

    tokio::task::spawn_blocking(move || {
        let mut triple_store_guard = triple_store.blocking_lock();
        if !triple_store_guard.delete(&triple_id) {
            return Err(AppError::NotFound(format!(
                "triple not found: {}",
                triple_id
            )));
        }
        triple_store_guard
            .save(&app_data_dir)
            .map_err(|e| AppError::Internal(e.to_string()))?;
        Ok(())
    })
    .await??;

    Ok(())
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_property_from_topic() {
        let result = asset_type_from_signal("location", None, Some("property"), Some("Unit 204"));
        assert_eq!(result, AssetType::Property);
    }

    #[test]
    fn test_classify_property_from_topic_real_estate_alias() {
        let result =
            asset_type_from_signal("location", None, Some("real_estate"), Some("Unit 204"));
        assert_eq!(result, AssetType::Property);
    }

    #[test]
    fn test_classify_vehicle_from_topic() {
        let result = asset_type_from_signal("location", None, Some("vehicle"), Some("Innova Crysta"));
        assert_eq!(result, AssetType::Vehicle);
    }

    #[test]
    fn test_classify_business_from_org() {
        let result = asset_type_from_signal("organization", None, Some("business"), Some("Acme Corp"));
        assert_eq!(result, AssetType::Business);
    }

    #[test]
    fn test_classify_investment_overrides_entity_type() {
        // Even a "person" entity_type gets Investment if topic says so.
        let result = asset_type_from_signal("person", None, Some("investment"), Some("Some Fund"));
        assert_eq!(result, AssetType::Investment);

        let result_insurance =
            asset_type_from_signal("organization", None, Some("insurance"), Some("LIC Policy"));
        assert_eq!(result_insurance, AssetType::Investment);
    }

    #[test]
    fn test_classify_other_default() {
        let result = asset_type_from_signal("person", None, None, Some("Alex Doe"));
        assert_eq!(result, AssetType::Other);
    }

    #[test]
    fn test_classify_financial_from_identifier_subclass() {
        let result = asset_type_from_signal("identifier", Some("iban"), None, None);
        assert_eq!(result, AssetType::Financial);

        let result_bank = asset_type_from_signal("identifier", Some("bank_account"), None, None);
        assert_eq!(result_bank, AssetType::Financial);

        let result_other = asset_type_from_signal("identifier", Some("pan"), None, None);
        assert_eq!(result_other, AssetType::Other);
    }

    #[test]
    fn test_classify_property_fallback_from_name_hint() {
        let result = asset_type_from_signal("location", None, None, Some("AlphaComplex Plot 12"));
        assert_eq!(result, AssetType::Property);
    }

    #[test]
    fn test_classify_vehicle_fallback_from_name_hint() {
        let result = asset_type_from_signal("location", None, None, Some("Innova Car"));
        assert_eq!(result, AssetType::Vehicle);
    }
}
