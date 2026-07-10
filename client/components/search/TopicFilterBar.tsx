/**
 * TopicFilterBar — chip row showing top topics by count.
 *
 * Phase 8 Plan 09 — UI-SPEC §6 Topic Filter.
 *
 * Props:
 *   selected: the currently-selected topic slug (snake_case) or null
 *   onSelect: called with the topic string when a chip is clicked, or null to clear
 *
 * Renders:
 *   [Topics:] [chip 1] [chip 2] ... [chip N] [Show more]
 *
 * Pagination: initial 20 chips, each click of "Show more" reveals 20 more (D-37).
 * Null return when: data not yet loaded OR no topics indexed (hides empty bar).
 */

import { useState } from "react";
import { Bookmark } from "lucide-react";
import { cn } from "@/lib/utils";
import { useTopics } from "@/hooks/useTauri";

// ── Display transform ──────────────────────────────────────────────────────────

/**
 * Convert a snake_case topic slug to sentence-case display text.
 * Per UI-SPEC §6 Copywriting: "term_insurance" → "Term insurance".
 * Underscore-to-space + capitalize first letter only.
 */
export function formatTopic(topic: string): string {
  const spaced = topic.replace(/_/g, " ");
  if (!spaced) return spaced;
  return spaced.charAt(0).toUpperCase() + spaced.slice(1);
}

// ── TopicFilterChip ────────────────────────────────────────────────────────────

export interface TopicFilterChipProps {
  /** snake_case topic slug */
  topic: string;
  /** Number of documents with this topic */
  count: number;
  /** Whether this chip is the currently-selected filter */
  active: boolean;
  /** Called when the chip is clicked */
  onClick: () => void;
}

/**
 * A single interactive chip inside TopicFilterBar.
 *
 * Active state: accent-fill (bg-accent-primary text-white border-accent-primary)
 * Inactive state: neutral (bg-bg-secondary text-text-secondary border-border-primary hover:bg-bg-tertiary)
 *
 * Intentionally uses a raw <button> to match the existing FilterChip interactive pattern
 * in SearchPage.tsx. Includes a count suffix ("Finance · 12") to help users pick
 * popular filters.
 *
 * Note: TopicChip (non-interactive, document sidebar) uses <div>.
 * TopicFilterChip is the exception — it IS interactive (per UI-SPEC §5 note).
 */
export function TopicFilterChip({ topic, count, active, onClick }: TopicFilterChipProps) {
  return (
    <button
      data-testid="topic-filter-chip"
      onClick={onClick}
      aria-pressed={active}
      aria-label={`Filter by topic: ${formatTopic(topic)}`}
      className={cn(
        "px-3 py-1 rounded-full text-xs inline-flex items-center gap-1 transition-colors border",
        active
          ? "bg-accent-primary text-white border-accent-primary"
          : "bg-bg-secondary text-text-secondary border-border-primary hover:bg-bg-tertiary",
      )}
    >
      <Bookmark size={12} />
      {formatTopic(topic)}
      <span className="opacity-60 ml-0.5">· {count}</span>
    </button>
  );
}

// ── TopicFilterBar ─────────────────────────────────────────────────────────────

export interface TopicFilterBarProps {
  /** Currently-selected topic slug, or null when no filter active */
  selected: string | null;
  /**
   * Called when a chip is clicked.
   * Receives the topic slug to select, or null to clear the filter
   * (when the user clicks the already-selected chip).
   */
  onSelect: (topic: string | null) => void;
}

/**
 * Renders a horizontal row of topic filter chips.
 *
 * Uses useTopics() internally — parent owns the filter state via `selected`/`onSelect`.
 * Returns null when:
 *   - data is undefined (loading or error)
 *   - data is empty (no topics indexed yet)
 *
 * Pagination: starts at 20 visible chips. "Show more" increments by 20 each click.
 * "Show more" button is hidden when all topics are visible (visibleCount >= data.length).
 *
 * Layout: "Topics:" label + chip row + optional "Show more" inline button.
 * Per UI-SPEC §6: "Topics:" label is text-xs text-text-tertiary.
 */
export function TopicFilterBar({ selected, onSelect }: TopicFilterBarProps) {
  const [visibleCount, setVisibleCount] = useState(20);
  const { data: topics } = useTopics();

  // Don't render empty bar — preserves layout stability when no topics indexed
  if (!topics || topics.length === 0) {
    return null;
  }

  const visibleTopics = topics.slice(0, visibleCount);
  const hasMore = visibleCount < topics.length;

  return (
    <div className="flex flex-wrap items-center gap-2">
      <span className="text-xs text-text-tertiary">Topics:</span>
      {visibleTopics.map((tc) => (
        <TopicFilterChip
          key={tc.topic}
          topic={tc.topic}
          count={tc.count}
          active={selected === tc.topic}
          onClick={() => onSelect(selected === tc.topic ? null : tc.topic)}
        />
      ))}
      {hasMore && (
        <button
          className="text-xs text-accent-primary hover:text-accent-hover ml-2 inline-block"
          onClick={() => setVisibleCount((v) => v + 20)}
          aria-label="Show more topics"
        >
          Show more
        </button>
      )}
    </div>
  );
}
