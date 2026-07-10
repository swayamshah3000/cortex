//! Pass 1 — Deterministic Pattern Extractor (Phase 8, Plan 02)
//!
//! Extracts entities from document text using regex patterns + checksum validators.
//! Runs on every document regardless of LLM availability (D-01, D-31).
//! Idempotent: same input → same `Vec<ExtractedEntity>`, sorted + deduped + capped at 20 (D-02, LLME-03).
//!
//! Entity classes emitted:
//!   Date, Email, Phone, Amount, Identifier (all in the 8-class locked schema).
//!   URL is emitted with `class = None` — it is NOT in the 8-class taxonomy (D-09).

use regex::Regex;
use crate::error::AppError;
use crate::types::ExtractedEntity;

// ─── GSTIN base-36 charset ────────────────────────────────────────────────────
/// Ordered chars for GSTIN Mod-36 checksum: ordinal of c = index in this array.
const GSTIN_CHARSET: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";

// ─── VIN transliteration table (NHTSA standard, A=idx 0) ─────────────────────
/// Maps letter A-Z (index = letter - 'A') to its VIN numeric value.
/// I (idx 8), O (idx 14), Q (idx 16) are never legal in a VIN — they map to 0 but
/// the VIN regex `[A-HJ-NPR-Z0-9]{17}` will never produce them.
const VIN_CHAR_VALUES: [u8; 26] = [
    1, 2, 3, 4, 5, 6, 7, 8, // A(0) B(1) C(2) D(3) E(4) F(5) G(6) H(7)
    0,                       // I(8)  — never in VIN
    1, 2, 3, 4, 5,           // J(9) K(10) L(11) M(12) N(13)
    0,                       // O(14) — never in VIN
    7,                       // P(15)
    0,                       // Q(16) — never in VIN
    9, 2, 3, 4, 5, 6, 7, 8, 9, // R(17) S(18) T(19) U(20) V(21) W(22) X(23) Y(24) Z(25)
];

/// NHTSA VIN position weights for indices 0-16 (17 characters total).
/// Position 8 (0-indexed, the check digit) has weight 0 so it does not contribute to the sum.
const VIN_WEIGHTS: [u32; 17] = [8, 7, 6, 5, 4, 3, 2, 10, 0, 9, 8, 7, 6, 5, 4, 3, 2];

/// Maximum number of entities returned (LLME-03).
const ENTITY_CAP: usize = 20;

// ─── Pass1PatternExtractor ────────────────────────────────────────────────────

/// Deterministic regex + checksum extractor for Phase 8 Pass 1.
///
/// Constructor compiles all regexes eagerly — if any pattern is malformed the
/// build fails at startup rather than at extraction time (fail-fast, T-08-06).
/// All patterns in the `regex` crate are linear-time (no ReDoS risk).
///
/// `extract()` is pure, synchronous, and suitable for `spawn_blocking` callers.
pub struct Pass1PatternExtractor {
    date_candidate_re: Regex,
    email_re:          Regex,
    phone_re:          Regex,
    amount_re:         Regex,
    url_re:            Regex,
    // Identifier candidate patterns (regex finds candidates; validator confirms)
    aadhaar_re:        Regex,
    iban_re:           Regex,
    credit_card_re:    Regex,
    pan_re:            Regex,
    ssn_re:            Regex,
    nino_re:           Regex,
    sin_re:            Regex,
    vin_re:            Regex,
    gstin_re:          Regex,
    weak_id_re:        Regex,
}

impl Pass1PatternExtractor {
    /// Compile all regex patterns eagerly.  Returns `AppError::Internal` on any bad pattern
    /// (should never happen in practice — patterns are static strings).
    pub fn new() -> Result<Self, AppError> {
        let make = |pattern: &str| -> Result<Regex, AppError> {
            Regex::new(pattern)
                .map_err(|e| AppError::Internal(format!("regex compile error: {}", e)))
        };

        Ok(Self {
            // ── Dates: broad candidates — dateparser validates each match ───────
            // Handles ISO-8601, RFC-3339, US slash, DD-MM-YYYY, written month forms.
            // Excludes bare digits and version numbers like "1.2.3".
            date_candidate_re: make(concat!(
                r"(?x)\b(?:",
                    r"\d{4}-\d{2}-\d{2}(?:T\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:\d{2})?)?", // ISO / RFC-3339
                    r"|\d{1,2}/\d{1,2}/\d{4}",                   // M/D/YYYY
                    r"|\d{1,2}-\d{1,2}-\d{4}",                   // D-M-YYYY (4-digit year)
                    r"|\d{1,2}\s+(?:Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)[a-z]*\.?\s+\d{4}", // 3 Jul 2026
                    r"|(?:Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)[a-z]*\.?\s+\d{1,2},?\s+\d{4}", // Jan 1, 2025
                    r"|(?:January|February|March|April|May|June|July|August|September|October|November|December)\s+\d{1,2},?\s+\d{4}", // January 15, 2024
                r")\b"
            ))?,

            // ── Email: RFC-5322 practical subset ─────────────────────────────────
            email_re: make(r"\b[A-Za-z0-9._%+\-]+@[A-Za-z0-9.\-]+\.[A-Za-z]{2,}\b")?,

            // ── Phone: E.164 and US national format ───────────────────────────────
            phone_re: make(concat!(
                r"(?:",
                    r"\+\d{1,3}[\s\-.]?\(?\d{1,4}\)?[\s\-.]?\d{3,5}[\s\-.]?\d{4,7}", // E.164
                    r"|\(?\d{3}\)?[\s\-\.]\d{3}[\s\-\.]\d{4}",   // US (555) 123-4567
                r")"
            ))?,

            // ── Amount: currency symbol (prefix) or ISO code (suffix) ─────────────
            // Named groups: sym + num1  OR  num2 + code
            amount_re: make(concat!(
                r"(?P<sym>[$\u{20B9}\u{20AC}\u{00A3}\u{00A5}])\s*(?P<num1>[\d,]+(?:\.\d{1,2})?)",
                r"|\b(?P<num2>[\d,]+(?:\.\d{1,2})?)\s*(?P<code>USD|INR|EUR|GBP|JPY)\b"
            ))?,

            // ── URL: http/https ───────────────────────────────────────────────────
            url_re: make(r"https?://[^\s<>]{3,}")?,

            // ── Aadhaar: 12 digits, optional spaces every 4 ──────────────────────
            aadhaar_re: make(r"\b\d{4}\s?\d{4}\s?\d{4}\b")?,

            // ── IBAN: country code + check + body (with optional spaces) ──────────
            iban_re: make(r"\b[A-Z]{2}\d{2}[A-Z0-9\s]{4,32}\b")?,

            // ── Credit card: 13-19 digits with optional space/dash separators ─────
            // Two sub-patterns: 16-digit (Visa/MC/Discover) and 15-digit (Amex).
            credit_card_re: make(concat!(
                r"\b(?:",
                    r"\d{4}[\s\-]?\d{4}[\s\-]?\d{4}[\s\-]?\d{1,4}", // 13-16 digits
                    r"|\d{4}[\s\-]?\d{6}[\s\-]?\d{5}",               // 15 digits (Amex)
                r")\b"
            ))?,

            // ── PAN: exactly AAAAA0000A (5 uppercase + 4 digits + 1 uppercase) ───
            pan_re: make(r"\b[A-Z]{5}[0-9]{4}[A-Z]\b")?,

            // ── SSN: NNN-NN-NNNN (strict dash format per D-03) ───────────────────
            ssn_re: make(r"\b\d{3}-\d{2}-\d{4}\b")?,

            // ── NINO: UK national insurance (two prefix letters + 6 digits + A-D) ─
            nino_re: make(r"\b[A-CEGHJ-PR-TW-Z]{2}\d{6}[A-D]\b")?,

            // ── SIN: Canada — 9 digits (optional space/dash separators) ──────────
            sin_re: make(r"\b\d{3}[\s\-]?\d{3}[\s\-]?\d{3}\b")?,

            // ── VIN: exactly 17 chars from VIN alphabet (no I, O, Q) ─────────────
            vin_re: make(r"\b[A-HJ-NPR-Z0-9]{17}\b")?,

            // ── GSTIN: 2-digit state + 5-char PAN base + 4 digits + alpha + alnum + Z + alnum ─
            gstin_re: make(r"\b\d{2}[A-Z]{5}\d{4}[A-Z][A-Z0-9]Z[A-Z0-9]\b")?,

            // ── Weak-format IDs: context-keyword + alphanumeric token (D-04) ─────
            weak_id_re: make(
                r"(?i)(?:policy|folio|account|receipt|invoice|plot)\s*(?:number|no\.?|#)?\s*[:.\-]?\s*([A-Z0-9][A-Z0-9\-]{3,19})"
            )?,
        })
    }

    /// Extract entities from `text`.
    ///
    /// Processing order: Date → Email → Phone → Amount → URL → Identifiers.
    /// Output is sorted by (entity_type, value), deduplicated by the same key,
    /// then capped at 20 entities (LLME-03).
    pub fn extract(&self, text: &str) -> Result<Vec<ExtractedEntity>, AppError> {
        let mut entities: Vec<ExtractedEntity> = Vec::new();

        self.extract_dates(text, &mut entities);
        self.extract_emails(text, &mut entities);
        self.extract_phones(text, &mut entities);
        self.extract_amounts(text, &mut entities);
        self.extract_urls(text, &mut entities);
        self.extract_identifiers(text, &mut entities);

        // Sort → dedup → cap (mirrors entities.rs `sort_dedup_cap` pattern)
        entities.sort_by(|a, b| {
            a.entity_type.cmp(&b.entity_type).then(a.value.cmp(&b.value))
        });
        entities.dedup_by(|a, b| a.entity_type == b.entity_type && a.value == b.value);
        entities.truncate(ENTITY_CAP);

        Ok(entities)
    }

    // ─── Private extraction helpers ───────────────────────────────────────────

    fn extract_dates(&self, text: &str, out: &mut Vec<ExtractedEntity>) {
        for m in self.date_candidate_re.find_iter(text) {
            let candidate = m.as_str().trim();
            // dateparser validates the format and returns DateTime<Utc>.
            // For date-only inputs (no explicit time), dateparser injects the current wall-clock
            // time, producing different RFC-3339 values on each parse.  Pitfall 3 (RESEARCH.md):
            // we normalise all dates to "YYYY-MM-DD" so that dedup by (entity_type, value) works
            // correctly and the output is deterministic (D-02, LLME-03).
            if let Ok(dt) = dateparser::parse(candidate) {
                // Normalize to ISO-8601 date (no time component) for stable values.
                let value = dt.format("%Y-%m-%d").to_string();
                out.push(ExtractedEntity {
                    label: candidate.to_string(),
                    value,
                    entity_type: "date".to_string(),
                    canonical_id: None,
                    class: Some("Date".to_string()),
                    subclass: None,
                    canonical_short_name: None,
                    confidence: Some(1.0),
                });
            }
        }
    }

    fn extract_emails(&self, text: &str, out: &mut Vec<ExtractedEntity>) {
        for m in self.email_re.find_iter(text) {
            let val = m.as_str().to_string();
            out.push(ExtractedEntity {
                label: val.clone(),
                value: val,
                entity_type: "email".to_string(),
                canonical_id: None,
                class: Some("Email".to_string()),
                subclass: None,
                canonical_short_name: None,
                confidence: Some(1.0),
            });
        }
    }

    fn extract_phones(&self, text: &str, out: &mut Vec<ExtractedEntity>) {
        for m in self.phone_re.find_iter(text) {
            let val = m.as_str().trim().to_string();
            if val.len() >= 7 {
                out.push(ExtractedEntity {
                    label: val.clone(),
                    value: val,
                    entity_type: "phone".to_string(),
                    canonical_id: None,
                    class: Some("Phone".to_string()),
                    subclass: None,
                    canonical_short_name: None,
                    confidence: Some(1.0),
                });
            }
        }
    }

    fn extract_amounts(&self, text: &str, out: &mut Vec<ExtractedEntity>) {
        for caps in self.amount_re.captures_iter(text) {
            let (value, currency) = if let (Some(sym), Some(num)) =
                (caps.name("sym"), caps.name("num1"))
            {
                (format!("{}{}", sym.as_str(), num.as_str()), currency_subclass(sym.as_str()))
            } else if let (Some(num), Some(code)) = (caps.name("num2"), caps.name("code")) {
                (format!("{} {}", num.as_str(), code.as_str()), currency_subclass(code.as_str()))
            } else {
                continue;
            };

            out.push(ExtractedEntity {
                label: value.clone(),
                value,
                entity_type: "amount".to_string(),
                canonical_id: None,
                class: Some("Amount".to_string()),
                subclass: currency,
                canonical_short_name: None,
                confidence: Some(1.0),
            });
        }
    }

    fn extract_urls(&self, text: &str, out: &mut Vec<ExtractedEntity>) {
        for m in self.url_re.find_iter(text) {
            let val = m.as_str().to_string();
            out.push(ExtractedEntity {
                label: val.clone(),
                value: val,
                entity_type: "url".to_string(),
                canonical_id: None,
                class: None,      // D-09: URL is NOT in the 8-class taxonomy
                subclass: Some("url".to_string()),
                canonical_short_name: None,
                confidence: Some(1.0),
            });
        }
    }

    fn extract_identifiers(&self, text: &str, out: &mut Vec<ExtractedEntity>) {
        // Aadhaar — 12 digits + Verhoeff checksum
        for m in self.aadhaar_re.find_iter(text) {
            let raw = m.as_str();
            let digits: String = raw.chars().filter(|c| c.is_ascii_digit()).collect();
            if digits.len() == 12 && validate_aadhaar(&digits) {
                out.push(make_id_entity(raw, "aadhaar", 1.0));
            }
        }

        // IBAN — Mod-97 validated
        for m in self.iban_re.find_iter(text) {
            let raw = m.as_str();
            if validate_iban(raw) {
                out.push(make_id_entity(raw.trim(), "iban", 1.0));
            }
        }

        // Credit card — Luhn + (BIN prefix OR context word)
        for m in self.credit_card_re.find_iter(text) {
            let raw = m.as_str();
            let digits: String = raw.chars().filter(|c| c.is_ascii_digit()).collect();
            // Context window: ±40 bytes around the match (D-03).
            // CR-02 fix: saturating_sub(40) can land on a non-char boundary
            // in multi-byte UTF-8 text (e.g. Hindi, German, Japanese).
            // Walk forward/backward to the nearest valid char boundary so
            // the slice operator does not panic.
            let start = {
                let candidate = m.start().saturating_sub(40);
                let mut s = candidate;
                while s < m.start() && !text.is_char_boundary(s) { s += 1; }
                s
            };
            let end = {
                let candidate = (m.end() + 40).min(text.len());
                let mut e = candidate;
                while e > m.end() && !text.is_char_boundary(e) { e -= 1; }
                e
            };
            let context = &text[start..end];
            if validate_credit_card(&digits, context) {
                out.push(make_id_entity(raw, "credit_card", 1.0));
            }
        }

        // PAN — strict format validated
        for m in self.pan_re.find_iter(text) {
            let raw = m.as_str();
            if validate_pan(raw) {
                out.push(make_id_entity(raw, "pan", 1.0));
            }
        }

        // SSN — area-code sanity checked
        for m in self.ssn_re.find_iter(text) {
            let raw = m.as_str();
            if validate_ssn(raw) {
                out.push(make_id_entity(raw, "ssn", 1.0));
            }
        }

        // NINO — format validated
        for m in self.nino_re.find_iter(text) {
            let raw = m.as_str();
            if validate_nino(raw) {
                out.push(make_id_entity(raw, "nino", 1.0));
            }
        }

        // SIN — 9 digits + Luhn
        for m in self.sin_re.find_iter(text) {
            let raw = m.as_str();
            let digits: String = raw.chars().filter(|c| c.is_ascii_digit()).collect();
            if digits.len() == 9 && validate_sin(&digits) {
                out.push(make_id_entity(raw, "sin", 1.0));
            }
        }

        // VIN — NHTSA weighted checksum
        for m in self.vin_re.find_iter(text) {
            let raw = m.as_str();
            if raw.len() == 17 && validate_vin(raw) {
                out.push(make_id_entity(raw, "vin", 1.0));
            }
        }

        // GSTIN — Mod-36 checksum
        for m in self.gstin_re.find_iter(text) {
            let raw = m.as_str();
            if raw.len() == 15 && validate_gstin(raw) {
                out.push(make_id_entity(raw, "gstin", 1.0));
            }
        }

        // Weak-format IDs — policy/folio/account/receipt/invoice/plot + token (D-04)
        for caps in self.weak_id_re.captures_iter(text) {
            if let Some(token) = caps.get(1) {
                let val = token.as_str().to_uppercase();
                out.push(ExtractedEntity {
                    label: val.clone(),
                    value: val,
                    entity_type: "identifier".to_string(),
                    canonical_id: None,
                    class: Some("Identifier".to_string()),
                    subclass: Some("unknown".to_string()),
                    canonical_short_name: None,
                    confidence: Some(0.7),
                });
            }
        }
    }
}

// ─── Helper constructors ──────────────────────────────────────────────────────

fn make_id_entity(raw: &str, subclass: &str, confidence: f32) -> ExtractedEntity {
    ExtractedEntity {
        label: raw.to_string(),
        value: raw.to_string(),
        entity_type: "identifier".to_string(),
        canonical_id: None,
        class: Some("Identifier".to_string()),
        subclass: Some(subclass.to_string()),
        canonical_short_name: None,
        confidence: Some(confidence),
    }
}

fn currency_subclass(s: &str) -> Option<String> {
    let sub = match s {
        "$" => "usd",
        "\u{20B9}" => "inr",  // ₹
        "\u{20AC}" => "eur",  // €
        "\u{00A3}" => "gbp",  // £
        "\u{00A5}" => "jpy",  // ¥
        other => {
            return match other.to_uppercase().as_str() {
                "USD" => Some("usd".to_string()),
                "INR" => Some("inr".to_string()),
                "EUR" => Some("eur".to_string()),
                "GBP" => Some("gbp".to_string()),
                "JPY" => Some("jpy".to_string()),
                _ => None,
            };
        }
    };
    Some(sub.to_string())
}

// ─── Identifier validators ────────────────────────────────────────────────────
// These are private functions — unit-tested in the test module below.
// Task 3 (TDD) replaces the stubs below with real implementations.

/// Aadhaar — 12 digits + Verhoeff checksum (D-03).
/// crate: `verhoeff 1.0.0` — `validate()` is the free function on the `Verhoeff` trait.
fn validate_aadhaar(digits: &str) -> bool {
    if digits.len() != 12 || !digits.chars().all(|c| c.is_ascii_digit()) {
        return false;
    }
    verhoeff::validate(digits)
}

/// IBAN — strips spaces then validates Mod-97 (ISO 13616) via `iban_validate 5.0.1` (D-03).
/// Note: the package is named `iban_validate` but its lib crate is named `iban`.
fn validate_iban(s: &str) -> bool {
    use iban::Iban;
    let normalized: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    if normalized.len() < 15 || normalized.len() > 34 {
        return false;
    }
    normalized.parse::<Iban>().is_ok()
}

/// Credit card — Luhn MUST pass, then BIN prefix valid OR context word present (D-03).
/// Pure-Luhn has ~10% false-positive rate; this gate is mandatory (T-08-05).
/// crate: `luhn 1.0.1`
fn validate_credit_card(digits: &str, context: &str) -> bool {
    let len = digits.len();
    if len < 13 || len > 19 { return false; }
    if !digits.chars().all(|c| c.is_ascii_digit()) { return false; }
    if !luhn::valid(digits) { return false; }
    has_valid_bin_prefix(digits) || has_card_context_word(context)
}

/// BIN prefix table: Visa, Mastercard (two ranges), Amex, Discover (three ranges).
fn has_valid_bin_prefix(digits: &str) -> bool {
    if digits.len() < 4 { return false; }
    let first1 = &digits[..1];
    let first2: u32 = digits[..2].parse().unwrap_or(0);
    let first4: u32 = digits[..4].parse().unwrap_or(0);
    let first6_opt: Option<u32> = if digits.len() >= 6 { digits[..6].parse().ok() } else { None };

    first1 == "4"                               // Visa
    || (51..=55).contains(&first2)              // Mastercard range 1
    || (2221..=2720).contains(&first4)          // Mastercard range 2
    || first2 == 34 || first2 == 37             // Amex
    || first4 == 6011                           // Discover
    || first2 == 65                             // Discover
    || first6_opt.map_or(false, |f6| (644000..=649999).contains(&f6)) // Discover 644-649
}

/// Context-word check within a ±40-char window around the candidate (D-03).
fn has_card_context_word(context: &str) -> bool {
    let lower = context.to_lowercase();
    lower.contains("card")
        || lower.contains("visa")
        || lower.contains("mastercard")
        || lower.contains("amex")
        || lower.contains("credit")
}

/// PAN (India) — exactly `[A-Z]{5}[0-9]{4}[A-Z]`, strict uppercase (D-03).
fn validate_pan(s: &str) -> bool {
    let s = s.trim();
    if s.len() != 10 { return false; }
    let bytes = s.as_bytes();
    bytes[..5].iter().all(|b| b.is_ascii_uppercase())
        && bytes[5..9].iter().all(|b| b.is_ascii_digit())
        && bytes[9].is_ascii_uppercase()
}

/// SSN (US) — `\d{3}-\d{2}-\d{4}` + full SSA validity rules (D-03):
/// area not 000/666/900-999, group not 00, serial not 0000.
///
/// WR-04 fix: prior implementation only rejected invalid area codes. SSA rules
/// also prohibit group number 00 ("123-00-6789") and serial number 0000
/// ("123-45-0000"). Omitting these checks widened the false-positive surface.
fn validate_ssn(s: &str) -> bool {
    let s = s.trim();
    if s.len() != 11 { return false; }
    let b = s.as_bytes();
    if b[3] != b'-' || b[6] != b'-' { return false; }
    if !b[..3].iter().all(|b| b.is_ascii_digit()) { return false; }
    if !b[4..6].iter().all(|b| b.is_ascii_digit()) { return false; }
    if !b[7..11].iter().all(|b| b.is_ascii_digit()) { return false; }
    let area: u32   = s[..3].parse().unwrap_or(0);
    let group: u32  = s[4..6].parse().unwrap_or(0);
    let serial: u32 = s[7..11].parse().unwrap_or(0);
    area != 0
        && area != 666
        && !(900..=999).contains(&area)
        && group != 0    // WR-04: group 00 is invalid per SSA rules
        && serial != 0   // WR-04: serial 0000 is invalid per SSA rules
}

/// NINO (UK) — two chars from `[A-CEGHJ-PR-TW-Z]`, six digits, one of `[A-D]` (D-03).
/// Excluded first/second chars: D, F, I, Q, U, V.
fn validate_nino(s: &str) -> bool {
    let s = s.trim();
    if s.len() != 9 { return false; }
    let upper = s.to_uppercase();
    let b = upper.as_bytes();
    let valid_prefix_char = |c: u8| {
        c.is_ascii_uppercase() && !matches!(c, b'D' | b'F' | b'I' | b'Q' | b'U' | b'V')
    };
    valid_prefix_char(b[0])
        && valid_prefix_char(b[1])
        && b[2..8].iter().all(|c| c.is_ascii_digit())
        && matches!(b[8], b'A' | b'B' | b'C' | b'D')
}

/// SIN (Canada) — 9 digits, Luhn-valid (D-03).
/// crate: `luhn 1.0.1`
fn validate_sin(digits: &str) -> bool {
    if digits.len() != 9 || !digits.chars().all(|c| c.is_ascii_digit()) {
        return false;
    }
    luhn::valid(digits)
}

/// VIN — 17 chars, NHTSA weighted checksum at position 8 (0-indexed) (D-03).
///
/// Source: <https://en.wikibooks.org/wiki/Vehicle_Identification_Numbers_(VIN_codes)/Check_digit_calculation>
/// Test vector: "1M8GDM9AXKP042788" → sum = 351, 351 mod 11 = 10 → check char = 'X' at index 8. ✓
fn validate_vin(s: &str) -> bool {
    let s = s.trim();
    if s.len() != 17 { return false; }
    let upper = s.to_uppercase();
    let bytes = upper.as_bytes();

    let mut sum: u32 = 0;
    for (i, &b) in bytes.iter().enumerate() {
        let val: u32 = if b.is_ascii_digit() {
            (b - b'0') as u32
        } else if b.is_ascii_uppercase() {
            let idx = (b - b'A') as usize;
            if idx >= VIN_CHAR_VALUES.len() { return false; }
            VIN_CHAR_VALUES[idx] as u32
        } else {
            return false;
        };
        sum += val * VIN_WEIGHTS[i];
    }

    let remainder = sum % 11;
    let check = bytes[8]; // position 8 (0-indexed) = 9th character (check digit position)
    if remainder == 10 {
        check == b'X'
    } else {
        check == b'0' + remainder as u8
    }
}

/// GSTIN (India) — 15-char format + Mod-36 Luhn-variant checksum (D-03, Open Question 1).
///
/// Checksum algorithm (alternate-position doubling in base-36):
///   For each of the first 14 characters (0-indexed):
///     factor = 2 if position is odd, 1 if even
///     product = char_ordinal × factor   (ordinal from GSTIN_CHARSET)
///     addend = (product / 36) + (product % 36)   — reduces two-digit base-36 products
///     sum += addend
///   check_value = (36 - sum%36) % 36
///   check_char  = GSTIN_CHARSET[check_value]
///
/// Test vectors (computed with this algorithm):
///   "27AABCU9603R1ZN" → total=193, check='N' (idx 23) ✓
///   "29GGGGG9999G1ZY" → total=254, check='Y' (idx 34) ✓
fn validate_gstin(s: &str) -> bool {
    let s = s.trim();
    if s.len() != 15 { return false; }
    let b = s.as_bytes();

    // Format: 2 digits + 5 uppercase + 4 digits + 1 uppercase + 1 alnum + Z + 1 alnum
    if !b[..2].iter().all(|c| c.is_ascii_digit()) { return false; }
    if !b[2..7].iter().all(|c| c.is_ascii_uppercase()) { return false; }
    if !b[7..11].iter().all(|c| c.is_ascii_digit()) { return false; }
    if !b[11].is_ascii_uppercase() { return false; }
    if !b[12].is_ascii_alphanumeric() { return false; }
    if b[13] != b'Z' { return false; }
    if !b[14].is_ascii_alphanumeric() { return false; }

    // Checksum
    let mut total: u32 = 0;
    for (i, &byte) in b[..14].iter().enumerate() {
        let ordinal = match GSTIN_CHARSET.iter().position(|&c| c == byte) {
            Some(pos) => pos as u32,
            None => return false,
        };
        let factor: u32 = if i % 2 == 1 { 2 } else { 1 };
        let product = ordinal * factor;
        total += (product / 36) + (product % 36);
    }

    let check_value = (36 - (total % 36)) % 36;
    b[14] == GSTIN_CHARSET[check_value as usize]
}

impl Default for Pass1PatternExtractor {
    fn default() -> Self {
        Self::new().expect("default Pass1PatternExtractor must succeed — all patterns are static")
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn extractor() -> Pass1PatternExtractor {
        Pass1PatternExtractor::new().expect("Pass1PatternExtractor::new() must succeed")
    }

    // ── Task 2: Regex-based extractor tests (no identifier validators) ────────

    #[test]
    fn test_extractor_new_succeeds() {
        // Constructor must compile all regexes without panic.
        let _ = extractor();
    }

    #[test]
    fn test_date_iso() {
        let e = extractor();
        let entities = e.extract("The document was created on 2024-01-15.").unwrap();
        let dates: Vec<_> = entities.iter().filter(|e| e.entity_type == "date").collect();
        assert!(!dates.is_empty(), "expected at least one date entity");
        // Value is normalized to YYYY-MM-DD (time is stripped for dedup stability — Pitfall 3)
        assert!(dates.iter().any(|d| d.value == "2024-01-15"));
        // Class must be "Date"
        assert!(dates.iter().all(|d| d.class.as_deref() == Some("Date")));
    }

    #[test]
    fn test_date_mm_dd_yyyy() {
        // dateparser defaults to US date ordering (MM/DD/YYYY — Pitfall 3, RESEARCH.md).
        // "15/07/1985" would be interpreted as month=15 (invalid) and fail parsing.
        // We use the unambiguous US form "07/15/1985" here; DD/MM locale support is deferred.
        let e = extractor();
        let entities = e.extract("Date of birth: 07/15/1985.").unwrap();
        let dates: Vec<_> = entities.iter().filter(|e| e.entity_type == "date").collect();
        assert!(!dates.is_empty(), "expected a date from 07/15/1985");
        assert!(dates.iter().any(|d| d.value.starts_with("1985-07-15")));
    }

    #[test]
    fn test_date_written_month() {
        let e = extractor();
        let entities = e.extract("Issue date: 3 Jul 2026.").unwrap();
        let dates: Vec<_> = entities.iter().filter(|e| e.entity_type == "date").collect();
        assert!(!dates.is_empty(), "expected written-month date '3 Jul 2026'");
    }

    #[test]
    fn test_date_jan_style() {
        let e = extractor();
        let entities = e.extract("Effective from Jan 1, 2025 until further notice.").unwrap();
        let dates: Vec<_> = entities.iter().filter(|e| e.entity_type == "date").collect();
        assert!(!dates.is_empty(), "expected 'Jan 1, 2025' to be extracted");
    }

    #[test]
    fn test_email_extract() {
        let e = extractor();
        let entities = e.extract("Send invoice to billing@example.com for approval.").unwrap();
        let emails: Vec<_> = entities.iter().filter(|e| e.entity_type == "email").collect();
        assert!(!emails.is_empty(), "expected at least one email");
        assert!(emails.iter().any(|m| m.value == "billing@example.com"));
        assert!(emails.iter().all(|m| m.class.as_deref() == Some("Email")));
    }

    #[test]
    fn test_phone_e164() {
        let e = extractor();
        let entities = e.extract("Call us at +1 555 123 4567 for support.").unwrap();
        let phones: Vec<_> = entities.iter().filter(|e| e.entity_type == "phone").collect();
        assert!(!phones.is_empty(), "expected phone entity from +1 555 123 4567");
        assert!(phones.iter().all(|p| p.class.as_deref() == Some("Phone")));
    }

    #[test]
    fn test_amount_dollar_subclass_usd() {
        let e = extractor();
        let entities = e.extract("Total amount due: $1,234.56.").unwrap();
        let amounts: Vec<_> = entities.iter().filter(|e| e.entity_type == "amount").collect();
        assert!(!amounts.is_empty(), "expected dollar amount");
        let dollar = amounts.iter().find(|a| a.value.contains("1,234.56")).unwrap();
        assert_eq!(dollar.subclass.as_deref(), Some("usd"), "$ should map to subclass 'usd'");
        assert_eq!(dollar.class.as_deref(), Some("Amount"));
    }

    #[test]
    fn test_amount_rupee_subclass_inr() {
        let e = extractor();
        let entities = e.extract("GST paid: \u{20B9}5,000.00 as per invoice.").unwrap();
        let amounts: Vec<_> = entities.iter().filter(|e| e.entity_type == "amount").collect();
        assert!(!amounts.is_empty(), "expected rupee amount");
        let rupee = amounts.iter().find(|a| a.value.contains("5,000")).unwrap();
        assert_eq!(rupee.subclass.as_deref(), Some("inr"), "\u{20B9} should map to subclass 'inr'");
    }

    #[test]
    fn test_amount_euro_subclass_eur() {
        let e = extractor();
        let entities = e.extract("Fee: \u{20AC}100.00 per month.").unwrap();
        let amounts: Vec<_> = entities.iter().filter(|e| e.entity_type == "amount").collect();
        assert!(!amounts.is_empty(), "expected euro amount");
        assert!(amounts.iter().any(|a| a.subclass.as_deref() == Some("eur")));
    }

    #[test]
    fn test_amount_iso_code_usd() {
        let e = extractor();
        let entities = e.extract("Remit 500 USD by end of month.").unwrap();
        let amounts: Vec<_> = entities.iter().filter(|e| e.entity_type == "amount").collect();
        assert!(!amounts.is_empty(), "expected amount from '500 USD'");
        assert!(amounts.iter().any(|a| a.subclass.as_deref() == Some("usd")));
    }

    #[test]
    fn test_url_extract_class_none() {
        let e = extractor();
        let entities = e.extract("See https://docs.example.com/api for details.").unwrap();
        let urls: Vec<_> = entities.iter().filter(|e| e.entity_type == "url").collect();
        assert!(!urls.is_empty(), "expected URL entity");
        // D-09: URL is NOT in the 8-class taxonomy; class must be None
        assert!(
            urls.iter().all(|u| u.class.is_none()),
            "URL entity must have class=None (not in 8-class taxonomy)"
        );
        assert!(urls.iter().any(|u| u.value.starts_with("https://")));
    }

    #[test]
    fn test_empty_text_returns_empty_vec() {
        let e = extractor();
        let entities = e.extract("").unwrap();
        assert!(entities.is_empty(), "empty text must produce empty Vec");
    }

    #[test]
    fn test_cap_at_20_emails() {
        let e = extractor();
        // Generate 30 distinct email addresses
        let text: String = (1..=30)
            .map(|i| format!("user{}@example.com ", i))
            .collect();
        let entities = e.extract(&text).unwrap();
        assert_eq!(
            entities.len(), 20,
            "output must be capped at 20 (LLME-03), got {}",
            entities.len()
        );
    }

    #[test]
    fn test_sort_dedup_dates() {
        let e = extractor();
        // Repeat the same date multiple times
        let text = "Due 2024-06-01. Reminder: 2024-06-01. Final: 2024-06-01.";
        let entities = e.extract(text).unwrap();
        let dates: Vec<_> = entities.iter().filter(|e| e.entity_type == "date").collect();
        // After dedup, only one entity for this date value
        assert_eq!(dates.len(), 1, "duplicate date values must be deduped, got {:?}", dates);
    }

    #[test]
    fn test_confidence_set_on_regex_entities() {
        let e = extractor();
        let entities = e.extract("Send to test@example.com the total $99.99 by 2024-12-01.").unwrap();
        // All regex-based entities (not identifier validators) must have confidence=Some(1.0)
        for entity in &entities {
            if entity.entity_type != "identifier" {
                assert_eq!(
                    entity.confidence,
                    Some(1.0),
                    "regex entity should have confidence=1.0: {:?}",
                    entity
                );
            }
        }
    }

    // ── Task 3 (TDD — RED phase): Identifier validator tests ─────────────────
    // These tests call the private validator functions directly.
    // In RED phase they document the expected behavior.
    // In GREEN phase all pass after real implementations replace stubs.

    // Aadhaar
    #[test]
    fn test_aadhaar_valid_checksum() {
        // "234123412346" is a documented Verhoeff-valid 12-digit sample.
        assert!(validate_aadhaar("234123412346"), "234123412346 must pass Verhoeff");
    }

    #[test]
    fn test_aadhaar_invalid_last_digit() {
        // Wrong check digit must fail.
        assert!(!validate_aadhaar("234123412340"), "234123412340 must fail Verhoeff (wrong check digit)");
    }

    #[test]
    fn test_aadhaar_wrong_length() {
        assert!(!validate_aadhaar("23412341234"), "11 digits must fail");
        assert!(!validate_aadhaar("2341234123456"), "13 digits must fail");
    }

    // IBAN
    #[test]
    fn test_iban_valid() {
        // Well-known IBAN test vector (NatWest UK account).
        assert!(validate_iban("GB29 NWBK 6016 1331 9268 19"), "known-good IBAN must pass");
        assert!(validate_iban("GB29NWBK60161331926819"), "no-space form must also pass");
    }

    #[test]
    fn test_iban_invalid_check_digit() {
        assert!(!validate_iban("GB29 NWBK 6016 1331 9268 20"), "wrong check digit must fail");
    }

    #[test]
    fn test_iban_too_short() {
        assert!(!validate_iban("GB29NWB"), "too short must fail");
    }

    // Credit card
    #[test]
    fn test_credit_card_valid_visa_bin() {
        // 4532015112830366 is Luhn-valid and starts with 4 (Visa BIN).
        assert!(
            validate_credit_card("4532015112830366", ""),
            "Luhn-valid Visa BIN number must pass even with empty context"
        );
    }

    #[test]
    fn test_credit_card_invalid_luhn() {
        // 4532015112830367 — last digit changed, Luhn fails.
        assert!(
            !validate_credit_card("4532015112830367", "visa card"),
            "Luhn-invalid number must fail even with context word"
        );
    }

    #[test]
    fn test_credit_card_luhn_valid_no_bin_no_context() {
        // 3800000000000006 — Luhn-valid but starts with 38 (not a standard BIN).
        // With no context word: must REJECT (D-03 gate).
        assert!(
            !validate_credit_card("3800000000000006", "no relevant words here"),
            "Luhn-valid unknown-BIN with no context must be rejected"
        );
    }

    #[test]
    fn test_credit_card_luhn_valid_context_word() {
        // Same number but context contains "credit card" → must ACCEPT.
        assert!(
            validate_credit_card("3800000000000006", "please use your credit card to pay"),
            "Luhn-valid unknown-BIN with 'credit card' context must be accepted"
        );
    }

    // PAN
    #[test]
    fn test_pan_valid() {
        assert!(validate_pan("ABCDE1234F"), "ABCDE1234F is a valid PAN format");
    }

    #[test]
    fn test_pan_invalid_no_trailing_letter() {
        assert!(!validate_pan("ABCDE12345"), "PAN without trailing letter must fail");
    }

    #[test]
    fn test_pan_invalid_lowercase() {
        // D-03 requires strict uppercase
        assert!(!validate_pan("abcde1234f"), "lowercase PAN must fail");
    }

    // SSN
    #[test]
    fn test_ssn_valid() {
        assert!(validate_ssn("123-45-6789"), "valid SSN must pass");
    }

    #[test]
    fn test_ssn_invalid_area_000() {
        assert!(!validate_ssn("000-45-6789"), "area code 000 must fail");
    }

    #[test]
    fn test_ssn_invalid_area_666() {
        assert!(!validate_ssn("666-45-6789"), "area code 666 must fail");
    }

    #[test]
    fn test_ssn_invalid_area_900() {
        assert!(!validate_ssn("900-45-6789"), "area code 900-999 must fail");
    }

    // NINO
    #[test]
    fn test_nino_valid() {
        assert!(validate_nino("AB123456C"), "AB123456C is a valid NINO");
        assert!(validate_nino("SW123456A"), "SW123456A is a valid NINO");
    }

    #[test]
    fn test_nino_invalid_excluded_prefix() {
        // 'D' is an excluded first character
        assert!(!validate_nino("DA123456C"), "prefix starting with D must fail");
        // 'Q' is excluded
        assert!(!validate_nino("QA123456C"), "prefix starting with Q must fail");
    }

    #[test]
    fn test_nino_invalid_suffix() {
        // Suffix must be A-D only
        assert!(!validate_nino("AB123456E"), "suffix 'E' must fail");
        assert!(!validate_nino("AB123456Z"), "suffix 'Z' must fail");
    }

    // SIN
    #[test]
    fn test_sin_valid() {
        // 046454286 is the documented Luhn-valid SIN test vector
        assert!(validate_sin("046454286"), "046454286 must pass Luhn");
    }

    #[test]
    fn test_sin_invalid_luhn() {
        assert!(!validate_sin("046454287"), "046454287 must fail Luhn (last digit changed)");
    }

    // VIN
    #[test]
    fn test_vin_valid() {
        // NHTSA documented test vector: sum=351, 351%11=10 → check='X' at index 8.
        assert!(validate_vin("1M8GDM9AXKP042788"), "1M8GDM9AXKP042788 is a documented valid VIN");
    }

    #[test]
    fn test_vin_invalid_checksum() {
        // Change check digit from X to Y → must fail.
        assert!(!validate_vin("1M8GDM9AYKP042788"), "check digit Y instead of X must fail");
    }

    #[test]
    fn test_vin_wrong_length() {
        assert!(!validate_vin("1M8GDM9AXK"), "VIN shorter than 17 must fail");
    }

    // GSTIN
    #[test]
    fn test_gstin_valid() {
        // Computed test vectors using this implementation's Mod-36 algorithm:
        // "27AABCU9603R1Z": total=193, check=(36-193%36)%36=(36-13)%36=23 → 'N'
        assert!(validate_gstin("27AABCU9603R1ZN"), "27AABCU9603R1ZN must be valid per Mod-36 algorithm");
        // "29GGGGG9999G1Z": total=254, check=(36-254%36)%36=(36-2)%36=34 → 'Y'
        assert!(validate_gstin("29GGGGG9999G1ZY"), "29GGGGG9999G1ZY must be valid per Mod-36 algorithm");
    }

    #[test]
    fn test_gstin_invalid_checksum() {
        assert!(!validate_gstin("27AABCU9603R1ZX"), "corrupted check char must fail");
        assert!(!validate_gstin("29GGGGG9999G1ZA"), "corrupted check char must fail");
    }

    #[test]
    fn test_gstin_invalid_format() {
        assert!(!validate_gstin("27AABCU9603R1AY"), "position 13 must be 'Z'");
        assert!(!validate_gstin("27aabcu9603r1zn"), "lowercase must fail format check");
    }

    // Weak-format IDs
    #[test]
    fn test_weak_id_policy_number() {
        let e = extractor();
        let entities = e.extract("Policy number: POL-2024-4531 applies to this claim.").unwrap();
        let weak: Vec<_> = entities.iter()
            .filter(|e| e.entity_type == "identifier" && e.subclass.as_deref() == Some("unknown"))
            .collect();
        assert!(!weak.is_empty(), "policy number should produce a weak-format identifier entity");
        assert!(weak.iter().any(|w| w.value.contains("POL-2024-4531")));
        // D-04: confidence must be 0.7 for weak-format IDs
        assert!(weak.iter().all(|w| (w.confidence.unwrap_or(0.0) - 0.7).abs() < 1e-5));
    }

    #[test]
    fn test_weak_id_invoice_number() {
        let e = extractor();
        let entities = e.extract("Invoice No. INV-20240601 for services rendered.").unwrap();
        let weak: Vec<_> = entities.iter()
            .filter(|e| e.entity_type == "identifier" && e.subclass.as_deref() == Some("unknown"))
            .collect();
        assert!(!weak.is_empty(), "invoice number should produce a weak identifier");
    }

    // Integration: identifier extraction via extract()
    #[test]
    fn test_extract_pan_from_text() {
        let e = extractor();
        let entities = e.extract("PAN of holder: ABCDE1234F as per records.").unwrap();
        let ids: Vec<_> = entities.iter()
            .filter(|e| e.entity_type == "identifier" && e.subclass.as_deref() == Some("pan"))
            .collect();
        assert!(!ids.is_empty(), "PAN should be extracted from text");
    }

    #[test]
    fn test_extract_ssn_from_text() {
        let e = extractor();
        let entities = e.extract("Employee SSN: 123-45-6789 on file.").unwrap();
        let ids: Vec<_> = entities.iter()
            .filter(|e| e.entity_type == "identifier" && e.subclass.as_deref() == Some("ssn"))
            .collect();
        assert!(!ids.is_empty(), "SSN should be extracted from text");
    }
}
