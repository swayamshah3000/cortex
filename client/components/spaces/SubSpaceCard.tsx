/**
 * SubSpaceCard — Compact card variant for sub-spaces in SpaceDetailPage sub-space grid.
 *
 * Extracted from SpaceDetailPage.tsx (Plan 10-08) and extended with Phase 10 features:
 *   - Icon 18px (smaller than SpaceCard 24px — reinforces sub hierarchy)
 *   - Padding p-4 (16px) vs SpaceCard p-6 (24px)
 *   - Name text-sm font-medium vs SpaceCard text-lg font-semibold
 *   - isMisc prop: adds border-dashed on the left border for "Misc" unclustered sub-spaces
 *
 * UI-SPEC §5 constraints:
 *   - No tooltip (description tooltip reserved for top-level SpaceCard)
 *   - No entity hint chip (reserved for top-level SpaceCard)
 *   - No sub-count footer (sub-spaces never have sub-sub-spaces — max depth = 2)
 *
 * Plan 10-08 Task 1.
 */

import { Link } from "react-router-dom";
import { resolveIcon } from "@/lib/icons";
import { cn } from "@/lib/utils";
import type { Space } from "@/lib/types";

export interface SubSpaceCardProps {
  space: Space;
  /** true when space.name === "Misc" — adds border-dashed to the left border */
  isMisc?: boolean;
}

export function SubSpaceCard({ space, isMisc = false }: SubSpaceCardProps) {
  const Icon = resolveIcon(space.icon);
  return (
    <Link
      to={`/spaces/${space.id}`}
      className={cn(
        "card p-4 hover:shadow-md hover:border-accent-primary/50 transition-all border-l-4",
        isMisc && "border-dashed"
      )}
      style={{ borderLeftColor: space.color }}
    >
      <div className="flex items-center gap-3">
        <div className="p-2 rounded-lg bg-accent-subtle text-accent-primary">
          <Icon size={18} />
        </div>
        <div className="flex-1 min-w-0">
          <p className="font-medium text-text-primary text-sm truncate">{space.name}</p>
          <p className="text-xs text-text-tertiary">{space.documentCount} docs</p>
        </div>
      </div>
    </Link>
  );
}
