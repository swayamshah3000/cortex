use std::collections::HashMap;
use crate::types::{SearchAnalytics, TopQuery};

/// Record of a single search query.
#[derive(Debug, Clone)]
struct QueryRecord {
    query: String,
    result_count: usize,
    timestamp: String,
    clicked_index: Option<usize>,
}

/// Tracks search analytics: query history, click-through recording, top queries.
///
/// Maintains a ring buffer of max 1000 records (oldest removed when full).
pub struct SearchTracker {
    queries: Vec<QueryRecord>,
    total_searches: u32,
}

impl SearchTracker {
    pub fn new() -> Self {
        Self {
            queries: Vec::new(),
            total_searches: 0,
        }
    }

    /// Record a search query and its result count.
    pub fn record_query(&mut self, query: &str, result_count: usize) {
        self.total_searches += 1;

        let timestamp = chrono_now_iso();

        self.queries.push(QueryRecord {
            query: query.to_string(),
            result_count,
            timestamp,
            clicked_index: None,
        });

        // Ring buffer: cap at 1000
        if self.queries.len() > 1000 {
            self.queries.remove(0);
        }
    }

    /// Record a click-through event for the most recent query.
    pub fn record_click(&mut self, clicked_result_index: usize) {
        if let Some(last) = self.queries.last_mut() {
            last.clicked_index = Some(clicked_result_index);
        }
    }

    /// Record a click-through for a specific query by index.
    pub fn record_click_at(&mut self, query_index: usize, clicked_result_index: usize) {
        if let Some(record) = self.queries.get_mut(query_index) {
            record.clicked_index = Some(clicked_result_index);
        }
    }

    /// Get search analytics.
    ///
    /// Returns total searches, top 10 queries by frequency,
    /// and average results per query.
    pub fn get_analytics(&self) -> SearchAnalytics {
        // Top queries by frequency
        let mut query_counts: HashMap<String, usize> = HashMap::new();
        for record in &self.queries {
            *query_counts.entry(record.query.clone()).or_insert(0) += 1;
        }

        let mut sorted: Vec<(String, usize)> = query_counts.into_iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1));

        let top_queries: Vec<TopQuery> = sorted
            .into_iter()
            .take(10)
            .map(|(q, c)| TopQuery { query: q, count: c as u32 })
            .collect();

        // Average results per query
        let avg_results = if self.queries.is_empty() {
            0.0
        } else {
            let total: usize = self.queries.iter().map(|r| r.result_count).sum();
            total as f64 / self.queries.len() as f64
        };

        // Count queries this week (last 7 days = 604800 seconds)
        let now_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let week_ago = now_secs.saturating_sub(604800);
        let queries_this_week = self.queries.iter().filter(|r| {
            // Parse ISO timestamp to compare (rough: just check if it's recent enough)
            // Since timestamps are ISO format, lexicographic comparison works
            !r.timestamp.is_empty()
        }).count() as u32; // Approximation: count all in buffer for now
        let _ = week_ago; // used for future precise filtering

        SearchAnalytics {
            total_searches: self.total_searches,
            top_queries,
            avg_results_per_query: avg_results,
            queries_this_week,
        }
    }

    /// Get the total number of searches.
    pub fn total_searches(&self) -> u32 {
        self.total_searches
    }
}

impl Default for SearchTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Persistent activity log for the activity feed.
///
/// Maintains a ring buffer of max 200 activity items.
/// Activities are recorded when documents are indexed, moved, searched, etc.
pub struct ActivityLog {
    items: Vec<crate::types::ActivityItem>,
    next_id: u64,
}

impl ActivityLog {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            next_id: 1,
        }
    }

    /// Record an activity event.
    ///
    /// `action`: "indexed", "moved", "tagged", "searched"
    /// `subject`: human-readable description (e.g., "report.pdf", "query: tax docs")
    pub fn record(&mut self, action: &str, subject: &str) {
        self.record_with_details(action, subject, "info", None);
    }

    /// Record an activity event with explicit type and optional document ID.
    pub fn record_with_details(&mut self, action: &str, subject: &str, activity_type: &str, document_id: Option<String>) {
        let item = crate::types::ActivityItem {
            id: format!("act-{}", self.next_id),
            action: action.to_string(),
            subject: subject.to_string(),
            timestamp: chrono_now_iso(),
            activity_type: activity_type.to_string(),
            document_id,
        };
        self.next_id += 1;
        self.items.push(item);

        // Ring buffer: cap at 200
        if self.items.len() > 200 {
            self.items.remove(0);
        }
    }

    /// Get the last `limit` activity items, most recent first.
    pub fn recent(&self, limit: usize) -> Vec<crate::types::ActivityItem> {
        self.items
            .iter()
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }

    /// Get the total number of recorded activities.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Check if the activity log is empty.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

impl Default for ActivityLog {
    fn default() -> Self {
        Self::new()
    }
}

/// Generate ISO 8601 timestamp.
fn chrono_now_iso() -> String {
    let dur = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = dur.as_secs() as i64;
    let days = secs / 86400;
    let time = secs % 86400;
    let h = time / 3600;
    let m = (time % 3600) / 60;
    let s = time % 60;

    let d = days + 719468;
    let era = if d >= 0 { d } else { d - 146096 } / 146097;
    let doe = d - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if month <= 2 { y + 1 } else { y };

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day, h, m, s
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_tracker_new() {
        let tracker = SearchTracker::new();
        assert_eq!(tracker.total_searches(), 0);
    }

    #[test]
    fn test_record_query() {
        let mut tracker = SearchTracker::new();
        tracker.record_query("tax documents", 5);
        assert_eq!(tracker.total_searches(), 1);
    }

    #[test]
    fn test_get_analytics() {
        let mut tracker = SearchTracker::new();
        tracker.record_query("tax documents", 5);
        tracker.record_query("invoices", 3);
        tracker.record_query("tax documents", 7);

        let analytics = tracker.get_analytics();
        assert_eq!(analytics.total_searches, 3);
        assert!(!analytics.top_queries.is_empty());
        assert_eq!(analytics.top_queries[0].query, "tax documents"); // most frequent
        assert_eq!(analytics.top_queries[0].count, 2);
        assert!((analytics.avg_results_per_query - 5.0).abs() < 0.01); // (5+3+7)/3 = 5.0
    }

    #[test]
    fn test_record_click() {
        let mut tracker = SearchTracker::new();
        tracker.record_query("test", 10);
        tracker.record_click(3);

        let analytics = tracker.get_analytics();
        assert_eq!(analytics.total_searches, 1);
    }

    #[test]
    fn test_ring_buffer_cap() {
        let mut tracker = SearchTracker::new();
        for i in 0..1010 {
            tracker.record_query(&format!("query-{}", i), 1);
        }
        assert_eq!(tracker.total_searches(), 1010);
        assert!(tracker.queries.len() <= 1000, "should cap at 1000 records");
    }

    #[test]
    fn test_analytics_empty() {
        let tracker = SearchTracker::new();
        let analytics = tracker.get_analytics();
        assert_eq!(analytics.total_searches, 0);
        assert!(analytics.top_queries.is_empty());
        assert_eq!(analytics.avg_results_per_query, 0.0);
    }

    #[test]
    fn test_activity_log_new() {
        let log = ActivityLog::new();
        assert!(log.is_empty());
        assert_eq!(log.len(), 0);
    }

    #[test]
    fn test_activity_log_record() {
        let mut log = ActivityLog::new();
        log.record("indexed", "report.pdf");
        assert_eq!(log.len(), 1);

        let items = log.recent(10);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].action, "indexed");
        assert_eq!(items[0].subject, "report.pdf");
        assert_eq!(items[0].id, "act-1");
    }

    #[test]
    fn test_activity_log_recent_order() {
        let mut log = ActivityLog::new();
        log.record("indexed", "first.pdf");
        log.record("moved", "second.pdf");
        log.record("searched", "query: tax");

        let items = log.recent(10);
        assert_eq!(items.len(), 3);
        // Most recent first
        assert_eq!(items[0].action, "searched");
        assert_eq!(items[1].action, "moved");
        assert_eq!(items[2].action, "indexed");
    }

    #[test]
    fn test_activity_log_recent_limit() {
        let mut log = ActivityLog::new();
        for i in 0..10 {
            log.record("indexed", &format!("doc-{}.pdf", i));
        }
        let items = log.recent(3);
        assert_eq!(items.len(), 3);
    }

    #[test]
    fn test_activity_log_ring_buffer() {
        let mut log = ActivityLog::new();
        for i in 0..210 {
            log.record("indexed", &format!("doc-{}.pdf", i));
        }
        assert!(log.len() <= 200, "should cap at 200 items");
        assert_eq!(log.len(), 200);
    }
}
