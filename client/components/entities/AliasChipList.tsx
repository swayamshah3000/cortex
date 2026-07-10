/**
 * AliasChipList — Section wrapper for alias chips on /entities/:id.
 *
 * Hides itself entirely when aliases.length === 1 AND aliases[0] === canonicalName
 * (the only alias is the canonical name itself — nothing meaningful to show).
 *
 * Plan 06-07 Task 1
 */

import { AliasChip } from "./AliasChip";

interface AliasChipListProps {
  aliases: string[];
  canonicalName: string;
  onSplit: (alias: string) => void;
}

export function AliasChipList({ aliases, canonicalName, onSplit }: AliasChipListProps) {
  // Hide section if only one alias and it equals the canonical name
  if (aliases.length === 1 && aliases[0] === canonicalName) {
    return null;
  }

  return (
    <div className="space-y-3">
      <h2 className="section-header text-text-primary">Aliases ({aliases.length})</h2>
      <p className="text-sm text-text-secondary">
        These surface forms were merged into this entity. If any look wrong, click Split to
        separate.
      </p>
      <div className="flex flex-wrap gap-2">
        {aliases.map((alias) => (
          <AliasChip
            key={alias}
            alias={alias}
            isCanonical={alias === canonicalName}
            onSplit={() => onSplit(alias)}
          />
        ))}
      </div>
    </div>
  );
}
