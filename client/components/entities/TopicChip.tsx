/**
 * TopicChip — Displays a single doc-level LLM-extracted topic.
 *
 * Visual: accent-tinted pill (rounded-full) with Bookmark icon prefix.
 * Non-clickable in Phase 8 (div, not button or Link).
 * Phase 11 will add filter behavior.
 *
 * Display transform: snake_case → "Sentence case"
 *   "term_insurance" → "Term insurance"
 *
 * Null-guards: returns null for empty string or "other" (default fallback topic).
 *
 * UI-SPEC §Topic vs Tag Visual Distinction Contract:
 *   bg-accent-primary/10, text-accent-primary, border-accent-primary/20, rounded-full
 */

import { Bookmark } from "lucide-react";

interface TopicChipProps {
  topic: string;
}

export function TopicChip({ topic }: TopicChipProps) {
  if (!topic || topic === "other") return null;

  // Transform: replace underscores with spaces, capitalize first letter of the result
  const display = topic.replace(/_/g, " ");
  const capitalized = display.charAt(0).toUpperCase() + display.slice(1);

  return (
    <div
      className="inline-flex items-center gap-1 px-3 py-1 rounded-full text-xs bg-accent-primary/10 text-accent-primary border border-accent-primary/20"
      title={topic}
    >
      <Bookmark size={12} />
      <span>{capitalized}</span>
    </div>
  );
}
