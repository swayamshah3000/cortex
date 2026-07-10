use regex::Regex;
use crate::types::ExtractedEntity;

/// Regex-based entity extractor for dates, amounts, emails, and person/org names.
pub struct EntityExtractor {
    date_re: Regex,
    amount_re: Regex,
    email_re: Regex,
    person_re: Regex,
}

impl EntityExtractor {
    /// Construct a new EntityExtractor, compiling all regex patterns once.
    pub fn new() -> Self {
        let date_re = Regex::new(
            r"\b(\d{4}-\d{2}-\d{2}|\d{1,2}/\d{1,2}/\d{2,4}|(?:Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)[a-z]*\.?\s+\d{1,2},?\s+\d{4})\b"
        ).expect("date regex is valid");

        let amount_re = Regex::new(
            r"(?:USD\s*)?[$\u{00a3}\u{20ac}]\s*[\d,]+(?:\.\d{2})?|[\d,]+(?:\.\d{2})?\s*(?:USD|EUR|GBP)"
        ).expect("amount regex is valid");

        let email_re = Regex::new(
            r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}\b"
        ).expect("email regex is valid");

        let person_re = Regex::new(
            r"\b([A-Z][a-z]+\s+[A-Z][a-z]+)\b"
        ).expect("person regex is valid");

        Self { date_re, amount_re, email_re, person_re }
    }

    /// Extract entities from text using regex patterns only.
    /// Results are sorted by (entity_type, value), deduplicated by (value, entity_type), and capped at 20.
    pub fn extract(&self, text: &str) -> Vec<ExtractedEntity> {
        let entities = self.extract_regex_entities(text);
        sort_dedup_cap(entities)
    }

    /// Collect raw regex entities (without sort/dedup/cap).
    fn extract_regex_entities(&self, text: &str) -> Vec<ExtractedEntity> {
        let mut entities: Vec<ExtractedEntity> = Vec::new();

        for cap in self.date_re.captures_iter(text) {
            entities.push(ExtractedEntity {
                label: "Date".to_string(),
                value: cap[0].to_string(),
                entity_type: "date".to_string(),
                canonical_id: None,
                ..Default::default()
            });
        }

        for cap in self.amount_re.find_iter(text) {
            entities.push(ExtractedEntity {
                label: "Amount".to_string(),
                value: cap.as_str().to_string(),
                entity_type: "amount".to_string(),
                canonical_id: None,
                ..Default::default()
            });
        }

        for cap in self.email_re.find_iter(text) {
            entities.push(ExtractedEntity {
                label: "Email".to_string(),
                value: cap.as_str().to_string(),
                entity_type: "email".to_string(),   // Fixed: was "person" (bug per PATTERNS.md line 58)
                canonical_id: None,
                ..Default::default()
            });
        }

        for cap in self.person_re.captures_iter(text) {
            entities.push(ExtractedEntity {
                label: "Person/Org".to_string(),
                value: cap[1].to_string(),
                entity_type: "person".to_string(),
                canonical_id: None,
                ..Default::default()
            });
        }

        entities
    }

}

/// Sort entities by (entity_type, value), deduplicate by (value, entity_type), then cap at 20.
///
/// Dedup is by (value, entity_type) pair per CONTEXT D-02 — same string with different
/// types remains distinct. e.g., "2024-03-15" as "date" and "text" are BOTH kept,
/// but "John Smith" + "John Smith" both as "person" → only one survives.
fn sort_dedup_cap(mut entities: Vec<ExtractedEntity>) -> Vec<ExtractedEntity> {
    // Sort by (entity_type, value) so dedup_by sees adjacent pairs grouped correctly
    entities.sort_by(|a, b| {
        a.entity_type.cmp(&b.entity_type).then(a.value.cmp(&b.value))
    });
    // Dedup by (value, entity_type) — note: dedup_by treats consecutive elements
    // After sort by (entity_type, value), equal (entity_type, value) pairs are adjacent
    entities.dedup_by(|a, b| a.value == b.value && a.entity_type == b.entity_type);
    // Cap at 20 entities
    entities.truncate(20);
    entities
}

impl Default for EntityExtractor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn extractor() -> EntityExtractor {
        EntityExtractor::new()
    }

    #[test]
    fn test_extract_iso_date() {
        let e = extractor();
        let results = e.extract("Meeting on 2024-03-15 to discuss Q2.");
        let dates: Vec<_> = results.iter().filter(|e| e.entity_type == "date").collect();
        assert!(!dates.is_empty(), "should find at least one date");
        assert!(dates.iter().any(|d| d.value == "2024-03-15"));
    }

    #[test]
    fn test_extract_us_date() {
        let e = extractor();
        let results = e.extract("Due 3/15/2024 please submit.");
        let dates: Vec<_> = results.iter().filter(|e| e.entity_type == "date").collect();
        assert!(!dates.is_empty());
        assert!(dates.iter().any(|d| d.value == "3/15/2024"));
    }

    #[test]
    fn test_extract_written_date() {
        let e = extractor();
        let results = e.extract("Invoice dated January 15, 2024.");
        let dates: Vec<_> = results.iter().filter(|e| e.entity_type == "date").collect();
        assert!(!dates.is_empty(), "should find written date");
    }

    #[test]
    fn test_extract_dollar_amount() {
        let e = extractor();
        let results = e.extract("Total: $1,234.56 due by end of month.");
        let amounts: Vec<_> = results.iter().filter(|e| e.entity_type == "amount").collect();
        assert!(!amounts.is_empty());
        assert!(amounts.iter().any(|a| a.value.contains("1,234.56")));
    }

    #[test]
    fn test_extract_person_name() {
        let e = extractor();
        // Use text where John Smith is not preceded by another capitalized word
        let results = e.extract("Please reach out to John Smith regarding the invoice.");
        let persons: Vec<_> = results.iter().filter(|e| e.entity_type == "person").collect();
        assert!(!persons.is_empty());
        assert!(persons.iter().any(|p| p.value == "John Smith"), "got: {:?}", persons);
    }

    #[test]
    fn test_deduplication() {
        let e = extractor();
        let results = e.extract("John Smith contacted John Smith again.");
        let persons: Vec<_> = results.iter().filter(|p| p.value == "John Smith").collect();
        assert_eq!(persons.len(), 1, "duplicate values should be removed");
    }

    #[test]
    fn test_truncation_at_20() {
        let e = extractor();
        // Generate text with many distinct amounts
        let text: String = (1..=30)
            .map(|i| format!("${}.00 ", i * 100))
            .collect();
        let results = e.extract(&text);
        assert!(results.len() <= 20, "should cap at 20 entities, got {}", results.len());
    }

    #[test]
    fn test_empty_text_returns_empty() {
        let e = extractor();
        let results = e.extract("");
        assert!(results.is_empty());
    }

    #[test]
    fn test_no_false_positives_on_plain_text() {
        let e = extractor();
        let results = e.extract("the quick brown fox jumps over the lazy dog");
        let dates: Vec<_> = results.iter().filter(|e| e.entity_type == "date").collect();
        let amounts: Vec<_> = results.iter().filter(|e| e.entity_type == "amount").collect();
        assert!(dates.is_empty());
        assert!(amounts.is_empty());
    }

    // === New tests for Task 2 behaviors ===

    /// Test 1: Email regex emits entity_type == "email" (NOT "person").
    /// Regression test for the existing bug per PATTERNS.md line 58.
    #[test]
    fn test_email_entity_type_is_email_not_person() {
        let e = extractor();
        let results = e.extract("Contact us at admin@example.com for support.");
        let emails: Vec<_> = results.iter().filter(|e| e.value.contains('@')).collect();
        assert!(!emails.is_empty(), "Expected at least one email entity");
        for email in &emails {
            assert_eq!(
                email.entity_type, "email",
                "Email entity should have type 'email', not 'person': {:?}", email
            );
        }
        // Regression: confirm "person" is NOT set for email addresses
        let wrong_type: Vec<_> = results.iter()
            .filter(|e| e.value.contains('@') && e.entity_type == "person")
            .collect();
        assert!(wrong_type.is_empty(), "No email should have entity_type='person': {:?}", wrong_type);
    }

    /// Test 3: Dedup is by (value, entity_type) pair — not just value.
    /// Same value with different types => both survive.
    /// Same value + same type => only one survives.
    #[test]
    fn test_dedup_by_value_and_type_pair() {
        // Same value "2024-03-15" but different entity_types: both should survive
        let entities_mixed_types = vec![
            ExtractedEntity { label: "d".to_string(), value: "2024-03-15".to_string(), entity_type: "date".to_string(), canonical_id: None, ..Default::default() },
            ExtractedEntity { label: "t".to_string(), value: "2024-03-15".to_string(), entity_type: "text".to_string(), canonical_id: None, ..Default::default() },
        ];
        let result = sort_dedup_cap(entities_mixed_types);
        assert_eq!(result.len(), 2,
            "Same value with different entity_types should both survive: {:?}", result);

        // Same value "John Smith" + same type "person": only one should survive
        let entities_same_type = vec![
            ExtractedEntity { label: "p".to_string(), value: "John Smith".to_string(), entity_type: "person".to_string(), canonical_id: None, ..Default::default() },
            ExtractedEntity { label: "p".to_string(), value: "John Smith".to_string(), entity_type: "person".to_string(), canonical_id: None, ..Default::default() },
        ];
        let result2 = sort_dedup_cap(entities_same_type);
        assert_eq!(result2.len(), 1,
            "Same value + same entity_type should produce one entity: {:?}", result2);
    }

}
