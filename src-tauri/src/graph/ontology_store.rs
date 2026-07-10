//! In-memory store for the adaptive ontology (predicates + entity subclasses
//! + pending consolidation), backed by a JSON sidecar at
//! `{app_data_dir}/ontology.json`.
//!
//! Mirrors the `SavedSearchStore` / `TripleStore` / `SpaceLabelCache`
//! JSON-sidecar pattern (D-19 from 11.6-CONTEXT.md): silent-fail on load,
//! atomic tmp-file + rename on save, indices rebuilt from the persisted
//! schema on every load.
//!
//! # Vocabulary layering (D-03, D-18)
//! The *effective* predicate vocabulary is the deterministic merge of four
//! layers, in this priority order (later layers never override earlier ones
//! for the same predicate name):
//! 1. `SEED_PREDICATES` — the frozen 21-token Phase 11.5 baseline. Always
//!    present, always source=Seed, immutable (never renamed/merged/removed).
//! 2. `corpus_seed` — bootstrap output (D-01/D-02), source=Corpus.
//! 3. `manual_predicates` — user-added via Settings (D-21), source=Manual.
//! 4. `adaptive_predicates` — promoted from Pass 3 `new_predicates` after
//!    crossing the minimum-support gate (D-06/D-07), source=Adaptive.
//!
//! # Minimum-support promotion gate (D-06, D-08)
//! New predicates observed by Pass 3 land in `pending_predicates` and only
//! graduate to `adaptive_predicates` after being seen in
//! `PENDING_PROMOTION_MIN_SUPPORT` (2) distinct documents, and only while the
//! effective vocabulary is under `VOCABULARY_HARD_CAP` (200). This prevents
//! one-off LLM hallucinations from polluting the vocabulary and caps runaway
//! growth (T-11.6-04).
//!
//! # Seed immutability (D-03, T-11.6-05)
//! `rename_predicate` and `merge_predicates` reject any seed-predicate name
//! as an argument — the 21 baked-in predicates always survive a "reset to
//! seed" and can never be renamed away or merged into something else.
//!
//! # Error resilience (T-11.6-07)
//! `load()` never panics: any I/O or JSON parse error silently returns
//! `Self::default()`, matching `SavedSearchStore`/`TripleStore` resilience
//! contracts.

use crate::error::AppError;
use crate::types::{
    BootstrapSeed, ConsolidationKind, EntitySubclass, OntologyStoreSchema, PendingConsolidation,
    Predicate, PromoteResult, PromotionSource, SEED_PREDICATES, VOCABULARY_HARD_CAP,
    PENDING_PROMOTION_MIN_SUPPORT,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// Kind of triple-level rewrite a caller must apply to `TripleStore` after
/// `OntologyStore::apply_consolidation` mutates the vocabulary. The
/// `OntologyStore` only owns vocabulary state — it never touches
/// `TripleStore` directly, so this instruction is the hand-off contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TripleRewriteKind {
    Rename,
    Merge,
    Split,
}

/// Describes a rewrite the caller must apply to `TripleStore` triples after
/// a consolidation suggestion is applied to the ontology vocabulary.
///
/// - `Rename`: rewrite every triple with predicate `from[0]` to use `to`.
/// - `Merge`: rewrite every triple whose predicate is any of `from` to `to`.
/// - `Split`: `to` is empty — Phase 11.6 does not auto-rewrite splits; the
///   vocabulary entries for `from`'s replacements are registered, but the
///   caller must defer actual triple reassignment to the user (D-16/D-17).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TripleRewriteInstruction {
    pub kind: TripleRewriteKind,
    pub from: Vec<String>,
    pub to: String,
}

/// In-memory mirror of `{app_data_dir}/ontology.json`, plus two derived
/// indices rebuilt on every load and kept in sync on every mutation.
pub struct OntologyStore {
    schema: OntologyStoreSchema,
    /// Forward index: predicate name -> source (Seed/Corpus/Adaptive/Manual).
    /// Rebuilt on load and on every mutation. Does NOT include
    /// `pending_predicates` (those are not yet part of the vocabulary).
    name_to_source: HashMap<String, PromotionSource>,
    /// Reverse index: entity class -> set of observed subclasses.
    class_to_subclasses: HashMap<String, HashSet<String>>,
    /// Set of the SEED_PREDICATES names for O(1) contains() checks. Seed
    /// predicates always win on rename/merge collision (D-03) and can never
    /// be renamed or merged away.
    seed_set: HashSet<String>,
}

impl Default for OntologyStore {
    fn default() -> Self {
        let schema = OntologyStoreSchema::default();
        let seed_set: HashSet<String> = SEED_PREDICATES.iter().map(|s| s.to_string()).collect();
        let mut name_to_source = HashMap::new();
        for name in &seed_set {
            name_to_source.insert(name.clone(), PromotionSource::Seed);
        }
        Self {
            schema,
            name_to_source,
            class_to_subclasses: HashMap::new(),
            seed_set,
        }
    }
}

/// Validate a manual/adaptive predicate name: non-empty, <= 64 chars,
/// snake_case (`^[a-z][a-z0-9_]*$`).
fn is_valid_predicate_name(name: &str) -> bool {
    if name.is_empty() || name.len() > 64 {
        return false;
    }
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_lowercase() {
        return false;
    }
    chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

impl OntologyStore {
    /// Load the store from `{app_data_dir}/ontology.json`.
    ///
    /// Returns `Self::default()` on any I/O or JSON parse error — never
    /// panics (T-11.6-07 mitigation, mirrors `SavedSearchStore`/`TripleStore`).
    pub fn load(app_data_dir: &Path) -> Self {
        let path = app_data_dir.join("ontology.json");
        let store = match std::fs::read_to_string(&path) {
            Ok(s) => {
                let schema: OntologyStoreSchema =
                    serde_json::from_str(&s).unwrap_or_default();
                Self::from_schema(schema)
            }
            Err(_) => Self::default(),
        };
        eprintln!(
            "[ontology_store] loaded {} adaptive + {} pending + {} manual predicates from {}",
            store.schema.adaptive_predicates.len(),
            store.schema.pending_predicates.len(),
            store.schema.manual_predicates.len(),
            path.display()
        );
        store
    }

    /// Build an `OntologyStore` from a deserialized schema, rebuilding both
    /// derived indices. Insertion order for `name_to_source` is
    /// Seed -> Corpus -> Manual -> Adaptive so later inserts never clobber
    /// an earlier authoritative source for the same name (in practice, seed
    /// names are never duplicated in corpus/manual/adaptive lists because
    /// mutation methods reject them up front — D-03).
    fn from_schema(schema: OntologyStoreSchema) -> Self {
        let seed_set: HashSet<String> = SEED_PREDICATES.iter().map(|s| s.to_string()).collect();
        let mut name_to_source = HashMap::new();
        for name in &seed_set {
            name_to_source.insert(name.clone(), PromotionSource::Seed);
        }
        if let Some(corpus_seed) = &schema.corpus_seed {
            for p in &corpus_seed.predicates {
                name_to_source
                    .entry(p.name.clone())
                    .or_insert(PromotionSource::Corpus);
            }
        }
        for p in &schema.manual_predicates {
            name_to_source
                .entry(p.name.clone())
                .or_insert(PromotionSource::Manual);
        }
        for p in &schema.adaptive_predicates {
            name_to_source
                .entry(p.name.clone())
                .or_insert(PromotionSource::Adaptive);
        }

        let mut class_to_subclasses: HashMap<String, HashSet<String>> = HashMap::new();
        for es in &schema.entity_subclasses {
            class_to_subclasses
                .entry(es.class.clone())
                .or_default()
                .insert(es.subclass.clone());
        }

        Self {
            schema,
            name_to_source,
            class_to_subclasses,
            seed_set,
        }
    }

    /// Persist the store to `{app_data_dir}/ontology.json` atomically via a
    /// `.json.tmp` write + rename, mirroring `SavedSearchStore::save`.
    ///
    /// Never called automatically on mutation — IPC commands (Plan 06) and
    /// background tasks (Plans 04/05/07) call this explicitly.
    pub fn save(&self, app_data_dir: &Path) -> std::io::Result<()> {
        if !app_data_dir.exists() {
            std::fs::create_dir_all(app_data_dir)?;
        }
        let path = app_data_dir.join("ontology.json");
        let tmp = path.with_extension("json.tmp");
        let json = serde_json::to_string_pretty(&self.schema)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        std::fs::write(&tmp, json)?;
        std::fs::rename(tmp, path)?;
        Ok(())
    }

    /// Deterministic merge of the effective predicate vocabulary:
    /// SEED_PREDICATES, then corpus_seed, then manual_predicates, then
    /// adaptive_predicates — each layer skipping any name already present
    /// from an earlier layer (D-03, D-05 prompt determinism).
    pub fn effective_predicates(&self) -> Vec<Predicate> {
        let mut seen: HashSet<String> = HashSet::new();
        let mut out: Vec<Predicate> = Vec::new();

        for name in SEED_PREDICATES {
            let name = name.to_string();
            if seen.insert(name.clone()) {
                out.push(Predicate {
                    name,
                    description: "Baseline vocabulary from Phase 11.5".to_string(),
                    source: PromotionSource::Seed,
                    count: u32::MAX,
                    first_seen_doc_id: None,
                    first_seen_at: None,
                    promoted_at: None,
                    subject_class: None,
                    object_class: None,
                });
            }
        }

        if let Some(corpus_seed) = &self.schema.corpus_seed {
            for p in &corpus_seed.predicates {
                if seen.insert(p.name.clone()) {
                    out.push(p.clone());
                }
            }
        }

        for p in &self.schema.manual_predicates {
            if seen.insert(p.name.clone()) {
                out.push(p.clone());
            }
        }

        for p in &self.schema.adaptive_predicates {
            if seen.insert(p.name.clone()) {
                out.push(p.clone());
            }
        }

        out
    }

    /// Names only, in the same deterministic order as `effective_predicates()`.
    pub fn effective_predicate_names(&self) -> Vec<String> {
        self.effective_predicates().into_iter().map(|p| p.name).collect()
    }

    /// All observed entity subclasses (bootstrap-seeded, D-18).
    pub fn entity_subclasses(&self) -> Vec<EntitySubclass> {
        self.schema.entity_subclasses.clone()
    }

    pub fn automatic_growth_enabled(&self) -> bool {
        self.schema.automatic_growth_enabled
    }

    pub fn set_automatic_growth(&mut self, enabled: bool) {
        self.schema.automatic_growth_enabled = enabled;
    }

    pub fn bootstrap_completed(&self) -> bool {
        self.schema.bootstrap_completed_at.is_some()
    }

    /// Clone the full on-disk schema for read-through IPC responses (Plan 06
    /// `get_ontology`). `OntologyStoreSchema` derives `Clone`, so this is a
    /// cheap deep-copy of the current in-memory state — no I/O.
    pub fn schema_clone(&self) -> OntologyStoreSchema {
        self.schema.clone()
    }

    // === Mutation API ===

    /// Record a Pass-3-observed predicate occurrence. Transitions
    /// pending -> adaptive once `PENDING_PROMOTION_MIN_SUPPORT` distinct-doc
    /// occurrences are reached, gated by `VOCABULARY_HARD_CAP` (D-06, D-08).
    pub fn record_pending_predicate(
        &mut self,
        name: &str,
        description: &str,
        subject_class: Option<String>,
        object_class: Option<String>,
        doc_id: &str,
        now_rfc3339: &str,
    ) -> PromoteResult {
        if name.is_empty() || name.len() > 64 {
            return PromoteResult::CapExceeded;
        }
        if self.name_to_source.contains_key(name) {
            return PromoteResult::AlreadyPresent;
        }

        if let Some(pos) = self
            .schema
            .pending_predicates
            .iter()
            .position(|p| p.name == name)
        {
            self.schema.pending_predicates[pos].count += 1;
            let count = self.schema.pending_predicates[pos].count;

            if count >= PENDING_PROMOTION_MIN_SUPPORT {
                if self.effective_predicate_names().len() < VOCABULARY_HARD_CAP {
                    let mut promoted = self.schema.pending_predicates.remove(pos);
                    promoted.source = PromotionSource::Adaptive;
                    promoted.promoted_at = Some(now_rfc3339.to_string());
                    self.name_to_source
                        .insert(promoted.name.clone(), PromotionSource::Adaptive);
                    self.schema.adaptive_predicates.push(promoted);
                    return PromoteResult::Promoted;
                }
                return PromoteResult::CapExceeded;
            }

            return PromoteResult::StillPending { count };
        }

        self.schema.pending_predicates.push(Predicate {
            name: name.to_string(),
            description: description.to_string(),
            source: PromotionSource::Adaptive,
            count: 1,
            first_seen_doc_id: Some(doc_id.to_string()),
            first_seen_at: Some(now_rfc3339.to_string()),
            promoted_at: None,
            subject_class,
            object_class,
        });
        PromoteResult::StillPending { count: 1 }
    }

    /// Apply the corpus-seeded bootstrap output (D-01, D-02). Idempotent:
    /// a no-op if bootstrap already ran once for this install (D-02). Skips
    /// any predicate whose name collides with a seed predicate (D-03).
    pub fn apply_bootstrap(&mut self, seed: BootstrapSeed, now_rfc3339: &str) {
        if self.schema.bootstrap_completed_at.is_some() {
            return;
        }

        for p in &seed.predicates {
            if self.seed_set.contains(&p.name) {
                continue;
            }
            self.name_to_source
                .entry(p.name.clone())
                .or_insert(PromotionSource::Corpus);
        }

        for es in &seed.entity_subclasses {
            let already_present = self.schema.entity_subclasses.iter().any(|existing| {
                existing.class == es.class && existing.subclass == es.subclass
            });
            if !already_present {
                self.schema.entity_subclasses.push(es.clone());
            }
            self.class_to_subclasses
                .entry(es.class.clone())
                .or_default()
                .insert(es.subclass.clone());
        }

        self.schema.corpus_seed = Some(seed);
        self.schema.bootstrap_completed_at = Some(now_rfc3339.to_string());
    }

    /// Register a user-authored predicate via Settings > Ontology (D-21).
    /// Validates snake_case naming, rejects duplicates and cap overflow.
    pub fn register_manual_predicate(
        &mut self,
        name: String,
        description: String,
        subject_class: Option<String>,
        object_class: Option<String>,
        now_rfc3339: &str,
    ) -> PromoteResult {
        if !is_valid_predicate_name(&name) {
            return PromoteResult::CapExceeded;
        }
        if self.name_to_source.contains_key(&name) {
            return PromoteResult::AlreadyPresent;
        }
        if self.effective_predicate_names().len() >= VOCABULARY_HARD_CAP {
            return PromoteResult::CapExceeded;
        }

        self.name_to_source
            .insert(name.clone(), PromotionSource::Manual);
        self.schema.manual_predicates.push(Predicate {
            name,
            description,
            source: PromotionSource::Manual,
            count: 0,
            first_seen_doc_id: None,
            first_seen_at: Some(now_rfc3339.to_string()),
            promoted_at: None,
            subject_class,
            object_class,
        });
        PromoteResult::Promoted
    }

    /// Rename a non-seed predicate in place. Seed predicates can never be
    /// renamed (D-03, T-11.6-05). Only mutates ontology vocabulary — the
    /// caller is responsible for rewriting existing `TripleStore` triples.
    pub fn rename_predicate(
        &mut self,
        old: &str,
        new: &str,
        now_rfc3339: &str,
    ) -> Result<(), AppError> {
        if self.seed_set.contains(old) {
            return Err(AppError::Invalid(
                "cannot rename seed predicate".to_string(),
            ));
        }
        if !is_valid_predicate_name(new) {
            return Err(AppError::Invalid(format!(
                "invalid predicate name: {new}"
            )));
        }

        let found = Self::find_and_rename_in(&mut self.schema.pending_predicates, old, new, now_rfc3339)
            || Self::find_and_rename_in(&mut self.schema.adaptive_predicates, old, new, now_rfc3339)
            || Self::find_and_rename_in(&mut self.schema.manual_predicates, old, new, now_rfc3339)
            || self
                .schema
                .corpus_seed
                .as_mut()
                .map(|cs| Self::find_and_rename_in(&mut cs.predicates, old, new, now_rfc3339))
                .unwrap_or(false);

        if !found {
            return Err(AppError::NotFound(format!("predicate: {old}")));
        }

        if let Some(source) = self.name_to_source.remove(old) {
            self.name_to_source.insert(new.to_string(), source);
        }

        Ok(())
    }

    /// Helper: find `old` by name in `list` and rename in place. Returns
    /// `true` if found (and mutated), `false` otherwise. Only sets
    /// `promoted_at` for non-pending lists is left to the caller's
    /// semantics — here we just stamp it unconditionally, matching the
    /// plan's "mutate in place" instruction.
    fn find_and_rename_in(list: &mut [Predicate], old: &str, new: &str, now_rfc3339: &str) -> bool {
        if let Some(entry) = list.iter_mut().find(|p| p.name == old) {
            entry.name = new.to_string();
            entry.promoted_at = Some(now_rfc3339.to_string());
            true
        } else {
            false
        }
    }

    /// Merge `from` predicates into `into`. Seed predicates cannot appear in
    /// `from` (D-03). `into` must already exist in the effective vocabulary.
    pub fn merge_predicates(
        &mut self,
        from: Vec<String>,
        into: String,
        _now_rfc3339: &str,
    ) -> Result<(), AppError> {
        if from.iter().any(|f| self.seed_set.contains(f)) {
            return Err(AppError::Invalid(
                "cannot merge seed predicate".to_string(),
            ));
        }
        if !self.name_to_source.contains_key(&into) {
            return Err(AppError::NotFound(format!("predicate: {into}")));
        }

        let mut merged_count: u32 = 0;

        for name in &from {
            if *name == into {
                continue;
            }
            merged_count += Self::remove_and_take_count(&mut self.schema.pending_predicates, name);
            merged_count += Self::remove_and_take_count(&mut self.schema.adaptive_predicates, name);
            merged_count += Self::remove_and_take_count(&mut self.schema.manual_predicates, name);
            if let Some(cs) = self.schema.corpus_seed.as_mut() {
                merged_count += Self::remove_and_take_count(&mut cs.predicates, name);
            }
            self.name_to_source.remove(name);
        }

        if merged_count > 0 {
            for list in [
                &mut self.schema.adaptive_predicates,
                &mut self.schema.manual_predicates,
                &mut self.schema.pending_predicates,
            ] {
                if let Some(entry) = list.iter_mut().find(|p| p.name == into) {
                    entry.count += merged_count;
                    break;
                }
            }
        }

        Ok(())
    }

    /// Remove all entries named `name` from `list`, returning the sum of
    /// their `count` fields (used to fold merged support counts into the
    /// surviving `into` predicate).
    fn remove_and_take_count(list: &mut Vec<Predicate>, name: &str) -> u32 {
        let mut total = 0;
        list.retain(|p| {
            if p.name == name {
                total += p.count;
                false
            } else {
                true
            }
        });
        total
    }

    /// Wipe all corpus-seeded / adaptive / pending / manual state and
    /// entity subclasses back to the frozen seed vocabulary. Preserves the
    /// user's `automatic_growth_enabled` preference (D-18 discretion: user
    /// settings are user-owned, not derived from corpus).
    pub fn reset_to_seed(&mut self, _now_rfc3339: &str) {
        let automatic_growth_enabled = self.schema.automatic_growth_enabled;
        self.schema = OntologyStoreSchema {
            automatic_growth_enabled,
            ..OntologyStoreSchema::default()
        };
        self.class_to_subclasses.clear();
        self.name_to_source.clear();
        for name in &self.seed_set {
            self.name_to_source.insert(name.clone(), PromotionSource::Seed);
        }
    }

    /// Clear only `corpus_seed` + `bootstrap_completed_at` (D-01/D-02
    /// "Regenerate ontology" action, Plan 06). Preserves
    /// `adaptive_predicates`, `pending_predicates`, `manual_predicates`,
    /// `entity_subclasses`, and `pending_consolidation` — unlike
    /// `reset_to_seed`, this is NOT a nuclear wipe. Removing the corpus_seed
    /// predicates from `name_to_source` lets the next 30-doc backfill batch
    /// re-trigger `apply_bootstrap` without colliding with stale names.
    pub fn clear_corpus_seed_for_regeneration(&mut self, _now_rfc3339: &str) {
        if let Some(corpus_seed) = self.schema.corpus_seed.take() {
            for p in &corpus_seed.predicates {
                self.name_to_source.remove(&p.name);
            }
        }
        self.schema.bootstrap_completed_at = None;
    }

    pub fn set_pending_consolidation(&mut self, pc: Option<PendingConsolidation>) {
        self.schema.pending_consolidation = pc;
    }

    pub fn pending_consolidation(&self) -> Option<PendingConsolidation> {
        self.schema.pending_consolidation.clone()
    }

    /// Apply a single pending consolidation suggestion by id: mutate the
    /// ontology vocabulary accordingly and return the rewrite instructions
    /// the caller must apply to `TripleStore` (D-16, D-17, D-20).
    pub fn apply_consolidation(
        &mut self,
        suggestion_id: &str,
        now_rfc3339: &str,
    ) -> Result<Vec<TripleRewriteInstruction>, AppError> {
        let Some(pc) = self.schema.pending_consolidation.as_mut() else {
            return Err(AppError::NotFound(format!(
                "consolidation suggestion: {suggestion_id}"
            )));
        };

        let pos = pc
            .suggestions
            .iter()
            .position(|s| s.id == suggestion_id)
            .ok_or_else(|| {
                AppError::NotFound(format!("consolidation suggestion: {suggestion_id}"))
            })?;

        let suggestion = pc.suggestions.remove(pos);
        let suggestions_now_empty = pc.suggestions.is_empty();

        let instructions = match suggestion.kind {
            ConsolidationKind::Rename { from, to } => {
                self.rename_predicate(&from, &to, now_rfc3339)?;
                vec![TripleRewriteInstruction {
                    kind: TripleRewriteKind::Rename,
                    from: vec![from],
                    to,
                }]
            }
            ConsolidationKind::Merge { from, into } => {
                self.merge_predicates(from.clone(), into.clone(), now_rfc3339)?;
                vec![TripleRewriteInstruction {
                    kind: TripleRewriteKind::Merge,
                    from,
                    to: into,
                }]
            }
            ConsolidationKind::Split { from, into } => {
                if self.seed_set.contains(&from) {
                    return Err(AppError::Invalid(
                        "cannot split seed predicate".to_string(),
                    ));
                }
                for name in &into {
                    self.register_manual_predicate(
                        name.clone(),
                        format!("Split from {from}"),
                        None,
                        None,
                        now_rfc3339,
                    );
                }
                // Remove `from` from adaptive/manual/pending (not seed —
                // guarded above).
                Self::remove_and_take_count(&mut self.schema.pending_predicates, &from);
                Self::remove_and_take_count(&mut self.schema.adaptive_predicates, &from);
                Self::remove_and_take_count(&mut self.schema.manual_predicates, &from);
                if let Some(cs) = self.schema.corpus_seed.as_mut() {
                    Self::remove_and_take_count(&mut cs.predicates, &from);
                }
                self.name_to_source.remove(&from);
                vec![TripleRewriteInstruction {
                    kind: TripleRewriteKind::Split,
                    from: vec![from],
                    to: String::new(),
                }]
            }
        };

        if suggestions_now_empty {
            self.schema.pending_consolidation = None;
        }
        self.schema.last_consolidation_at = Some(now_rfc3339.to_string());

        Ok(instructions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn predicate(name: &str, source: PromotionSource, count: u32) -> Predicate {
        Predicate {
            name: name.to_string(),
            description: format!("desc for {name}"),
            source,
            count,
            first_seen_doc_id: None,
            first_seen_at: None,
            promoted_at: None,
            subject_class: None,
            object_class: None,
        }
    }

    fn entity_subclass(class: &str, subclass: &str) -> EntitySubclass {
        EntitySubclass {
            class: class.to_string(),
            subclass: subclass.to_string(),
            source: PromotionSource::Corpus,
            count: 1,
            first_seen_doc_id: None,
            example_value: None,
        }
    }

    // === Task 1 tests ===

    #[test]
    fn test_default_store_has_seed_predicates_only() {
        let store = OntologyStore::default();
        let names = store.effective_predicate_names();
        assert_eq!(names.len(), 21, "default store must have exactly 21 seed predicates");
        assert!(names.contains(&"owns".to_string()));
    }

    #[test]
    fn test_load_missing_file_returns_default() {
        let dir = TempDir::new().unwrap();
        let store = OntologyStore::load(dir.path());
        assert!(!store.bootstrap_completed());
        assert_eq!(store.effective_predicate_names().len(), 21);
    }

    #[test]
    fn test_load_corrupt_file_returns_default() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("ontology.json"), "not json").unwrap();
        let store = OntologyStore::load(dir.path());
        assert!(!store.bootstrap_completed());
        assert_eq!(store.effective_predicate_names().len(), 21);
    }

    #[test]
    fn test_load_round_trip() {
        let dir = TempDir::new().unwrap();
        let mut store = OntologyStore::default();
        store.register_manual_predicate(
            "custody_of".to_string(),
            "custody relation".to_string(),
            None,
            None,
            "2026-07-10T00:00:00Z",
        );
        store.save(dir.path()).unwrap();

        let loaded = OntologyStore::load(dir.path());
        assert_eq!(loaded.schema.manual_predicates.len(), 1);
        assert_eq!(loaded.schema.manual_predicates[0].name, "custody_of");
        assert_eq!(
            loaded.schema.version, store.schema.version,
            "schema must round-trip equal"
        );
    }

    #[test]
    fn test_effective_predicates_dedup_across_sources() {
        let mut store = OntologyStore::default();
        store.schema.corpus_seed = Some(BootstrapSeed {
            predicates: vec![predicate("owns", PromotionSource::Corpus, 1)],
            entity_subclasses: vec![],
            generated_at: "2026-07-10T00:00:00Z".to_string(),
            sample_doc_count: 30,
            model_used: "test".to_string(),
        });

        let effective = store.effective_predicates();
        let owns_entries: Vec<&Predicate> = effective.iter().filter(|p| p.name == "owns").collect();
        assert_eq!(owns_entries.len(), 1, "owns must appear exactly once");
        assert_eq!(owns_entries[0].source, PromotionSource::Seed);
    }

    #[test]
    fn test_effective_predicates_preserves_insertion_order() {
        let mut store = OntologyStore::default();
        store.schema.corpus_seed = Some(BootstrapSeed {
            predicates: vec![predicate("car_registered_to", PromotionSource::Corpus, 1)],
            entity_subclasses: vec![],
            generated_at: "2026-07-10T00:00:00Z".to_string(),
            sample_doc_count: 30,
            model_used: "test".to_string(),
        });
        store
            .schema
            .adaptive_predicates
            .push(predicate("neighbor_of", PromotionSource::Adaptive, 2));
        store
            .schema
            .manual_predicates
            .push(predicate("custody_of", PromotionSource::Manual, 0));

        let names = store.effective_predicates();
        let tail: Vec<&str> = names[21..].iter().map(|p| p.name.as_str()).collect();
        assert_eq!(
            tail,
            vec!["car_registered_to", "custody_of", "neighbor_of"],
            "order must be corpus, then manual, then adaptive"
        );
    }

    #[test]
    fn test_class_to_subclasses_index_built_on_load() {
        let dir = TempDir::new().unwrap();
        let mut store = OntologyStore::default();
        store.schema.entity_subclasses = vec![
            entity_subclass("Location", "apartment"),
            entity_subclass("Location", "plot"),
            entity_subclass("Person", "spouse"),
        ];
        store.save(dir.path()).unwrap();

        let loaded = OntologyStore::load(dir.path());
        assert_eq!(
            loaded.class_to_subclasses.get("Location").map(|s| s.len()),
            Some(2)
        );
        assert_eq!(
            loaded.class_to_subclasses.get("Person").map(|s| s.len()),
            Some(1)
        );
    }

    // === Task 2 tests ===

    #[test]
    fn test_record_pending_predicate_first_occurrence_stays_pending() {
        let mut store = OntologyStore::default();
        let result = store.record_pending_predicate(
            "neighbor_of",
            "neighboring relation",
            None,
            None,
            "doc-1",
            "2026-07-10T00:00:00Z",
        );
        assert_eq!(result, PromoteResult::StillPending { count: 1 });
        assert_eq!(store.schema.pending_predicates.len(), 1);
        assert_eq!(store.schema.adaptive_predicates.len(), 0);
    }

    #[test]
    fn test_record_pending_predicate_second_occurrence_promotes() {
        let mut store = OntologyStore::default();
        store.record_pending_predicate(
            "neighbor_of",
            "neighboring relation",
            None,
            None,
            "doc-1",
            "2026-07-10T00:00:00Z",
        );
        let result = store.record_pending_predicate(
            "neighbor_of",
            "neighboring relation",
            None,
            None,
            "doc-2",
            "2026-07-10T01:00:00Z",
        );
        assert_eq!(result, PromoteResult::Promoted);
        assert_eq!(store.schema.pending_predicates.len(), 0);
        assert_eq!(store.schema.adaptive_predicates.len(), 1);
        assert_eq!(store.schema.adaptive_predicates[0].name, "neighbor_of");
        assert!(store.effective_predicate_names().contains(&"neighbor_of".to_string()));
    }

    #[test]
    fn test_record_pending_predicate_already_present_returns_alreadypresent() {
        let mut store = OntologyStore::default();
        let result = store.record_pending_predicate(
            "owns",
            "already a seed predicate",
            None,
            None,
            "doc-1",
            "2026-07-10T00:00:00Z",
        );
        assert_eq!(result, PromoteResult::AlreadyPresent);
    }

    #[test]
    fn test_promote_respects_vocabulary_hard_cap() {
        let mut store = OntologyStore::default();
        // Fill effective vocab to the cap via manual predicates.
        // Cap is 200; 21 are seed, so add 179 manual predicates to reach 200.
        for i in 0..(VOCABULARY_HARD_CAP - 21) {
            let name = format!("manual_pred_{i}");
            let result = store.register_manual_predicate(
                name,
                "filler".to_string(),
                None,
                None,
                "2026-07-10T00:00:00Z",
            );
            assert_eq!(result, PromoteResult::Promoted);
        }
        assert_eq!(store.effective_predicate_names().len(), VOCABULARY_HARD_CAP);

        // Now attempt to promote a pending predicate — cap is already hit.
        store.record_pending_predicate(
            "over_cap_pred",
            "should not fit",
            None,
            None,
            "doc-1",
            "2026-07-10T00:00:00Z",
        );
        let result = store.record_pending_predicate(
            "over_cap_pred",
            "should not fit",
            None,
            None,
            "doc-2",
            "2026-07-10T01:00:00Z",
        );
        assert_eq!(result, PromoteResult::CapExceeded);
        // Still parked in pending, not lost.
        assert!(store
            .schema
            .pending_predicates
            .iter()
            .any(|p| p.name == "over_cap_pred"));
    }

    #[test]
    fn test_apply_bootstrap_idempotent() {
        let mut store = OntologyStore::default();
        let seed = BootstrapSeed {
            predicates: vec![predicate("registered_to", PromotionSource::Corpus, 1)],
            entity_subclasses: vec![entity_subclass("Location", "apartment")],
            generated_at: "2026-07-10T00:00:00Z".to_string(),
            sample_doc_count: 30,
            model_used: "test".to_string(),
        };
        store.apply_bootstrap(seed.clone(), "2026-07-10T00:00:00Z");
        let len_after_first = store.effective_predicate_names().len();

        store.apply_bootstrap(seed, "2026-07-10T02:00:00Z");
        let len_after_second = store.effective_predicate_names().len();

        assert_eq!(len_after_first, len_after_second, "second apply_bootstrap must be a no-op");
        assert_eq!(
            store.schema.bootstrap_completed_at,
            Some("2026-07-10T00:00:00Z".to_string()),
            "bootstrap_completed_at must not change on idempotent no-op"
        );
    }

    #[test]
    fn test_apply_bootstrap_skips_seed_dup() {
        let mut store = OntologyStore::default();
        let seed = BootstrapSeed {
            predicates: vec![predicate("owns", PromotionSource::Corpus, 1)],
            entity_subclasses: vec![],
            generated_at: "2026-07-10T00:00:00Z".to_string(),
            sample_doc_count: 30,
            model_used: "test".to_string(),
        };
        store.apply_bootstrap(seed, "2026-07-10T00:00:00Z");

        let effective = store.effective_predicates();
        let owns_entries: Vec<&Predicate> = effective.iter().filter(|p| p.name == "owns").collect();
        assert_eq!(owns_entries.len(), 1);
        assert_eq!(owns_entries[0].source, PromotionSource::Seed);
    }

    #[test]
    fn test_register_manual_predicate_snake_case_validation() {
        let mut store = OntologyStore::default();
        assert_eq!(
            store.register_manual_predicate(
                "CamelCase".to_string(),
                "d".to_string(),
                None,
                None,
                "now"
            ),
            PromoteResult::CapExceeded
        );
        assert_eq!(
            store.register_manual_predicate(
                "with-dash".to_string(),
                "d".to_string(),
                None,
                None,
                "now"
            ),
            PromoteResult::CapExceeded
        );
        assert_eq!(
            store.register_manual_predicate("".to_string(), "d".to_string(), None, None, "now"),
            PromoteResult::CapExceeded
        );
        assert_eq!(
            store.register_manual_predicate(
                "custody_of".to_string(),
                "d".to_string(),
                None,
                None,
                "now"
            ),
            PromoteResult::Promoted
        );
    }

    #[test]
    fn test_rename_predicate_rejects_seed() {
        let mut store = OntologyStore::default();
        let result = store.rename_predicate("owns", "owning", "now");
        assert!(matches!(result, Err(AppError::Invalid(_))));
    }

    #[test]
    fn test_rename_predicate_adaptive_updates_index() {
        let mut store = OntologyStore::default();
        store.register_manual_predicate(
            "neighbor_of".to_string(),
            "d".to_string(),
            None,
            None,
            "now",
        );
        store.rename_predicate("neighbor_of", "neighbor", "now2").unwrap();

        assert!(!store.name_to_source.contains_key("neighbor_of"));
        assert!(store.name_to_source.contains_key("neighbor"));
        assert!(store.effective_predicate_names().contains(&"neighbor".to_string()));
        assert!(!store.effective_predicate_names().contains(&"neighbor_of".to_string()));
    }

    #[test]
    fn test_merge_predicates_rejects_seed_in_from_list() {
        let mut store = OntologyStore::default();
        store.register_manual_predicate(
            "belongs_to".to_string(),
            "d".to_string(),
            None,
            None,
            "now",
        );
        let result = store.merge_predicates(vec!["owns".to_string()], "belongs_to".to_string(), "now");
        assert!(matches!(result, Err(AppError::Invalid(_))));
    }

    #[test]
    fn test_reset_to_seed_clears_bootstrap_flag() {
        let mut store = OntologyStore::default();
        store.apply_bootstrap(
            BootstrapSeed {
                predicates: vec![predicate("registered_to", PromotionSource::Corpus, 1)],
                entity_subclasses: vec![],
                generated_at: "now".to_string(),
                sample_doc_count: 30,
                model_used: "test".to_string(),
            },
            "2026-07-10T00:00:00Z",
        );
        assert!(store.bootstrap_completed());

        store.reset_to_seed("2026-07-10T01:00:00Z");
        assert!(!store.bootstrap_completed());
        assert_eq!(store.effective_predicate_names().len(), 21);
    }

    #[test]
    fn test_apply_consolidation_rename_returns_rewrite_instruction() {
        let mut store = OntologyStore::default();
        store.register_manual_predicate(
            "neighbor_of".to_string(),
            "d".to_string(),
            None,
            None,
            "now",
        );
        store.set_pending_consolidation(Some(PendingConsolidation {
            suggestions: vec![crate::types::ConsolidationSuggestion {
                id: "sugg-1".to_string(),
                kind: ConsolidationKind::Rename {
                    from: "neighbor_of".to_string(),
                    to: "neighbor".to_string(),
                },
                rationale: "clearer name".to_string(),
                confidence: 0.9,
            }],
            generated_at: "now".to_string(),
            model_used: "test".to_string(),
            triple_count_at_generation: 500,
        }));

        let instructions = store.apply_consolidation("sugg-1", "2026-07-10T02:00:00Z").unwrap();
        assert_eq!(instructions.len(), 1);
        assert_eq!(instructions[0].kind, TripleRewriteKind::Rename);
        assert_eq!(instructions[0].from, vec!["neighbor_of".to_string()]);
        assert_eq!(instructions[0].to, "neighbor");
    }

    #[test]
    fn test_apply_consolidation_removes_applied_suggestion() {
        let mut store = OntologyStore::default();
        store.register_manual_predicate(
            "neighbor_of".to_string(),
            "d".to_string(),
            None,
            None,
            "now",
        );
        store.set_pending_consolidation(Some(PendingConsolidation {
            suggestions: vec![crate::types::ConsolidationSuggestion {
                id: "sugg-1".to_string(),
                kind: ConsolidationKind::Rename {
                    from: "neighbor_of".to_string(),
                    to: "neighbor".to_string(),
                },
                rationale: "clearer name".to_string(),
                confidence: 0.9,
            }],
            generated_at: "now".to_string(),
            model_used: "test".to_string(),
            triple_count_at_generation: 500,
        }));

        store.apply_consolidation("sugg-1", "2026-07-10T02:00:00Z").unwrap();
        assert!(
            store.pending_consolidation().is_none(),
            "pending_consolidation must clear once its only suggestion is applied"
        );
    }
}
