//! Entity Normalizer — Phase 11.6, Plan 03 (D-09..D-11)
//!
//! Rule-based canonical_short_name derivation. Runs after Pass 2 merge, before
//! EntityStore registration. Deterministic + fast — sub-microsecond per entity.
//! Never fails.
//!
//! Purpose: verbose LLM-extracted entity values (e.g. "Alpha Beta Corp
//! Complex-Unit-204") get a shorter chip-friendly form ("Unit 204") for
//! sidebar prominence, entity chips, and Ontology settings display. When no
//! rule applies, `canonical_short_name` stays `None` and callers fall back to
//! the raw `value`.

use std::sync::OnceLock;

use regex::Regex;

use crate::types::ExtractedEntity;

// ─── Rule constants ────────────────────────────────────────────────────────────

/// Common corporate suffixes stripped from Organization entity values
/// (case-insensitive). Iterating all of these lets "Foo Inc. Ltd" collapse to
/// "Foo" via repeated stripping.
pub const CORPORATE_SUFFIXES: &[&str] = &[
    " Ltd",
    " Ltd.",
    " Limited",
    " Inc",
    " Inc.",
    " Corp",
    " Corp.",
    " Corporation",
    " LLC",
    " L.L.C.",
    " LLP",
    " L.L.P.",
    " GmbH",
    " AG",
    " Pvt Ltd",
    " Pvt. Ltd.",
    " Private Limited",
    " Co",
    " Co.",
    " Company",
    " Sons",
    " & Sons",
];

/// Matches a "unit number" token: 1-6 letters optionally followed by a dash or
/// space, then one or more digits. Examples: "Unit 204", "P705", "T-12",
/// "A-1". Anchored `^...$` with bounded quantifiers — no catastrophic
/// backtracking risk (T-11.6-09 mitigation).
pub const HYPHEN_UNIT_REGEX: &str = r"^[A-Za-z]{1,6}[- ]?\d+$";

/// Lazily-compiled `HYPHEN_UNIT_REGEX`. Uses `OnceLock` (stable since Rust
/// 1.70, compatible with this crate's `rust-version = 1.77.2`) rather than
/// `once_cell`/`LazyLock` (the latter requires Rust 1.80+) to avoid adding a
/// new dependency or raising the MSRV.
fn hyphen_unit_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(HYPHEN_UNIT_REGEX).expect("HYPHEN_UNIT_REGEX must compile"))
}

// ─── Helpers ────────────────────────────────────────────────────────────────────

/// Strip a trailing corporate suffix from an organization name
/// (case-insensitive). Tries every entry in `CORPORATE_SUFFIXES` and picks
/// the LONGEST matching suffix so compound suffixes like " Pvt Ltd" are
/// stripped as a single unit rather than being chipped away one word at a
/// time (e.g. "Foo Pvt Ltd" → "Foo", not "Foo Pvt" via a first " Ltd" match
/// followed by a second, unsupported " Pvt" strip).
/// Returns `None` when no suffix matched (caller decides fallback).
fn strip_corporate_suffixes(value: &str) -> Option<String> {
    let value_lower = value.to_ascii_lowercase();

    let best_match = CORPORATE_SUFFIXES
        .iter()
        .filter(|suffix| {
            value_lower.ends_with(suffix.to_ascii_lowercase().as_str()) && value.len() >= suffix.len()
        })
        .max_by_key(|suffix| suffix.len())?;

    let cut = value.len() - best_match.len();
    let stripped = value[..cut].trim_end_matches([',', '.', ' ']).to_string();

    if stripped.is_empty() {
        None
    } else {
        Some(stripped)
    }
}

/// Split on hyphens ONLY (not spaces — see below); return a unit-number
/// segment when found, per D-10: "if class=Location AND name contains
/// multiple hyphens/dashes, keep the last segment ONLY if it looks like a
/// unit number".
///
/// Splitting on '-' only (not spaces) is a deliberate deviation from an
/// earlier plan draft: splitting on spaces too would make "Downtown Riverside
/// P705" (0 hyphens, 3 space-segments) incorrectly match the last-segment
/// rule and collapse a legitimate full property name down to "P705". Per
/// 11.6-CONTEXT.md §specifics: "Rare special cases (Riverside Complex P705)
/// stay full." Requiring at least one literal hyphen restricts the rule to
/// clearly-compound property names like "Alpha Beta Complex-Unit-204".
///
/// Algorithm: split on '-', filter empty segments. Require >= 2 hyphen
/// segments (i.e. at least one hyphen present). Try the last segment alone;
/// if it doesn't match `HYPHEN_UNIT_REGEX`, try the last TWO segments joined
/// with a space (handles "...-Unit-204" where "2004" alone is digits-only
/// and fails the regex's required alphabetic prefix, but "Unit 204" matches).
/// Falls back to `None` when neither candidate matches.
fn last_hyphen_unit_segment(value: &str) -> Option<String> {
    let segments: Vec<&str> = value
        .split('-')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    if segments.len() < 2 {
        return None;
    }

    let re = hyphen_unit_regex();
    let n = segments.len();

    // Try last segment alone.
    let last = segments[n - 1];
    if re.is_match(last) {
        return Some(collapse_whitespace(last));
    }

    // Try last two segments joined with a space.
    if n >= 2 {
        let joined = format!("{} {}", segments[n - 2], segments[n - 1]);
        if re.is_match(&joined) {
            return Some(collapse_whitespace(&joined));
        }
    }

    None
}

/// Collapse consecutive whitespace to a single space and trim.
fn collapse_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

// ─── Main API ───────────────────────────────────────────────────────────────────

/// Compute `canonical_short_name` for one entity per D-10.
///
/// Rules (evaluated in order; first match wins):
///  1. If class=Organization AND a corporate suffix is present → strip it.
///  2. If class=Location AND value contains a hyphen AND the last
///     (or last-two-joined) segment matches the unit-number pattern →
///     return that segment.
///  3. Whitespace-only differences → collapse and return the normalized form
///     when it differs from `value`.
///  4. Otherwise → `None` (caller falls back to raw `value`).
///
/// Never returns an empty string. Never returns `Some(x)` where `x ==
/// entity.value` (idempotent short-name is not a rewrite — caller treats
/// `None` as "no change"). T-11.6-10 mitigation.
pub fn normalize_entity(entity: &ExtractedEntity) -> Option<String> {
    let value = entity.value.as_str();
    let class = entity.class.as_deref();

    let candidate = match class {
        Some("Organization") => strip_corporate_suffixes(value),
        Some("Location") => last_hyphen_unit_segment(value),
        _ => None,
    };

    let candidate = candidate.or_else(|| {
        let collapsed = collapse_whitespace(value);
        if collapsed != value && !collapsed.is_empty() {
            Some(collapsed)
        } else {
            None
        }
    });

    candidate.and_then(|c| {
        let trimmed = c.trim().to_string();
        if trimmed.is_empty() || trimmed == value {
            None
        } else {
            Some(trimmed)
        }
    })
}

/// Mutate a slice of entities in place, setting `canonical_short_name` per
/// `normalize_entity`. Safe to call multiple times (idempotent — repeated
/// calls produce identical output, since `normalize_entity` reads
/// `entity.value`/`entity.class`, not the previous `canonical_short_name`).
pub fn normalize_entities(entities: &mut [ExtractedEntity]) {
    for entity in entities.iter_mut() {
        entity.canonical_short_name = normalize_entity(entity);
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entity(class: Option<&str>, value: &str) -> ExtractedEntity {
        ExtractedEntity {
            label: value.to_string(),
            value: value.to_string(),
            entity_type: class.map(|c| c.to_lowercase()).unwrap_or_default(),
            canonical_id: None,
            class: class.map(|s| s.to_string()),
            subclass: None,
            canonical_short_name: None,
            confidence: None,
        }
    }

    #[test]
    fn test_organization_ltd_stripped() {
        let e = make_entity(Some("Organization"), "Acme Corp Ltd");
        assert_eq!(normalize_entity(&e), Some("Acme Corp".to_string()));
    }

    #[test]
    fn test_organization_multiple_suffixes() {
        let e = make_entity(Some("Organization"), "Foo Pvt Ltd");
        assert_eq!(normalize_entity(&e), Some("Foo".to_string()));
    }

    #[test]
    fn test_organization_no_suffix() {
        let e = make_entity(Some("Organization"), "John Roe");
        assert_eq!(normalize_entity(&e), None);
    }

    #[test]
    fn test_location_unit_segment_matches() {
        let e = make_entity(Some("Location"), "Alpha Beta Complex-Unit-204");
        assert_eq!(normalize_entity(&e), Some("Unit 204".to_string()));
    }

    #[test]
    fn test_location_last_segment_alone() {
        let e = make_entity(Some("Location"), "Building-P705");
        assert_eq!(normalize_entity(&e), Some("P705".to_string()));
    }

    #[test]
    fn test_location_letters_only_last_segment_falls_through() {
        let e = make_entity(Some("Location"), "Complex-Unit");
        assert_eq!(normalize_entity(&e), None);
    }

    #[test]
    fn test_no_hyphen_returns_none() {
        // "Riverside Complex P705" has 0 hyphens — must NOT collapse to "P705".
        let e = make_entity(Some("Location"), "Riverside Complex P705");
        assert_eq!(normalize_entity(&e), None);
    }

    #[test]
    fn test_multi_hyphen_last_letter_number_matches() {
        let e = make_entity(Some("Location"), "Alpha-Beta-Complex-Unit-204");
        assert_eq!(normalize_entity(&e), Some("Unit 204".to_string()));
    }

    #[test]
    fn test_person_returns_none() {
        let e = make_entity(Some("Person"), "Jane Q Doe");
        assert_eq!(normalize_entity(&e), None);
    }

    #[test]
    fn test_whitespace_collapse() {
        let e = make_entity(Some("Person"), "  Foo   Bar  ");
        assert_eq!(normalize_entity(&e), Some("Foo Bar".to_string()));
    }

    #[test]
    fn test_normalize_entities_bulk_mutates_in_place() {
        let mut entities = vec![
            make_entity(Some("Organization"), "Acme Corp Ltd"),
            make_entity(Some("Location"), "Building-P705"),
            make_entity(Some("Person"), "Jane Q Doe"),
        ];
        normalize_entities(&mut entities);
        assert_eq!(entities[0].canonical_short_name, Some("Acme Corp".to_string()));
        assert_eq!(entities[1].canonical_short_name, Some("P705".to_string()));
        assert_eq!(entities[2].canonical_short_name, None);
    }

    #[test]
    fn test_normalize_is_idempotent() {
        let mut entities = vec![
            make_entity(Some("Organization"), "Acme Corp Ltd"),
            make_entity(Some("Location"), "Alpha Beta Complex-Unit-204"),
        ];
        normalize_entities(&mut entities);
        let first_pass: Vec<Option<String>> =
            entities.iter().map(|e| e.canonical_short_name.clone()).collect();

        // Second call reads entity.value/class (unchanged) — not the previous
        // canonical_short_name — so output must be identical.
        normalize_entities(&mut entities);
        let second_pass: Vec<Option<String>> =
            entities.iter().map(|e| e.canonical_short_name.clone()).collect();

        assert_eq!(first_pass, second_pass, "normalize_entities must be idempotent");
    }

    #[test]
    fn test_no_class_returns_none_unless_whitespace() {
        let e = make_entity(None, "Some Value");
        assert_eq!(normalize_entity(&e), None);
    }

    #[test]
    fn test_never_returns_empty_string() {
        // A pathological "Ltd" alone should not strip to an empty string.
        let e = make_entity(Some("Organization"), "Ltd");
        let result = normalize_entity(&e);
        if let Some(s) = result {
            assert!(!s.is_empty(), "normalize_entity must never return Some(\"\")");
        }
    }
}
