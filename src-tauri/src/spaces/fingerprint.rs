//! Membership fingerprint and Jaccard distance for Space labeling cache.
//!
//! D-05: Fingerprint = SHA-256(sorted doc-id set). First 16 hex chars.
//! D-06: Shift threshold = 20% Jaccard distance from cached fingerprint.
//!       Formula: |added ∪ removed| / |union| > 0.20 → re-label.
//!
//! # Thread-safety
//! Both functions are pure (no global state). Safe to call from any thread.
//!
//! # Cache note (T-09-03)
//! SpaceLabelCache wraps this module's output in Arc<Mutex<>> in AppState
//! (Plan 04). The functions themselves have no I/O.

use sha2::{Digest, Sha256};
use std::collections::HashSet;

/// Compute a 16-char hex fingerprint from a cluster's document ID set.
///
/// Sorted before hashing so the result is order-independent: the same
/// set of doc-ids always produces the same fingerprint regardless of
/// the order they are supplied. A `\n` separator between IDs prevents
/// prefix-collisions (e.g., `"ab"+"c"` ≠ `"a"+"bc"`).
///
/// # Examples
///
/// ```ignore
/// let fp = membership_fingerprint(&["doc-1".to_string(), "doc-2".to_string()]);
/// assert_eq!(fp.len(), 16);
/// ```
pub fn membership_fingerprint(doc_ids: &[String]) -> String {
    let mut sorted: Vec<&String> = doc_ids.iter().collect();
    sorted.sort();
    let mut hasher = Sha256::new();
    for id in sorted {
        hasher.update(id.as_bytes());
        hasher.update(b"\n"); // separator prevents "ab"+"c" == "a"+"bc" collision
    }
    let result = hasher.finalize();
    // format!("{:x}") produces 64 hex chars; slice first 16 (D-05).
    format!("{:x}", result)[..16].to_string()
}

/// Jaccard distance between two document-id sets (D-06).
///
/// Returns `|added ∪ removed| / |union|` where:
/// - `added  = new_ids − old_ids`
/// - `removed = old_ids − new_ids`
/// - `union  = old_ids ∪ new_ids`
///
/// Returns `0.0` when both sets are empty (guards divide-by-zero).
/// A value **strictly greater than 0.20** triggers re-labeling.
pub fn jaccard_distance(old_ids: &HashSet<String>, new_ids: &HashSet<String>) -> f32 {
    let added = new_ids.difference(old_ids).count();
    let removed = old_ids.difference(new_ids).count();
    let union_size = old_ids.union(new_ids).count();
    if union_size == 0 {
        return 0.0;
    }
    (added + removed) as f32 / union_size as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sv(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    fn hs(v: &[&str]) -> HashSet<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    // --- membership_fingerprint ---

    #[test]
    fn test_fingerprint_order_independent() {
        // Different orderings of the same set → identical fingerprint (D-05).
        let a = membership_fingerprint(&sv(&["b", "a", "c"]));
        let b = membership_fingerprint(&sv(&["a", "b", "c"]));
        let c = membership_fingerprint(&sv(&["c", "b", "a"]));
        assert_eq!(a, b, "forward vs reversed order must match");
        assert_eq!(b, c, "different permutation must also match");
    }

    #[test]
    fn test_fingerprint_hex_length() {
        // Must be exactly 16 hex characters (D-05: first 16 hex chars of SHA-256).
        let fp = membership_fingerprint(&sv(&["doc-1", "doc-2", "doc-3"]));
        assert_eq!(fp.len(), 16, "fingerprint must be 16 hex chars, got {}", fp.len());
        // All chars are valid lowercase hex digits.
        assert!(fp.chars().all(|c| c.is_ascii_hexdigit()), "must be hex: {}", fp);
    }

    #[test]
    fn test_fingerprint_separator_no_collision() {
        // Without a separator "ab","c" → sorted "ab\nc\n" and "a","bc" → sorted "a\nbc\n"
        // These must differ — the \n separator enforces this.
        let a = membership_fingerprint(&sv(&["ab", "c"]));
        let b = membership_fingerprint(&sv(&["a", "bc"]));
        assert_ne!(a, b, "separator must prevent prefix collisions");
    }

    // --- jaccard_distance ---

    #[test]
    fn test_jaccard_self_distance() {
        // Identical sets → distance is 0 (no added, no removed, all in union).
        let set = hs(&["a", "b", "c"]);
        assert_eq!(
            jaccard_distance(&set, &set),
            0.0,
            "self-distance must be 0"
        );
    }

    #[test]
    fn test_jaccard_empty_union() {
        // Both sets empty → union size is 0; must return 0 without divide-by-zero.
        let empty: HashSet<String> = HashSet::new();
        assert_eq!(
            jaccard_distance(&empty, &empty),
            0.0,
            "empty union must return 0, not panic"
        );
    }

    #[test]
    fn test_jaccard_added_item_threshold() {
        // old = {a,b,c,d}, new = {a,b,c}: removed=1 (d), added=0, union=4 → 1/4 = 0.25.
        // 0.25 > 0.20 threshold → re-label should fire.
        let old = hs(&["a", "b", "c", "d"]);
        let new = hs(&["a", "b", "c"]);
        let dist = jaccard_distance(&old, &new);
        assert!(
            (dist - 0.25_f32).abs() < 1e-6,
            "expected 0.25, got {}",
            dist
        );
        assert!(dist > 0.20, "0.25 must be above the 0.20 re-label threshold");
    }

    #[test]
    fn test_jaccard_borderline() {
        // old = {a,b,c,d,e}, new = {a,b,c,d}: removed=1 (e), added=0, union=5 → 1/5 = 0.20.
        // Exactly AT threshold — NOT strictly above 0.20, so re-label does NOT fire.
        let old = hs(&["a", "b", "c", "d", "e"]);
        let new = hs(&["a", "b", "c", "d"]);
        let dist = jaccard_distance(&old, &new);
        assert!(
            (dist - 0.20_f32).abs() < 1e-6,
            "expected 0.20, got {}",
            dist
        );
        assert!(
            dist <= 0.20,
            "borderline 0.20 must NOT be strictly above threshold"
        );
    }

    #[test]
    fn test_jaccard_fully_disjoint() {
        // {a} vs {b}: added=1, removed=1, union=2 → 2/2 = 1.0 (maximum distance).
        let old = hs(&["a"]);
        let new = hs(&["b"]);
        assert_eq!(
            jaccard_distance(&old, &new),
            1.0,
            "fully disjoint sets must have distance 1.0"
        );
    }
}
