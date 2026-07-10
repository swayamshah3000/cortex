/**
 * TagChip — Displays a single LLM-extracted free-form tag.
 *
 * Visual: neutral rectangular pill (rounded-md) with Hash icon prefix.
 * Non-clickable in Phase 8 (div, not button or Link).
 * Phase 11 will add filter behavior.
 *
 * Display transform: snake_case → space-separated (NO capitalization)
 *   "khush_school" → "khush school"
 *
 * max-w-[120px] truncate prevents overly long tags from breaking layout.
 *
 * UI-SPEC §Topic vs Tag Visual Distinction Contract:
 *   bg-bg-tertiary, text-text-secondary, border-border-secondary, rounded-md
 */

import { Hash } from "lucide-react";

interface TagChipProps {
  tag: string;
}

export function TagChip({ tag }: TagChipProps) {
  if (!tag) return null;

  // Transform: replace underscores with spaces — no capitalization (unlike TopicChip)
  const display = tag.replace(/_/g, " ");

  return (
    <div
      className="inline-flex items-center gap-1 px-3 py-1 rounded-md text-xs bg-bg-tertiary text-text-secondary border border-border-secondary max-w-[120px] truncate"
      title={tag}
    >
      <Hash size={12} />
      <span className="truncate">{display}</span>
    </div>
  );
}
