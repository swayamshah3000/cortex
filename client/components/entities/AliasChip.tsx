/**
 * AliasChip — A single alias surface form with Split scissors button.
 *
 * When isCanonical=true, shows a checkmark and hides the Split button.
 * The Split button is hidden (opacity-0) by default and revealed on hover/focus.
 *
 * Plan 06-07 Task 1
 */

import { Scissors, Check } from "lucide-react";

interface AliasChipProps {
  alias: string;
  isCanonical: boolean;
  onSplit: () => void;
}

export function AliasChip({ alias, isCanonical, onSplit }: AliasChipProps) {
  return (
    <span className="group inline-flex items-center gap-2 px-3 py-1.5 rounded-full bg-bg-tertiary border border-border-secondary">
      {/* Canonical indicator */}
      {isCanonical && (
        <Check size={12} className="text-accent-primary flex-shrink-0" />
      )}

      <span className="text-sm text-text-primary">{alias}</span>

      {/* Split button — hidden unless canonical */}
      {!isCanonical && (
        <button
          type="button"
          onClick={onSplit}
          aria-label={`Split alias '${alias}' off`}
          className="opacity-0 group-hover:opacity-100 group-focus-within:opacity-100 transition-opacity p-0.5 rounded hover:bg-bg-primary text-text-tertiary hover:text-text-primary"
        >
          <Scissors size={14} />
        </button>
      )}
    </span>
  );
}
