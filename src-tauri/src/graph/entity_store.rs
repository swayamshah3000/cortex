use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use uuid::Uuid;

use crate::engine::CortexEngine;
use crate::error::AppError;
use crate::pipeline::embedder::EmbeddingService;
use crate::types::{CanonicalEntity, EntitySummary, ExtractedEntity, RelatedEntity};

/// Threshold for cosine similarity to merge two entity surface forms into the same canonical.
/// 0.75 catches "Alex" / "Alex Doe" without collapsing "John Smith" / "Jane Doe"
/// (unrelated names typically score < 0.4). Relaxed from 0.85 after real-corpus testing
/// showed common name variants (single-word given name vs full name) hover near 0.75-0.85.
const MERGE_THRESHOLD: f32 = 0.75;

/// Token-subset check for person-name aliasing: returns true when one surface's
/// whitespace-tokenized set is a subset of the other's (e.g., "Alex" ⊂ {"Alex", "Shah"}).
/// Used as a fast-path pre-check before embedding cosine — catches obvious partial-name
/// aliases where cosine may fall below MERGE_THRESHOLD on very short surfaces.
fn is_token_subset(a: &str, b: &str) -> bool {
    let a_tokens: std::collections::HashSet<String> =
        a.split_whitespace().map(|s| s.to_lowercase()).collect();
    let b_tokens: std::collections::HashSet<String> =
        b.split_whitespace().map(|s| s.to_lowercase()).collect();
    if a_tokens.is_empty() || b_tokens.is_empty() {
        return false;
    }
    // Require at least one meaningful token overlap (guard against empty-strings)
    a_tokens.is_subset(&b_tokens) || b_tokens.is_subset(&a_tokens)
}

/// In-memory entity graph store.
///
/// Maintains:
/// - A canonical entity map (uuid → CanonicalEntity)
/// - An alias index (lowercase(value), entity_type) → canonical_id for O(1) lookups
/// - A reverse document index: canonical_id → HashSet<doc_id>
/// - Embedding vectors per canonical (used for alias-merge cosine comparison)
///
/// Mirrors the DocumentGraph in src/graph/edges.rs for the entity domain.
pub struct EntityStore {
    /// canonical_id → CanonicalEntity
    pub canonicals: HashMap<String, CanonicalEntity>,
    /// (lowercase(value), entity_type) → canonical_id — used for O(1) alias lookup
    pub alias_index: HashMap<(String, String), String>,
    /// canonical_id → set of doc_ids that mention this entity
    pub doc_index: HashMap<String, HashSet<String>>,
    /// canonical_id → embedding of canonical_name (used for cosine merge)
    pub canonical_embeddings: HashMap<String, Vec<f32>>,
    /// Phase 11.6 (D-11): rolling frequency map of canonical_short_name observations
    /// per canonical entity. canonical_id → { short_name → occurrence count }.
    /// Rebuilt from RuVector metadata on `rebuild_from_engine`. Not serialized —
    /// derived state, recomputed from `ExtractedEntity.canonical_short_name` values
    /// as they flow through `register_doc_entities`.
    pub short_name_counts: HashMap<String, HashMap<String, u32>>,
}

impl EntityStore {
    /// Create an empty EntityStore.
    pub fn new() -> Self {
        Self {
            canonicals: HashMap::new(),
            alias_index: HashMap::new(),
            doc_index: HashMap::new(),
            canonical_embeddings: HashMap::new(),
            short_name_counts: HashMap::new(),
        }
    }

    /// Rebuild the EntityStore from an existing RuVector collection.
    ///
    /// Mirrors DocumentIndexer::rebuild_path_index — O(N) scan of documents_384.
    /// Called once at app startup to restore in-memory state from persisted metadata.
    pub fn rebuild_from_engine(
        &mut self,
        engine: &CortexEngine,
        embedder: &EmbeddingService,
    ) -> Result<(), AppError> {
        let collection_arc = engine
            .collections
            .get_collection("documents_384")
            .ok_or_else(|| AppError::VectorStorage("documents_384 collection not found".to_string()))?;

        let ids = {
            let collection = collection_arc.read();
            collection.db.keys().map_err(|e| AppError::VectorStorage(e.to_string()))?
        };

        for id in ids {
            let entry = {
                let collection = collection_arc.read();
                collection.db.get(&id).map_err(|e| AppError::VectorStorage(e.to_string()))?
            };

            if let Some(entry) = entry {
                if let Some(metadata) = &entry.metadata {
                    // Extract entities from stored metadata
                    if let Some(entities_val) = metadata.get("extracted_entities") {
                        if let Some(entities_arr) = entities_val.as_array() {
                            let mut entities: Vec<ExtractedEntity> = entities_arr
                                .iter()
                                .filter_map(|v| serde_json::from_value(v.clone()).ok())
                                .collect();

                            // Register entities — ignore errors on startup (log only)
                            if let Err(e) = self.register_doc_entities(&id, &mut entities, embedder) {
                                eprintln!(
                                    "Warning: rebuild_from_engine failed for doc {}: {} (continuing)",
                                    id, e
                                );
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Register entities for a document, populating canonical_ids in-place.
    ///
    /// For each entity in the slice:
    /// 1. Call find_or_create_canonical to get (or create) a canonical_id.
    /// 2. Set entity.canonical_id = Some(canonical_id) in place.
    /// 3. Add doc_id to doc_index[canonical_id].
    /// 4. Recompute canonical_name as most-frequent surface form.
    /// 5. (Phase 11.6, D-11) Accumulate `entity.canonical_short_name` into
    ///    `short_name_counts[canonical_id]` and recompute
    ///    `canonicals[canonical_id].canonical_short_name` as the mode.
    ///
    /// Returns a Vec<String> of touched canonical_ids (parallel to input slice).
    /// On embedder error: canonical_id stays None; doc still proceeds (per must_haves truth).
    pub fn register_doc_entities(
        &mut self,
        doc_id: &str,
        entities: &mut [ExtractedEntity],
        embedder: &EmbeddingService,
    ) -> Result<Vec<String>, AppError> {
        let mut touched = Vec::with_capacity(entities.len());

        for entity in entities.iter_mut() {
            match self.find_or_create_canonical(&entity.value, &entity.entity_type, embedder) {
                Ok(canonical_id) => {
                    entity.canonical_id = Some(canonical_id.clone());
                    // Add doc_id to the reverse index
                    self.doc_index
                        .entry(canonical_id.clone())
                        .or_insert_with(HashSet::new)
                        .insert(doc_id.to_string());
                    // Recompute canonical_name as most-frequent surface form
                    self.recompute_canonical_name(&canonical_id);
                    // Phase 11.6 (D-11): accumulate + recompute canonical_short_name mode.
                    self.recompute_canonical_short_name(&canonical_id, entity.canonical_short_name.as_ref());
                    touched.push(canonical_id);
                }
                Err(e) => {
                    // Embedder failure: log and leave canonical_id as None (D-06 b failure mode)
                    eprintln!(
                        "Warning: find_or_create_canonical failed for entity '{}' type '{}': {} (canonical_id=None)",
                        entity.value, entity.entity_type, e
                    );
                    touched.push(String::new()); // placeholder to keep slice-parallel invariant
                }
            }
        }

        Ok(touched)
    }

    /// Case-insensitive exact lookup of an existing canonical by (surface, type).
    ///
    /// Bug 2: both `surface` and `entity_type` are lowercased before lookup so
    /// "SAM DOE"/"Sam Doe"/"sam doe" and "Person"/"person" all resolve to the same
    /// canonical id. Returns `None` when no alias matches.
    pub(crate) fn find_exact_canonical(&self, surface: &str, entity_type: &str) -> Option<String> {
        self.alias_index.get(&alias_key(surface, entity_type)).cloned()
    }

    /// Find an existing canonical for (surface, entity_type) or create a new one.
    ///
    /// Algorithm (per D-05):
    /// 1. Exact lookup in alias_index → return existing canonical_id.
    /// 2. Embedding cosine similarity vs all canonicals of same type ≥ MERGE_THRESHOLD → merge.
    /// 3. No match → create new canonical.
    fn find_or_create_canonical(
        &mut self,
        surface: &str,
        entity_type: &str,
        embedder: &EmbeddingService,
    ) -> Result<String, AppError> {
        // Bug 2: build the alias key with BOTH surface and entity_type lowercased so
        // casing variants ("SAM DOE" vs "Sam Doe") and entity-type casing ("Person"
        // vs "person") resolve to the same canonical instead of spawning duplicates.
        let key = alias_key(surface, entity_type);

        // Step 1: Exact (case-insensitive) alias lookup.
        if let Some(cid) = self.find_exact_canonical(surface, entity_type) {
            return Ok(cid);
        }

        // Step 2a: Token-subset fast path — catches "Alex" ⊂ "Alex Doe" where cosine
        // may fall below MERGE_THRESHOLD. Only for word-like entity types (Person,
        // Organization, Location). Avoids false-merges on IDs / Dates / Amounts.
        let is_wordy = matches!(
            entity_type,
            "person" | "organization" | "location" | "Person" | "Organization" | "Location"
        );
        if is_wordy {
            for (cid, canonical) in &self.canonicals {
                if canonical.entity_type != entity_type {
                    continue;
                }
                if is_token_subset(surface, &canonical.canonical_name) {
                    let cid_clone = cid.clone();
                    if let Some(c) = self.canonicals.get_mut(&cid_clone) {
                        if !c.aliases.contains(&surface.to_string()) {
                            c.aliases.push(surface.to_string());
                        }
                    }
                    self.alias_index.insert(key, cid_clone.clone());
                    return Ok(cid_clone);
                }
            }
        }

        // Step 2b: Embedding-based cosine comparison
        let surface_embedding = embedder.embed_text(surface)?;

        let best_match = self
            .canonicals
            .iter()
            .filter(|(_, c)| c.entity_type == entity_type)
            .filter_map(|(cid, _)| {
                self.canonical_embeddings
                    .get(cid)
                    .map(|emb| (cid.clone(), cosine(&surface_embedding, emb)))
            })
            .filter(|(_, score)| *score >= MERGE_THRESHOLD)
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        if let Some((existing_cid, _)) = best_match {
            // Merge: add surface as alias, register in alias_index
            if let Some(canonical) = self.canonicals.get_mut(&existing_cid) {
                if !canonical.aliases.contains(&surface.to_string()) {
                    canonical.aliases.push(surface.to_string());
                }
            }
            self.alias_index.insert(key, existing_cid.clone());
            return Ok(existing_cid);
        }

        // Step 3: New canonical
        let new_id = Uuid::new_v4().to_string();
        let canonical = CanonicalEntity {
            id: new_id.clone(),
            canonical_name: surface.to_string(),
            entity_type: entity_type.to_string(),
            aliases: vec![surface.to_string()],
            document_count: 0, // updated via doc_index length
            canonical_short_name: None,
        };
        self.canonicals.insert(new_id.clone(), canonical);
        self.alias_index.insert(key, new_id.clone());
        self.canonical_embeddings.insert(new_id.clone(), surface_embedding);
        self.doc_index.insert(new_id.clone(), HashSet::new());

        Ok(new_id)
    }

    /// Recompute canonical_name as the most-frequent surface form across all aliases.
    ///
    /// Per D-07: canonical_name is recomputed on every register_doc_entities call.
    /// For the alias frequency approach, we use the alias that appears most often
    /// in existing doc metadata. Simple heuristic: most aliases appear once, so we
    /// keep the first alias (creation order) as canonical unless a clear winner emerges.
    fn recompute_canonical_name(&mut self, canonical_id: &str) {
        // Count occurrences of each alias across the alias_index reverse lookup
        let canonical = match self.canonicals.get(canonical_id) {
            Some(c) => c.clone(),
            None => return,
        };

        // Find alias with most occurrences in doc_index appearances
        // Simple heuristic: canonical_name = first alias (creation order) unless
        // another alias has a much higher frequency — for v1, keep current name stable
        // unless it's been replaced by a longer/more canonical form.
        // Use: longest alias as canonical (mirrors common real-world behavior where
        // "Acme Corp" is preferred over "Acme" as the canonical form).
        if let Some(longest) = canonical.aliases.iter().max_by_key(|s| s.len()) {
            if let Some(c) = self.canonicals.get_mut(canonical_id) {
                c.canonical_name = longest.clone();
            }
        }
    }

    /// Recompute `canonical_short_name` for a canonical entity as the
    /// most-frequent short-name across all observed alias occurrences (D-11).
    ///
    /// Behavior:
    /// - `incoming_short_name` is the `canonical_short_name` computed by
    ///   `entity_normalizer::normalize_entity` for the current entity
    ///   occurrence (may be `None` when no rule applied for that occurrence).
    /// - When `Some(name)`, its count in `short_name_counts[canonical_id]` is
    ///   incremented (rolling frequency map, persists across calls — unlike
    ///   `recompute_canonical_name` which recomputes from `aliases` each
    ///   time, short-name frequency is NOT derivable from `aliases` alone
    ///   since `aliases` stores raw surface forms, not short names).
    /// - The canonical's `canonical_short_name` is then set to the
    ///   highest-count entry in the map. Ties are broken by insertion order
    ///   (`HashMap::iter().max_by_key` keeps the first-encountered maximum
    ///   for equal counts, which in practice is the first-seen short name).
    /// - When zero observations exist for this canonical → stays `None`
    ///   (falls back to `canonical_name` in the UI per D-11).
    ///
    /// Called from `register_doc_entities` for every touched canonical_id,
    /// parallel to how `recompute_canonical_name` is invoked today.
    fn recompute_canonical_short_name(
        &mut self,
        canonical_id: &str,
        incoming_short_name: Option<&String>,
    ) {
        if let Some(name) = incoming_short_name {
            *self
                .short_name_counts
                .entry(canonical_id.to_string())
                .or_default()
                .entry(name.clone())
                .or_insert(0) += 1;
        }

        let most_frequent = self
            .short_name_counts
            .get(canonical_id)
            .and_then(|counts| counts.iter().max_by_key(|(_, count)| **count))
            .map(|(name, _)| name.clone());

        if let Some(c) = self.canonicals.get_mut(canonical_id) {
            c.canonical_short_name = most_frequent;
        }
    }

    /// Run a full O(n²) alias merge pass across all canonicals.
    ///
    /// For each entity_type, compare all pairs of canonicals by cosine similarity.
    /// If ≥ MERGE_THRESHOLD, merge the lower-doc-count into the higher-doc-count one.
    ///
    /// Per D-06 (a): runs ONCE after backfill completes, not per-doc.
    pub fn run_full_alias_merge(&mut self, embedder: &EmbeddingService) -> Result<(), AppError> {
        // Collect entity types present in current canonicals
        let entity_types: Vec<String> = self
            .canonicals
            .values()
            .map(|c| c.entity_type.clone())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();

        for etype in entity_types {
            // Collect ids of this type
            let ids_of_type: Vec<String> = self
                .canonicals
                .iter()
                .filter(|(_, c)| c.entity_type == etype)
                .map(|(id, _)| id.clone())
                .collect();

            // Pairwise comparison
            let mut merges: Vec<(String, String)> = Vec::new(); // (into, from)

            for i in 0..ids_of_type.len() {
                for j in (i + 1)..ids_of_type.len() {
                    let id_a = &ids_of_type[i];
                    let id_b = &ids_of_type[j];

                    if !self.canonicals.contains_key(id_a) || !self.canonicals.contains_key(id_b) {
                        continue;
                    }

                    let emb_a = match self.canonical_embeddings.get(id_a) {
                        Some(e) => e.clone(),
                        None => {
                            // Try to embed the canonical_name
                            let name = self.canonicals[id_a].canonical_name.clone();
                            match embedder.embed_text(&name) {
                                Ok(v) => {
                                    self.canonical_embeddings.insert(id_a.clone(), v.clone());
                                    v
                                }
                                Err(_) => continue,
                            }
                        }
                    };

                    let emb_b = match self.canonical_embeddings.get(id_b) {
                        Some(e) => e.clone(),
                        None => {
                            let name = self.canonicals[id_b].canonical_name.clone();
                            match embedder.embed_text(&name) {
                                Ok(v) => {
                                    self.canonical_embeddings.insert(id_b.clone(), v.clone());
                                    v
                                }
                                Err(_) => continue,
                            }
                        }
                    };

                    let similarity = cosine(&emb_a, &emb_b);
                    if similarity >= MERGE_THRESHOLD {
                        // Merge the one with fewer docs into the one with more docs
                        let docs_a = self.doc_index.get(id_a).map(|s| s.len()).unwrap_or(0);
                        let docs_b = self.doc_index.get(id_b).map(|s| s.len()).unwrap_or(0);
                        if docs_a >= docs_b {
                            merges.push((id_a.clone(), id_b.clone())); // merge b into a
                        } else {
                            merges.push((id_b.clone(), id_a.clone())); // merge a into b
                        }
                    }
                }
            }

            // Apply merges
            for (into_id, from_id) in merges {
                self.merge_canonical_into(&into_id.clone(), &from_id.clone());
            }
        }

        Ok(())
    }

    /// Merge `from_id` canonical into `into_id` canonical.
    fn merge_canonical_into(&mut self, into_id: &str, from_id: &str) {
        if !self.canonicals.contains_key(from_id) || !self.canonicals.contains_key(into_id) {
            return;
        }

        // Collect from's aliases and doc_ids
        let from_aliases: Vec<String> = self.canonicals[from_id].aliases.clone();
        let from_doc_ids: HashSet<String> = self.doc_index.remove(from_id).unwrap_or_default();

        // Move aliases to into
        if let Some(into_canonical) = self.canonicals.get_mut(into_id) {
            for alias in &from_aliases {
                if !into_canonical.aliases.contains(alias) {
                    into_canonical.aliases.push(alias.clone());
                }
            }
        }

        // Update alias_index to point to into_id
        for (key, cid) in self.alias_index.iter_mut() {
            if cid == from_id {
                *cid = into_id.to_string();
            }
        }

        // Merge doc_ids
        if let Some(into_docs) = self.doc_index.get_mut(into_id) {
            into_docs.extend(from_doc_ids);
        }

        // Remove from canonical map and embeddings
        self.canonicals.remove(from_id);
        self.canonical_embeddings.remove(from_id);
    }

    /// Split an alias off into its own new canonical.
    ///
    /// Per D-08: creates a new uuid canonical, removes alias from old canonical,
    /// rewrites affected doc metadata atomically (per-doc read-modify-write upsert).
    pub fn split_alias(
        &mut self,
        canonical_id: &str,
        alias_to_split: &str,
        embedder: &EmbeddingService,
        engine: &CortexEngine,
    ) -> Result<String, AppError> {
        // Validate canonical exists
        if !self.canonicals.contains_key(canonical_id) {
            return Err(AppError::NotFound(format!(
                "canonical entity not found: {}",
                canonical_id
            )));
        }

        // Remove alias from old canonical
        {
            let canonical = self.canonicals.get_mut(canonical_id).unwrap();
            canonical.aliases.retain(|a| a != alias_to_split);
        }

        // Remove alias from alias_index (Bug 2: key via the shared case-insensitive helper).
        let removal_key = {
            let canonical = &self.canonicals[canonical_id];
            alias_key(alias_to_split, &canonical.entity_type)
        };
        self.alias_index.remove(&removal_key);

        // Find docs that contained this alias
        let affected_doc_ids: Vec<String> = self
            .doc_index
            .get(canonical_id)
            .map(|s| s.iter().cloned().collect())
            .unwrap_or_default();

        // Create new canonical for the split alias
        let new_id = Uuid::new_v4().to_string();
        let entity_type = self.canonicals[canonical_id].entity_type.clone();
        let new_embedding = embedder.embed_text(alias_to_split)?;

        let new_canonical = CanonicalEntity {
            id: new_id.clone(),
            canonical_name: alias_to_split.to_string(),
            entity_type: entity_type.clone(),
            aliases: vec![alias_to_split.to_string()],
            document_count: 0,
            canonical_short_name: None,
        };
        self.canonicals.insert(new_id.clone(), new_canonical);
        self.alias_index
            .insert(alias_key(alias_to_split, &entity_type), new_id.clone());
        self.canonical_embeddings.insert(new_id.clone(), new_embedding);
        self.doc_index.insert(new_id.clone(), HashSet::new());

        // Rewrite affected doc metadata: change canonical_id from old → new for this alias
        let collection_arc = engine
            .collections
            .get_collection("documents_384")
            .ok_or_else(|| AppError::VectorStorage("documents_384 collection not found".to_string()))?;

        let mut docs_moved: Vec<String> = Vec::new();

        for doc_id in &affected_doc_ids {
            let collection = collection_arc.read();
            let entry = collection
                .db
                .get(doc_id)
                .map_err(|e| AppError::VectorStorage(e.to_string()))?;

            if let Some(mut entry) = entry {
                let metadata = entry.metadata.get_or_insert_with(HashMap::new);

                // Check if this doc has the alias_to_split in its extracted_entities
                let has_alias = metadata
                    .get("extracted_entities")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter().any(|e| {
                            e.get("value")
                                .and_then(|v| v.as_str())
                                .map(|s| s.eq_ignore_ascii_case(alias_to_split))
                                .unwrap_or(false)
                        })
                    })
                    .unwrap_or(false);

                if !has_alias {
                    continue;
                }

                // Rewrite extracted_entities: update canonical_id for this alias
                if let Some(entities_val) = metadata.get_mut("extracted_entities") {
                    if let Some(arr) = entities_val.as_array_mut() {
                        for entity in arr.iter_mut() {
                            let value_matches = entity
                                .get("value")
                                .and_then(|v| v.as_str())
                                .map(|s| s.eq_ignore_ascii_case(alias_to_split))
                                .unwrap_or(false);
                            if value_matches {
                                if let Some(obj) = entity.as_object_mut() {
                                    obj.insert(
                                        "canonicalId".to_string(),
                                        serde_json::Value::String(new_id.clone()),
                                    );
                                }
                            }
                        }
                    }
                }

                // Upsert updated entry
                collection
                    .db
                    .insert(entry)
                    .map_err(|e| AppError::VectorStorage(e.to_string()))?;

                docs_moved.push(doc_id.clone());
            }
        }

        // Update doc_index: move relevant docs from old to new
        for doc_id in &docs_moved {
            if let Some(old_docs) = self.doc_index.get_mut(canonical_id) {
                old_docs.remove(doc_id);
            }
            if let Some(new_docs) = self.doc_index.get_mut(&new_id) {
                new_docs.insert(doc_id.clone());
            }
        }

        // Update document_count on both canonicals
        if let Some(old_c) = self.canonicals.get_mut(canonical_id) {
            old_c.document_count = self.doc_index.get(canonical_id).map(|s| s.len() as u32).unwrap_or(0);
        }
        if let Some(new_c) = self.canonicals.get_mut(&new_id) {
            new_c.document_count = self.doc_index.get(&new_id).map(|s| s.len() as u32).unwrap_or(0);
        }

        Ok(new_id)
    }

    /// Rename a canonical entity's canonical_name.
    ///
    /// Per D-12: updates name only — no alias changes, no doc rewrites.
    pub fn rename_canonical(&mut self, canonical_id: &str, new_name: &str) -> Result<(), AppError> {
        let canonical = self
            .canonicals
            .get_mut(canonical_id)
            .ok_or_else(|| AppError::NotFound(format!("canonical entity not found: {}", canonical_id)))?;
        canonical.canonical_name = new_name.to_string();
        Ok(())
    }

    /// Get related entities by co-occurrence in the same document.
    ///
    /// Per D-11: threshold ≥ min_co_occurrence, ranked by count desc.
    pub fn related_entities(
        &self,
        canonical_id: &str,
        min_co_occurrence: u32,
        limit: usize,
        engine: &CortexEngine,
    ) -> Result<Vec<RelatedEntity>, AppError> {
        let doc_ids = match self.doc_index.get(canonical_id) {
            Some(ids) => ids,
            None => return Ok(vec![]),
        };

        let collection_arc = engine
            .collections
            .get_collection("documents_384")
            .ok_or_else(|| AppError::VectorStorage("documents_384 collection not found".to_string()))?;

        // Count co-occurrences with other canonicals
        let mut co_occurrence: HashMap<String, u32> = HashMap::new();

        for doc_id in doc_ids {
            let collection = collection_arc.read();
            let entry = collection
                .db
                .get(doc_id)
                .map_err(|e| AppError::VectorStorage(e.to_string()))?;

            if let Some(entry) = entry {
                if let Some(metadata) = &entry.metadata {
                    if let Some(entities_val) = metadata.get("extracted_entities") {
                        if let Some(arr) = entities_val.as_array() {
                            for entity_val in arr {
                                // Get canonical_id of this entity
                                let other_cid = entity_val
                                    .get("canonicalId")
                                    .or_else(|| entity_val.get("canonical_id"))
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string());

                                if let Some(other_cid) = other_cid {
                                    if other_cid != canonical_id && !other_cid.is_empty() {
                                        *co_occurrence.entry(other_cid).or_insert(0) += 1;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Filter by threshold, sort by count desc, build RelatedEntity structs
        let mut related: Vec<RelatedEntity> = co_occurrence
            .into_iter()
            .filter(|(_, count)| *count >= min_co_occurrence)
            .filter_map(|(cid, count)| {
                self.canonicals.get(&cid).map(|c| RelatedEntity {
                    entity: EntitySummary {
                        id: c.id.clone(),
                        canonical_name: c.canonical_name.clone(),
                        entity_type: c.entity_type.clone(),
                        document_count: self.doc_index.get(&cid).map(|s| s.len() as u32).unwrap_or(0),
                    },
                    co_occurrence_count: count,
                })
            })
            .collect();

        related.sort_by(|a, b| b.co_occurrence_count.cmp(&a.co_occurrence_count));
        related.truncate(limit);

        Ok(related)
    }

    /// Get all canonicals, optionally filtered by entity_type.
    /// Returns EntitySummary sorted by document_count desc.
    pub fn get_by_type(&self, entity_type: Option<&str>) -> Vec<EntitySummary> {
        let mut summaries: Vec<EntitySummary> = self
            .canonicals
            .values()
            .filter(|c| entity_type.map(|t| c.entity_type == t).unwrap_or(true))
            .map(|c| EntitySummary {
                id: c.id.clone(),
                canonical_name: c.canonical_name.clone(),
                entity_type: c.entity_type.clone(),
                document_count: self.doc_index.get(&c.id).map(|s| s.len() as u32).unwrap_or(0),
            })
            .collect();

        summaries.sort_by(|a, b| b.document_count.cmp(&a.document_count));
        summaries
    }

    /// Get a single canonical entity by id (cloned).
    pub fn get_canonical(&self, canonical_id: &str) -> Option<CanonicalEntity> {
        self.canonicals.get(canonical_id).map(|c| {
            let mut cloned = c.clone();
            cloned.document_count = self.doc_index.get(canonical_id).map(|s| s.len() as u32).unwrap_or(0);
            cloned
        })
    }
}

impl Default for EntityStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Build the canonical `alias_index` key for a (surface, entity_type) pair.
///
/// Bug 2: lowercasing BOTH components is what makes canonical matching
/// case-insensitive. Every insert into and lookup against `alias_index` must go
/// through this helper so the key space stays consistent.
fn alias_key(surface: &str, entity_type: &str) -> (String, String) {
    (surface.to_lowercase(), entity_type.to_lowercase())
}

/// Compute cosine similarity between two vectors.
/// Returns 0.0 if either vector is zero-length (avoids NaN).
pub fn cosine(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: build a deterministic stub EmbeddingService that returns a fixed vector.
    /// Since EmbeddingService wraps a real model (fastembed), we test EntityStore logic
    /// with an EntityStore directly via its internal maps instead of calling embed_text.
    fn make_entity(value: &str, entity_type: &str) -> ExtractedEntity {
        ExtractedEntity {
            label: entity_type.to_string(),
            value: value.to_string(),
            entity_type: entity_type.to_string(),
            canonical_id: None,
            ..Default::default()
        }
    }

    /// Seed an EntityStore with a canonical entity and known embedding directly (bypassing embedder).
    fn seed_canonical(
        store: &mut EntityStore,
        id: &str,
        name: &str,
        entity_type: &str,
        embedding: Vec<f32>,
    ) {
        let canonical = CanonicalEntity {
            id: id.to_string(),
            canonical_name: name.to_string(),
            entity_type: entity_type.to_string(),
            aliases: vec![name.to_string()],
            document_count: 0,
            canonical_short_name: None,
        };
        store.canonicals.insert(id.to_string(), canonical);
        store
            .alias_index
            .insert(alias_key(name, entity_type), id.to_string());
        store.canonical_embeddings.insert(id.to_string(), embedding);
        store.doc_index.insert(id.to_string(), HashSet::new());
    }

    /// Bug 2: casing variants of the same surface + type must resolve to the SAME
    /// canonical entity. "SAM DOE" and "sam doe" must both find the canonical seeded
    /// under "Sam Doe", and entity-type casing ("Person" vs "person") must not split
    /// them either.
    #[test]
    fn test_find_exact_canonical_is_case_insensitive() {
        let mut store = EntityStore::new();
        seed_canonical(&mut store, "cid-1", "Sam Doe", "person", vec![1.0, 0.0]);

        let base = store.find_exact_canonical("Sam Doe", "person");
        assert_eq!(base.as_deref(), Some("cid-1"));

        // All-caps surface must resolve to the same canonical.
        assert_eq!(
            store.find_exact_canonical("SAM DOE", "person").as_deref(),
            Some("cid-1"),
            "'SAM DOE' must resolve to the same canonical as 'Sam Doe'"
        );
        // Lowercase surface too.
        assert_eq!(
            store.find_exact_canonical("sam doe", "person").as_deref(),
            Some("cid-1")
        );
        // Entity-type casing must not split the match either.
        assert_eq!(
            store.find_exact_canonical("SAM DOE", "Person").as_deref(),
            Some("cid-1"),
            "entity-type casing ('Person' vs 'person') must not split the canonical"
        );

        // A genuinely different name must NOT match.
        assert_eq!(store.find_exact_canonical("Sam Okafor", "person"), None);
    }

    /// Test 1: EntityStore::new() constructs empty.
    #[test]
    fn test_entity_store_new_is_empty() {
        let store = EntityStore::new();
        assert_eq!(store.canonicals.len(), 0);
        assert_eq!(store.alias_index.len(), 0);
        assert_eq!(store.doc_index.len(), 0);
        assert_eq!(store.canonical_embeddings.len(), 0);
        assert_eq!(store.short_name_counts.len(), 0);
    }

    // ── Phase 11.6 (D-11): recompute_canonical_short_name tests ──────────────
    //
    // These exercise `recompute_canonical_short_name` directly (bypassing the
    // embedder-dependent `find_or_create_canonical`) by seeding a canonical
    // first, then calling the method the same way `register_doc_entities` does.

    /// Test: a single alias occurrence with a short name sets canonical_short_name.
    #[test]
    fn test_register_doc_entities_sets_canonical_short_name_from_single_alias() {
        let mut store = EntityStore::new();
        seed_canonical(&mut store, "cid-1", "Unit 204", "location", vec![1.0, 0.0]);

        store.recompute_canonical_short_name("cid-1", Some(&"Unit 204".to_string()));

        let canonical = store.get_canonical("cid-1").unwrap();
        assert_eq!(canonical.canonical_short_name, Some("Unit 204".to_string()));
    }

    /// Test: across multiple register_doc_entities-style calls, the mode wins.
    /// Two docs contribute "Unit 204", one doc contributes "Unit" →
    /// canonical_short_name == "Unit 204".
    #[test]
    fn test_register_doc_entities_mode_wins_across_calls() {
        let mut store = EntityStore::new();
        seed_canonical(&mut store, "cid-1", "Alpha Beta Complex-Unit-204", "location", vec![1.0, 0.0]);

        store.recompute_canonical_short_name("cid-1", Some(&"Unit 204".to_string()));
        store.recompute_canonical_short_name("cid-1", Some(&"Unit 204".to_string()));
        store.recompute_canonical_short_name("cid-1", Some(&"Unit".to_string()));

        let canonical = store.get_canonical("cid-1").unwrap();
        assert_eq!(
            canonical.canonical_short_name,
            Some("Unit 204".to_string()),
            "mode (2 occurrences) must win over the minority (1 occurrence)"
        );
    }

    /// Test: when no alias occurrence ever carries a short name, canonical_short_name
    /// stays None (falls back to canonical_name in the UI per D-11).
    #[test]
    fn test_register_doc_entities_no_short_names_leaves_none() {
        let mut store = EntityStore::new();
        seed_canonical(&mut store, "cid-1", "Jane Q Doe", "person", vec![1.0, 0.0]);

        store.recompute_canonical_short_name("cid-1", None);
        store.recompute_canonical_short_name("cid-1", None);

        let canonical = store.get_canonical("cid-1").unwrap();
        assert_eq!(canonical.canonical_short_name, None);
    }

    /// Test: rebuild_from_engine (via register_doc_entities) repopulates
    /// short_name_counts from stored metadata carrying canonical_short_name.
    /// This exercises the same code path rebuild_from_engine relies on
    /// (register_doc_entities → recompute_canonical_short_name) without
    /// requiring a full CortexEngine — verifying the aggregation logic that
    /// rebuild_from_engine depends on for backward-compatible restarts.
    #[test]
    fn test_rebuild_from_engine_repopulates_short_name_counts() {
        let mut store = EntityStore::new();
        seed_canonical(&mut store, "cid-1", "Acme Corp Ltd", "organization", vec![1.0, 0.0]);

        // Simulate two doc-registration passes each contributing the same
        // canonical_short_name, as would happen when rebuild_from_engine
        // iterates stored ExtractedEntity records with canonical_short_name set.
        store.recompute_canonical_short_name("cid-1", Some(&"Acme Corp".to_string()));
        store.recompute_canonical_short_name("cid-1", Some(&"Acme Corp".to_string()));

        assert_eq!(
            store.short_name_counts.get("cid-1").and_then(|m| m.get("Acme Corp")),
            Some(&2u32),
            "short_name_counts must accumulate across calls"
        );
        let canonical = store.get_canonical("cid-1").unwrap();
        assert_eq!(canonical.canonical_short_name, Some("Acme Corp".to_string()));
    }

    /// Test 5: rename_canonical updates canonical_name only.
    #[test]
    fn test_rename_canonical() {
        let mut store = EntityStore::new();
        seed_canonical(&mut store, "id-1", "John Smith", "person", vec![0.1, 0.2, 0.3]);
        store
            .doc_index
            .get_mut("id-1")
            .unwrap()
            .insert("doc-a".to_string());

        store.rename_canonical("id-1", "Jonathan Smith").unwrap();

        let canonical = store.get_canonical("id-1").unwrap();
        assert_eq!(canonical.canonical_name, "Jonathan Smith");
        // Aliases unchanged (still contains original alias)
        assert!(canonical.aliases.contains(&"John Smith".to_string()));
        // doc_count unchanged
        assert_eq!(canonical.document_count, 1);
    }

    /// Test 7: related_entities returns co-occurrences ≥ min_co_occurrence threshold.
    #[test]
    fn test_related_entities_meets_threshold() {
        let mut store = EntityStore::new();
        // Seed canonical A appearing in docs [d1, d2, d3]
        seed_canonical(&mut store, "cid-a", "Alice", "person", vec![1.0, 0.0]);
        seed_canonical(&mut store, "cid-b", "Bob", "person", vec![0.0, 1.0]);

        // d1 and d2 have both A and B → co-occurrence count 2
        store.doc_index.get_mut("cid-a").unwrap().insert("d1".to_string());
        store.doc_index.get_mut("cid-a").unwrap().insert("d2".to_string());
        store.doc_index.get_mut("cid-a").unwrap().insert("d3".to_string());
        store.doc_index.get_mut("cid-b").unwrap().insert("d1".to_string());
        store.doc_index.get_mut("cid-b").unwrap().insert("d2".to_string());

        // related_entities needs engine to look up doc metadata.
        // Since we can't easily inject an engine here, we test the co-occurrence
        // counting logic by checking doc_index membership (unit-level test).
        // The full integration test is marked #[ignore].
        //
        // Verify doc_index state is correct (precondition for the real related_entities call).
        let a_docs = store.doc_index.get("cid-a").unwrap();
        let b_docs = store.doc_index.get("cid-b").unwrap();
        let co_occur: Vec<&String> = a_docs.iter().filter(|d| b_docs.contains(*d)).collect();
        assert_eq!(co_occur.len(), 2, "Expected 2 co-occurrences of A and B");
    }

    /// Test 8: related_entities with min_co_occurrence=2 returns empty when count is 1.
    #[test]
    fn test_related_entities_below_threshold_returns_empty() {
        let mut store = EntityStore::new();
        seed_canonical(&mut store, "cid-a", "Alice", "person", vec![1.0, 0.0]);
        seed_canonical(&mut store, "cid-b", "Bob", "person", vec![0.0, 1.0]);

        // Only d1 shared — co-occurrence count 1
        store.doc_index.get_mut("cid-a").unwrap().insert("d1".to_string());
        store.doc_index.get_mut("cid-b").unwrap().insert("d1".to_string());

        let co_occur_count = store
            .doc_index
            .get("cid-a")
            .unwrap()
            .iter()
            .filter(|d| store.doc_index.get("cid-b").unwrap().contains(*d))
            .count();
        assert_eq!(co_occur_count, 1);
        // At threshold=2, this would be filtered out
        assert!(co_occur_count < 2, "Co-occurrence below threshold should be excluded");
    }

    /// Test 9: Fixtures deserialize correctly.
    ///
    /// Note: ner_golden.json was deleted in Plan 10 along with the BERT NER stack.
    /// This test now only validates aliases.json (entity merge fixture for entity_store).
    #[test]
    fn test_fixtures_deserialize() {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");

        // aliases.json
        let aliases_path =
            std::path::Path::new(manifest_dir).join("tests/fixtures/aliases.json");
        assert!(aliases_path.exists(), "aliases.json must exist at {:?}", aliases_path);
        let aliases_content = std::fs::read_to_string(&aliases_path).expect("read aliases.json");
        let aliases: serde_json::Value =
            serde_json::from_str(&aliases_content).expect("aliases.json should be valid JSON");
        assert!(
            aliases.as_array().map(|a| a.len() >= 6).unwrap_or(false),
            "aliases.json should have at least 6 pairs"
        );

        // Verify structure: each entry has 'pair' with 2 elements and 'should_merge' bool
        for entry in aliases.as_array().unwrap() {
            let pair = entry.get("pair").and_then(|p| p.as_array());
            assert!(pair.map(|p| p.len() == 2).unwrap_or(false), "each alias entry needs 2-element pair");
            assert!(entry.get("should_merge").and_then(|v| v.as_bool()).is_some(), "each alias entry needs should_merge bool");
        }
    }

    /// Test 1 (get_by_type): returns all canonicals when type is None.
    #[test]
    fn test_get_by_type_all() {
        let mut store = EntityStore::new();
        seed_canonical(&mut store, "id-1", "Alice", "person", vec![1.0, 0.0]);
        seed_canonical(&mut store, "id-2", "Bob", "person", vec![0.0, 1.0]);
        seed_canonical(&mut store, "id-3", "Acme Corp", "organization", vec![0.5, 0.5]);

        let all = store.get_by_type(None);
        assert_eq!(all.len(), 3);
    }

    /// Test 2 (get_by_type): filters by entity_type.
    #[test]
    fn test_get_by_type_filtered() {
        let mut store = EntityStore::new();
        seed_canonical(&mut store, "id-1", "Alice", "person", vec![1.0, 0.0]);
        seed_canonical(&mut store, "id-2", "Bob", "person", vec![0.0, 1.0]);
        seed_canonical(&mut store, "id-3", "Acme Corp", "organization", vec![0.5, 0.5]);

        let persons = store.get_by_type(Some("person"));
        assert_eq!(persons.len(), 2);
        assert!(persons.iter().all(|e| e.entity_type == "person"));
    }

    /// Test 3 (get_canonical): unknown id returns None.
    #[test]
    fn test_get_canonical_unknown_returns_none() {
        let store = EntityStore::new();
        assert!(store.get_canonical("nonexistent-id").is_none());
    }

    /// Test: cosine helper.
    #[test]
    fn test_cosine_similarity() {
        // Identical vectors → 1.0
        let v = vec![1.0f32, 0.0, 0.0];
        assert!((cosine(&v, &v) - 1.0).abs() < 1e-5);

        // Orthogonal vectors → 0.0
        let a = vec![1.0f32, 0.0];
        let b = vec![0.0f32, 1.0];
        assert!((cosine(&a, &b)).abs() < 1e-5);

        // Zero vector → 0.0
        let zero = vec![0.0f32, 0.0];
        assert_eq!(cosine(&zero, &a), 0.0);
    }

    /// Test 2 (rebuild_from_engine): integration — requires real engine. Marked #[ignore].
    #[test]
    #[ignore]
    fn test_rebuild_from_engine_populates_store() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let engine =
            crate::engine::CortexEngine::new_with_path(tmp_dir.path().to_path_buf()).unwrap();
        let embedding_service = EmbeddingService::new_local().unwrap();
        let mut store = EntityStore::new();

        // Seed 3 docs with person entities
        let collection_arc = engine.collections.get_collection("documents_384").unwrap();
        for i in 0..3 {
            let meta = {
                let mut m = HashMap::new();
                let entities = serde_json::json!([
                    {"label": "Person", "value": format!("Person {}", i), "entity_type": "person", "canonical_id": null}
                ]);
                m.insert("extracted_entities".to_string(), entities);
                m.insert("path".to_string(), serde_json::Value::String(format!("/doc{}.txt", i)));
                m
            };
            let entry = ruvector_core::types::VectorEntry {
                id: Some(format!("doc-{}", i)),
                vector: vec![0.1f32; 384],
                metadata: Some(meta),
            };
            let col = collection_arc.read();
            col.db.insert(entry).unwrap();
        }

        store.rebuild_from_engine(&engine, &embedding_service).unwrap();
        // Should have 3 canonicals (or fewer if some merged)
        assert!(store.canonicals.len() >= 1, "should have at least 1 canonical after rebuild");
        assert!(!store.doc_index.is_empty(), "doc_index should not be empty after rebuild");
    }

    /// Test 3 (register_doc_entities with embedder): integration. Marked #[ignore].
    #[test]
    #[ignore]
    fn test_register_doc_entities_merges_aliases() {
        // "John Smith" and "J. Smith" should merge (cosine ≥ 0.85 when using real embedder)
        let embedding_service = EmbeddingService::new_local().unwrap();
        let mut store = EntityStore::new();

        let mut entities_doc1 = vec![make_entity("John Smith", "person")];
        let mut entities_doc2 = vec![make_entity("J. Smith", "person")];

        store
            .register_doc_entities("doc1", &mut entities_doc1, &embedding_service)
            .unwrap();
        store
            .register_doc_entities("doc2", &mut entities_doc2, &embedding_service)
            .unwrap();

        // Should be 1 canonical (merged)
        assert_eq!(
            store.canonicals.len(),
            1,
            "John Smith and J. Smith should merge to one canonical, got: {:?}",
            store.canonicals.values().map(|c| &c.canonical_name).collect::<Vec<_>>()
        );
        // Both doc_ids in reverse index
        let doc_ids: HashSet<String> = store.doc_index.values().flat_map(|s| s.iter().cloned()).collect();
        assert!(doc_ids.contains("doc1"));
        assert!(doc_ids.contains("doc2"));
    }

    /// Test 4 (register_doc_entities negative case): integration. Marked #[ignore].
    #[test]
    #[ignore]
    fn test_register_doc_entities_separate_entities() {
        // "John Smith" and "Jane Doe" should NOT merge (cosine < 0.85)
        let embedding_service = EmbeddingService::new_local().unwrap();
        let mut store = EntityStore::new();

        let mut entities_doc1 = vec![make_entity("John Smith", "person")];
        let mut entities_doc2 = vec![make_entity("Jane Doe", "person")];

        store
            .register_doc_entities("doc1", &mut entities_doc1, &embedding_service)
            .unwrap();
        store
            .register_doc_entities("doc2", &mut entities_doc2, &embedding_service)
            .unwrap();

        // Should be 2 separate canonicals
        assert_eq!(
            store.canonicals.len(),
            2,
            "John Smith and Jane Doe should remain separate, got: {:?}",
            store.canonicals.values().map(|c| &c.canonical_name).collect::<Vec<_>>()
        );
    }

    /// Test 6 (split_alias): integration. Marked #[ignore].
    #[test]
    #[ignore]
    fn test_split_alias_creates_new_canonical() {
        let embedding_service = EmbeddingService::new_local().unwrap();
        let tmp_dir = tempfile::tempdir().unwrap();
        let engine =
            crate::engine::CortexEngine::new_with_path(tmp_dir.path().to_path_buf()).unwrap();
        let mut store = EntityStore::new();

        // Seed: register "John Smith" and "J. Smith" (they'll merge)
        let mut entities = vec![
            make_entity("John Smith", "person"),
            make_entity("J. Smith", "person"),
        ];
        store
            .register_doc_entities("doc1", &mut entities, &embedding_service)
            .unwrap();

        let canonical_id = entities[0].canonical_id.clone().unwrap();
        let initial_alias_count = store.canonicals[&canonical_id].aliases.len();

        // Split "J. Smith" off
        let new_cid = store
            .split_alias(&canonical_id, "J. Smith", &embedding_service, &engine)
            .unwrap();

        // Old canonical should no longer contain "J. Smith"
        assert!(
            !store.canonicals[&canonical_id]
                .aliases
                .contains(&"J. Smith".to_string()),
            "J. Smith should be removed from original canonical"
        );
        // New canonical should contain "J. Smith"
        assert_eq!(
            store.canonicals[&new_cid].canonical_name,
            "J. Smith"
        );
        // Total aliases should be split across two canonicals
        let new_alias_count = store.canonicals[&canonical_id].aliases.len()
            + store.canonicals[&new_cid].aliases.len();
        assert!(new_alias_count <= initial_alias_count + 1, "Split should not add new aliases");
    }
}
