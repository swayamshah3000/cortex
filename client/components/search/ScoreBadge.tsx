import { cn } from "../../lib/utils";

/**
 * Displays a document relevance score as a percentage badge.
 *
 * Semantic color thresholds (mirrors SearchPage inline implementation exactly):
 *   >= 80%  → green  (high relevance: text-green-400 bg-green-400/10)
 *   >= 50%  → amber  (medium relevance: text-amber-400 bg-amber-400/10)
 *    < 50%  → neutral (low relevance: text-text-tertiary bg-bg-tertiary)
 *
 * Extracted from client/pages/SearchPage.tsx (Phase 11 Plan 07) for reuse
 * on the DocumentPage Related Documents panel and any future score displays.
 * No visual change from the original inline component.
 */
export function ScoreBadge({ score }: { score: number }) {
  const pct = Math.round(score * 100);
  const color =
    pct >= 80
      ? "text-green-400 bg-green-400/10"
      : pct >= 50
        ? "text-amber-400 bg-amber-400/10"
        : "text-text-tertiary bg-bg-tertiary";
  return (
    <span className={cn("text-xs font-mono px-2 py-0.5 rounded", color)}>
      {pct}%
    </span>
  );
}
