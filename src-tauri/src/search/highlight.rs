/// Find the best-matching excerpt from document text for a query.
///
/// Uses a sliding window of `window_size` words over the document text,
/// scoring each window by the count of query word overlaps. Returns the
/// highest-scoring window as the excerpt string.
///
/// If no query words match (score 0), returns the first `window_size` words.
pub fn find_best_excerpt(doc_text: &str, query_text: &str, window_size: usize) -> String {
    let query_words: Vec<String> = tokenize(query_text);
    let doc_words: Vec<&str> = doc_text.split_whitespace().collect();

    if doc_words.is_empty() {
        return String::new();
    }

    if doc_words.len() <= window_size {
        return doc_words.join(" ");
    }

    if query_words.is_empty() {
        return doc_words[..window_size.min(doc_words.len())].join(" ");
    }

    let mut best_score = 0usize;
    let mut best_start = 0usize;

    for start in 0..=(doc_words.len() - window_size) {
        let mut score = 0usize;
        for word in &doc_words[start..start + window_size] {
            let lower = word.to_lowercase();
            let cleaned = clean_word(&lower);
            if !cleaned.is_empty() && query_words.contains(&cleaned) {
                score += 1;
            }
        }
        if score > best_score {
            best_score = score;
            best_start = start;
        }
    }

    doc_words[best_start..best_start + window_size].join(" ")
}

/// Tokenize text into lowercase words, stripping punctuation.
fn tokenize(text: &str) -> Vec<String> {
    text.split_whitespace()
        .map(|w| clean_word(&w.to_lowercase()))
        .filter(|w| !w.is_empty())
        .collect()
}

/// Remove leading/trailing punctuation from a word.
fn clean_word(word: &str) -> String {
    word.trim_matches(|c: char| !c.is_alphanumeric())
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_best_excerpt_basic() {
        let doc = "The quick brown fox jumps over the lazy dog and the fox runs fast";
        let result = find_best_excerpt(doc, "fox jumps", 5);
        assert!(
            result.contains("fox") && result.contains("jumps"),
            "excerpt should contain query words, got: {}",
            result
        );
    }

    #[test]
    fn test_find_best_excerpt_no_match_returns_first_window() {
        let doc = "alpha beta gamma delta epsilon zeta eta theta iota kappa";
        let result = find_best_excerpt(doc, "xyz nonexistent", 3);
        assert_eq!(result, "alpha beta gamma");
    }

    #[test]
    fn test_find_best_excerpt_short_doc() {
        let doc = "hello world";
        let result = find_best_excerpt(doc, "hello", 30);
        assert_eq!(result, "hello world");
    }

    #[test]
    fn test_find_best_excerpt_empty_doc() {
        let result = find_best_excerpt("", "query", 30);
        assert_eq!(result, "");
    }

    #[test]
    fn test_find_best_excerpt_empty_query() {
        let doc = "one two three four five six seven eight";
        let result = find_best_excerpt(doc, "", 3);
        assert_eq!(result, "one two three");
    }

    #[test]
    fn test_find_best_excerpt_case_insensitive() {
        let doc = "Invoice TOTAL amount is five hundred dollars paid to John Smith";
        let result = find_best_excerpt(doc, "invoice total", 5);
        let lower = result.to_lowercase();
        assert!(
            lower.contains("invoice") && lower.contains("total"),
            "should match case-insensitively, got: {}",
            result
        );
    }

    #[test]
    fn test_tokenize() {
        let words = tokenize("Hello, World! Test.");
        assert_eq!(words, vec!["hello", "world", "test"]);
    }

    #[test]
    fn test_clean_word() {
        assert_eq!(clean_word("hello,"), "hello");
        assert_eq!(clean_word("(test)"), "test");
        assert_eq!(clean_word("..."), "");
    }
}
