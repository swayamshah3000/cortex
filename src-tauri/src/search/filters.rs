use std::collections::HashSet;
use regex::Regex;

use crate::engine::CortexEngine;
use crate::error::AppError;
use crate::graph::entity_store::EntityStore;
use crate::spaces::manager::SpaceManager;
use crate::types::{EntityClassFilter, SearchFilters};

/// Parsed entity filter extracted from natural language query.
#[derive(Debug, Clone)]
pub struct EntityFilter {
    pub min_amount: Option<f64>,
    pub max_amount: Option<f64>,
    pub before_date: Option<String>,
    pub after_date: Option<String>,
}

/// Apply metadata filters to narrow the candidate set before vector search.
///
/// If all filter fields are None, returns None (no filtering — search all).
/// Otherwise, iterates the collection and intersects matching doc IDs.
pub fn apply_metadata_filters(
    filters: &SearchFilters,
    engine: &CortexEngine,
) -> Result<Option<HashSet<String>>, AppError> {
    // If all filters are None, skip filtering
    if filters.doc_type.is_none()
        && filters.space_id.is_none()
        && filters.date_from.is_none()
        && filters.date_to.is_none()
        && filters.tags.is_none()
    {
        return Ok(None);
    }

    let collection_arc = engine
        .collections
        .get_collection("documents_384")
        .ok_or_else(|| AppError::VectorStorage("documents_384 collection not found".to_string()))?;

    let collection = collection_arc.read();
    let all_ids = collection
        .db
        .keys()
        .map_err(|e| AppError::VectorStorage(e.to_string()))?;

    let mut result_set: HashSet<String> = all_ids.into_iter().collect();

    for id in result_set.clone() {
        let entry = collection
            .db
            .get(&id)
            .map_err(|e| AppError::VectorStorage(e.to_string()))?;

        let matches = match entry {
            Some(entry) => {
                let metadata = entry.metadata.as_ref();
                let mut pass = true;

                if let Some(ref doc_type) = filters.doc_type {
                    let stored = metadata
                        .and_then(|m| m.get("doc_type"))
                        .and_then(|v| v.as_str());
                    if stored != Some(doc_type.as_str()) {
                        pass = false;
                    }
                }

                if pass {
                    if let Some(ref space_id) = filters.space_id {
                        let stored = metadata
                            .and_then(|m| m.get("space_ids"))
                            .and_then(|v| v.as_array());
                        let in_space = stored
                            .map(|arr| {
                                arr.iter()
                                    .any(|v| v.as_str() == Some(space_id.as_str()))
                            })
                            .unwrap_or(false);
                        if !in_space {
                            pass = false;
                        }
                    }
                }

                if pass {
                    if let Some(ref date_from) = filters.date_from {
                        let stored = metadata
                            .and_then(|m| m.get("created_at"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        if stored < date_from.as_str() {
                            pass = false;
                        }
                    }
                }

                if pass {
                    if let Some(ref date_to) = filters.date_to {
                        let stored = metadata
                            .and_then(|m| m.get("created_at"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        if stored > date_to.as_str() {
                            pass = false;
                        }
                    }
                }

                if pass {
                    if let Some(ref filter_tags) = filters.tags {
                        let stored = metadata
                            .and_then(|m| m.get("tags"))
                            .and_then(|v| v.as_array());
                        let has_match = stored
                            .map(|arr| {
                                filter_tags.iter().any(|tag| {
                                    arr.iter().any(|v| v.as_str() == Some(tag.as_str()))
                                })
                            })
                            .unwrap_or(false);
                        if !has_match {
                            pass = false;
                        }
                    }
                }

                pass
            }
            None => false,
        };

        if !matches {
            result_set.remove(&id);
        }
    }

    Ok(Some(result_set))
}

/// Parse entity filters from a natural language query string.
///
/// Detects patterns like:
/// - "invoices over $500" -> min_amount = 500.0
/// - "invoices under $1000" -> max_amount = 1000.0
/// - "documents before 2024-01-01" -> before_date = "2024-01-01"
/// - "documents after 2023-06-01" -> after_date = "2023-06-01"
pub fn parse_entity_filter(query: &str) -> Option<EntityFilter> {
    let lower = query.to_lowercase();

    // Amount patterns
    let over_re = Regex::new(r"(?:over|above|more\s+than|greater\s+than|exceeding)\s+\$?([\d,]+(?:\.\d{2})?)").unwrap();
    let under_re = Regex::new(r"(?:under|below|less\s+than|cheaper\s+than)\s+\$?([\d,]+(?:\.\d{2})?)").unwrap();

    // Date patterns
    let before_re = Regex::new(r"before\s+(\d{4}-\d{2}-\d{2})").unwrap();
    let after_re = Regex::new(r"after\s+(\d{4}-\d{2}-\d{2})").unwrap();

    let min_amount = over_re.captures(&lower).and_then(|c| {
        c.get(1)
            .and_then(|m| m.as_str().replace(',', "").parse::<f64>().ok())
    });

    let max_amount = under_re.captures(&lower).and_then(|c| {
        c.get(1)
            .and_then(|m| m.as_str().replace(',', "").parse::<f64>().ok())
    });

    let before_date = before_re
        .captures(&lower)
        .and_then(|c| c.get(1).map(|m| m.as_str().to_string()));

    let after_date = after_re
        .captures(&lower)
        .and_then(|c| c.get(1).map(|m| m.as_str().to_string()));

    if min_amount.is_none() && max_amount.is_none() && before_date.is_none() && after_date.is_none()
    {
        return None;
    }

    Some(EntityFilter {
        min_amount,
        max_amount,
        before_date,
        after_date,
    })
}

/// Apply entity filter against a document's extracted_entities metadata.
///
/// Returns true if the document passes the filter (or if no relevant entities exist).
pub fn apply_entity_filter(
    entity_filter: &EntityFilter,
    metadata: &std::collections::HashMap<String, serde_json::Value>,
) -> bool {
    let entities = match metadata.get("extracted_entities") {
        Some(v) => match v.as_array() {
            Some(arr) => arr,
            None => return true,
        },
        None => return true,
    };

    // Check amount filters
    if entity_filter.min_amount.is_some() || entity_filter.max_amount.is_some() {
        let amounts: Vec<f64> = entities
            .iter()
            .filter(|e| {
                e.get("entity_type")
                    .and_then(|v| v.as_str())
                    == Some("amount")
            })
            .filter_map(|e| {
                e.get("value")
                    .and_then(|v| v.as_str())
                    .and_then(|s| {
                        let cleaned: String = s.chars().filter(|c| c.is_ascii_digit() || *c == '.').collect();
                        cleaned.parse::<f64>().ok()
                    })
            })
            .collect();

        if !amounts.is_empty() {
            if let Some(min) = entity_filter.min_amount {
                if !amounts.iter().any(|a| *a >= min) {
                    return false;
                }
            }
            if let Some(max) = entity_filter.max_amount {
                if !amounts.iter().any(|a| *a <= max) {
                    return false;
                }
            }
        }
    }

    // Check date filters
    if entity_filter.before_date.is_some() || entity_filter.after_date.is_some() {
        let dates: Vec<&str> = entities
            .iter()
            .filter(|e| {
                e.get("entity_type")
                    .and_then(|v| v.as_str())
                    == Some("date")
            })
            .filter_map(|e| e.get("value").and_then(|v| v.as_str()))
            .collect();

        if !dates.is_empty() {
            if let Some(ref before) = entity_filter.before_date {
                if !dates.iter().any(|d| *d < before.as_str()) {
                    return false;
                }
            }
            if let Some(ref after) = entity_filter.after_date {
                if !dates.iter().any(|d| *d > after.as_str()) {
                    return false;
                }
            }
        }
    }

    true
}

/// Expand a parent `space_id` into the set of descendant doc_ids (Plan 11.8-05, D-20).
///
/// Collects `doc_ids` belonging directly to `space_id`, then recurses into
/// `sub_space_ids` and unions their doc_ids too. Used to narrow the HNSW
/// candidate set for both the hyperbolic search path and the flat-HNSW
/// fallback path when a caller filters by parent Space.
///
/// Returns an empty `HashSet` (never panics) when `space_id` is unknown to
/// the `SpaceManager` — mirrors the "unknown entity → empty" convention used
/// by `apply_entity_class_filters`.
pub fn space_descendant_candidates(space_manager: &SpaceManager, space_id: &str) -> HashSet<String> {
    let mut result = HashSet::new();
    collect_descendant_doc_ids(space_manager, space_id, &mut result);
    result
}

/// Recursive helper for `space_descendant_candidates`.
fn collect_descendant_doc_ids(space_manager: &SpaceManager, space_id: &str, acc: &mut HashSet<String>) {
    let Some(space_data) = space_manager.get_space_data(space_id) else {
        return;
    };

    acc.extend(space_data.doc_ids.iter().cloned());

    for sub_space_id in &space_data.space.sub_space_ids {
        collect_descendant_doc_ids(space_manager, sub_space_id, acc);
    }
}

/// Apply entity-class filters: for each `EntityClassFilter`, look up the canonical_id
/// from `entity_store.alias_index`, then intersect the resulting doc sets.
///
/// Semantics:
/// - Empty slice → `None` (no narrowing; identical to the None-case in `apply_metadata_filters`).
/// - Single or multiple filters → AND semantics: a doc must appear in ALL filter doc sets.
/// - Miss on `alias_index` (unknown class/value pair) → `Some(empty)`, short-circuits loop.
///
/// Class key lookup uses the incoming `class` string first, then falls back to
/// `class.to_lowercase()` to bridge Phase 6 (lowercase entity_type) and Phase 8
/// (capitalized class name) data — pitfall #5 in 11-RESEARCH.md.
pub fn apply_entity_class_filters(
    entity_filters: &[EntityClassFilter],
    entity_store: &EntityStore,
) -> Option<HashSet<String>> {
    if entity_filters.is_empty() {
        return None;
    }

    let mut result: Option<HashSet<String>> = None;

    for ef in entity_filters {
        // Try exact class string first, then lowercase fallback (Phase 6/8 bridge).
        let key_exact = (ef.value.to_lowercase(), ef.class.clone());
        let key_lower = (ef.value.to_lowercase(), ef.class.to_lowercase());

        let canonical_id = entity_store
            .alias_index
            .get(&key_exact)
            .or_else(|| entity_store.alias_index.get(&key_lower))
            .cloned();

        let canonical_id = match canonical_id {
            Some(id) => id,
            None => return Some(HashSet::new()), // unknown entity → empty, no panic
        };

        let doc_set: HashSet<String> = entity_store
            .doc_index
            .get(&canonical_id)
            .cloned()
            .unwrap_or_default();

        result = Some(match result {
            None => doc_set,
            Some(existing) => existing.intersection(&doc_set).cloned().collect(),
        });
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: build an EntityStore seeded with known canonicals/aliases/doc sets.
    /// Mirrors the `seed_canonical` helper in entity_store.rs tests.
    fn make_seeded_store() -> EntityStore {
        use crate::types::CanonicalEntity;
        use std::collections::{HashMap, HashSet};

        let mut store = EntityStore::new();

        // cid-p1: "Alex Doe" → person → {d1, d2}
        let cid_p1 = "cid-p1".to_string();
        store.canonicals.insert(
            cid_p1.clone(),
            CanonicalEntity {
                id: cid_p1.clone(),
                canonical_name: "Alex Doe".to_string(),
                entity_type: "person".to_string(),
                aliases: vec!["Alex Doe".to_string()],
                document_count: 2,
            canonical_short_name: None,
            },
        );
        store.alias_index.insert(
            ("alex doe".to_string(), "person".to_string()),
            cid_p1.clone(),
        );
        let mut p1_docs = HashSet::new();
        p1_docs.insert("d1".to_string());
        p1_docs.insert("d2".to_string());
        store.doc_index.insert(cid_p1, p1_docs);

        // cid-loc1: "AlphaComplex" → location → {d2, d3}
        let cid_loc1 = "cid-loc1".to_string();
        store.canonicals.insert(
            cid_loc1.clone(),
            CanonicalEntity {
                id: cid_loc1.clone(),
                canonical_name: "AlphaComplex".to_string(),
                entity_type: "location".to_string(),
                aliases: vec!["AlphaComplex".to_string()],
                document_count: 2,
            canonical_short_name: None,
            },
        );
        store.alias_index.insert(
            ("alphacomplex".to_string(), "location".to_string()),
            cid_loc1.clone(),
        );
        let mut loc1_docs = HashSet::new();
        loc1_docs.insert("d2".to_string());
        loc1_docs.insert("d3".to_string());
        store.doc_index.insert(cid_loc1, loc1_docs);

        // cid-org1: "Acme Corp" → organization → {d4}
        let cid_org1 = "cid-org1".to_string();
        store.canonicals.insert(
            cid_org1.clone(),
            CanonicalEntity {
                id: cid_org1.clone(),
                canonical_name: "Acme Corp".to_string(),
                entity_type: "organization".to_string(),
                aliases: vec!["Acme Corp".to_string()],
                document_count: 1,
            canonical_short_name: None,
            },
        );
        store.alias_index.insert(
            ("acme corp".to_string(), "organization".to_string()),
            cid_org1.clone(),
        );
        let mut org1_docs = HashSet::new();
        org1_docs.insert("d4".to_string());
        store.doc_index.insert(cid_org1, org1_docs);

        store
    }

    // --- apply_entity_class_filters tests (Task 1: six behavior tests) ---

    /// Test A: empty filter list returns None (no narrowing).
    #[test]
    fn test_apply_entity_class_filters_empty_returns_none() {
        let store = make_seeded_store();
        let result = apply_entity_class_filters(&[], &store);
        assert!(result.is_none(), "empty filter list must return None");
    }

    /// Test B: single filter, known entity — returns matching doc set.
    #[test]
    fn test_apply_entity_class_filters_single_known() {
        let store = make_seeded_store();
        let filters = vec![EntityClassFilter {
            class: "Person".to_string(),
            value: "Alex Doe".to_string(),
        }];
        let result = apply_entity_class_filters(&filters, &store)
            .expect("single known filter must return Some");
        let mut expected = std::collections::HashSet::new();
        expected.insert("d1".to_string());
        expected.insert("d2".to_string());
        assert_eq!(result, expected, "should return {{d1, d2}} for Person:Alex Doe");
    }

    /// Test C: two filters AND together — intersection only.
    #[test]
    fn test_apply_entity_class_filters_two_filters_intersect() {
        let store = make_seeded_store();
        let filters = vec![
            EntityClassFilter {
                class: "Person".to_string(),
                value: "Alex Doe".to_string(),
            },
            EntityClassFilter {
                class: "Location".to_string(),
                value: "AlphaComplex".to_string(),
            },
        ];
        let result = apply_entity_class_filters(&filters, &store)
            .expect("two-filter AND must return Some");
        let mut expected = std::collections::HashSet::new();
        expected.insert("d2".to_string());
        assert_eq!(result, expected, "AND of Person+Location must yield {{d2}}");
    }

    /// Test D: unknown entity yields empty HashSet — no panic, not None.
    #[test]
    fn test_apply_entity_class_filters_unknown_entity_empty() {
        let store = make_seeded_store();
        let filters = vec![EntityClassFilter {
            class: "Person".to_string(),
            value: "Nobody Known".to_string(),
        }];
        let result = apply_entity_class_filters(&filters, &store)
            .expect("unknown entity must return Some(empty), not None");
        assert!(result.is_empty(), "unknown entity must produce empty set");
    }

    /// Test E: case-insensitive value lookup — "ALEX DOE" resolves same canonical.
    #[test]
    fn test_apply_entity_class_filters_case_insensitive_value() {
        let store = make_seeded_store();
        let filters = vec![EntityClassFilter {
            class: "Person".to_string(),
            value: "ALEX DOE".to_string(),
        }];
        let result = apply_entity_class_filters(&filters, &store)
            .expect("uppercase value must resolve same canonical");
        assert!(
            result.contains("d1") && result.contains("d2"),
            "case-insensitive value lookup must hit {{d1, d2}}"
        );
    }

    /// Test F: Phase 6/8 bridge — capitalized class "Organization" falls back to lowercase
    /// "organization" in alias_index and still hits the same canonical.
    #[test]
    fn test_apply_entity_class_filters_class_fallback_phase6_bridge() {
        let store = make_seeded_store();
        // alias_index stores ("acme corp", "organization") — lowercase entity_type (Phase 6).
        // Filter arrives with class="Organization" (capitalized, Phase 8 style).
        let filters = vec![EntityClassFilter {
            class: "Organization".to_string(),
            value: "Acme Corp".to_string(),
        }];
        let result = apply_entity_class_filters(&filters, &store)
            .expect("capitalized class must fall back to lowercase and hit canonical");
        assert!(
            result.contains("d4"),
            "Phase 6/8 bridge must find d4 via lowercase class fallback"
        );
    }

    // --- space_descendant_candidates tests (Plan 11.8-05 Task 1) ---

    /// Helper: build a SpaceManager seeded with a parent space + 2 sub-spaces,
    /// each carrying their own doc_ids, wired via `sub_space_ids`.
    fn make_seeded_space_manager() -> SpaceManager {
        use crate::spaces::manager::SpaceData;
        use crate::types::Space;

        fn make_space(id: &str, depth: u8, parent_id: Option<&str>, sub_space_ids: Vec<&str>) -> Space {
            Space {
                id: id.to_string(),
                name: format!("Space {}", id),
                icon: "Folder".to_string(),
                color: "#6D28D9".to_string(),
                document_count: 0,
                last_updated: "2026-07-08T00:00:00Z".to_string(),
                sub_spaces: vec![],
                parent_id: parent_id.map(String::from),
                sample_files: vec![],
                depth,
                sub_space_ids: sub_space_ids.into_iter().map(String::from).collect(),
                description: None,
                user_locked: false,
                canonical_entity_hint: None,
                label_status: None,
            }
        }

        let mut manager = SpaceManager::new();

        manager.insert_space_data_for_test(SpaceData {
            space: make_space("space-parent", 0, None, vec!["space-sub-1", "space-sub-2"]),
            centroid: vec![0.1, 0.2, 0.3],
            doc_ids: vec!["doc-parent-1".to_string()],
        });
        manager.insert_space_data_for_test(SpaceData {
            space: make_space("space-sub-1", 1, Some("space-parent"), vec![]),
            centroid: vec![0.11, 0.21, 0.31],
            doc_ids: vec!["doc-sub1-a".to_string(), "doc-sub1-b".to_string()],
        });
        manager.insert_space_data_for_test(SpaceData {
            space: make_space("space-sub-2", 1, Some("space-parent"), vec![]),
            centroid: vec![0.12, 0.22, 0.32],
            doc_ids: vec!["doc-sub2-a".to_string()],
        });

        manager
    }

    /// Test 1: space_descendant_candidates("space-parent") returns doc_ids from the
    /// parent AND both sub-spaces (recursive union).
    #[test]
    fn test_space_descendant_candidates_recursive_union() {
        let manager = make_seeded_space_manager();
        let result = space_descendant_candidates(&manager, "space-parent");

        let expected: HashSet<String> = [
            "doc-parent-1",
            "doc-sub1-a",
            "doc-sub1-b",
            "doc-sub2-a",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();

        assert_eq!(result, expected, "must union parent doc_ids with all descendant sub-space doc_ids");
    }

    /// Test 2: unknown space_id returns empty HashSet, does not panic.
    #[test]
    fn test_space_descendant_candidates_unknown_space_empty() {
        let manager = make_seeded_space_manager();
        let result = space_descendant_candidates(&manager, "unknown-space");
        assert!(result.is_empty(), "unknown space_id must yield empty set, not panic");
    }

    /// Test: querying a sub-space directly returns just its own doc_ids (no siblings).
    #[test]
    fn test_space_descendant_candidates_leaf_sub_space() {
        let manager = make_seeded_space_manager();
        let result = space_descendant_candidates(&manager, "space-sub-1");
        let expected: HashSet<String> = ["doc-sub1-a", "doc-sub1-b"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert_eq!(result, expected, "leaf sub-space query returns only its own doc_ids");
    }

    #[test]
    fn test_apply_metadata_filters_all_none() {
        let filters = SearchFilters {
            doc_type: None,
            space_id: None,
            date_from: None,
            date_to: None,
            tags: None,
            entity_filters: None, // Phase 11: new field; None = no entity class filter
        };
        let tmp = std::env::temp_dir().join("cortex-test-filters-none");
        let _ = std::fs::remove_dir_all(&tmp);
        let engine = CortexEngine::new_with_path(tmp.clone()).unwrap();
        let result = apply_metadata_filters(&filters, &engine).unwrap();
        assert!(result.is_none(), "all-None filters should return None");
        let _ = std::fs::remove_dir_all(tmp);
    }

    #[test]
    fn test_parse_entity_filter_amount_over() {
        let filter = parse_entity_filter("invoices over $500").unwrap();
        assert_eq!(filter.min_amount, Some(500.0));
        assert!(filter.max_amount.is_none());
    }

    #[test]
    fn test_parse_entity_filter_amount_under() {
        let filter = parse_entity_filter("receipts under $1,000.00").unwrap();
        assert_eq!(filter.max_amount, Some(1000.0));
        assert!(filter.min_amount.is_none());
    }

    #[test]
    fn test_parse_entity_filter_date_before() {
        let filter = parse_entity_filter("documents before 2024-01-01").unwrap();
        assert_eq!(filter.before_date, Some("2024-01-01".to_string()));
    }

    #[test]
    fn test_parse_entity_filter_date_after() {
        let filter = parse_entity_filter("files after 2023-06-01").unwrap();
        assert_eq!(filter.after_date, Some("2023-06-01".to_string()));
    }

    #[test]
    fn test_parse_entity_filter_no_match() {
        let result = parse_entity_filter("find my tax documents");
        assert!(result.is_none());
    }

    #[test]
    fn test_apply_entity_filter_amount_pass() {
        let filter = EntityFilter {
            min_amount: Some(100.0),
            max_amount: None,
            before_date: None,
            after_date: None,
        };
        let mut metadata = std::collections::HashMap::new();
        metadata.insert(
            "extracted_entities".to_string(),
            serde_json::json!([
                {"entity_type": "amount", "value": "$500.00", "label": "Amount"}
            ]),
        );
        assert!(apply_entity_filter(&filter, &metadata));
    }

    #[test]
    fn test_apply_entity_filter_amount_fail() {
        let filter = EntityFilter {
            min_amount: Some(1000.0),
            max_amount: None,
            before_date: None,
            after_date: None,
        };
        let mut metadata = std::collections::HashMap::new();
        metadata.insert(
            "extracted_entities".to_string(),
            serde_json::json!([
                {"entity_type": "amount", "value": "$50.00", "label": "Amount"}
            ]),
        );
        assert!(!apply_entity_filter(&filter, &metadata));
    }

    #[test]
    fn test_apply_entity_filter_no_entities() {
        let filter = EntityFilter {
            min_amount: Some(100.0),
            max_amount: None,
            before_date: None,
            after_date: None,
        };
        let metadata = std::collections::HashMap::new();
        assert!(apply_entity_filter(&filter, &metadata));
    }
}
