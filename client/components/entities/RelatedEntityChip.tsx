/**
 * RelatedEntityChip — EntityChip + co-occurrence count badge.
 * Used by Plan 07 EntityDetailPage.
 *
 * Shows "× {n}" count badge to the right of the chip.
 * Tooltip: "Co-occurs in {n} documents with this entity"
 */

import { EntityChip } from "./EntityChip";
import type { RelatedEntity } from "@/lib/types";

interface RelatedEntityChipProps {
  related: RelatedEntity;
}

export function RelatedEntityChip({ related }: RelatedEntityChipProps) {
  return (
    <div className="inline-flex items-center gap-1" title={`Co-occurs in ${related.coOccurrenceCount} documents with this entity`}>
      <EntityChip
        entity={{
          value: related.entity.canonicalName,
          entityType: related.entity.entityType,
          canonicalId: related.entity.id,
        }}
      />
      <span className="text-[10px] text-text-tertiary tabular-nums">
        × {related.coOccurrenceCount}
      </span>
    </div>
  );
}
