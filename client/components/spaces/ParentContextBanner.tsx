/**
 * ParentContextBanner — Informational banner shown above content on SpaceDetailPage
 * when the current space is a sub-space (space.parentId is set and parentSpace resolves).
 *
 * Placement: Between the breadcrumb and the header section.
 * Trigger: space.parentId is set AND parentSpace is found in the spaces array.
 * Fail-silent: when parentId is set but parent not found, do NOT render (stale data).
 *
 * UI-SPEC §3 constraints:
 *   - Container: bg-bg-secondary border border-border-primary rounded-lg px-4 py-2
 *   - Icon: ArrowLeft 16px, text-text-tertiary
 *   - Body: "Sub-space of" in text-text-secondary
 *   - Parent name: text-accent-primary hover:text-accent-hover font-medium — IS the CTA
 *   - No separate "Back" button; parent name link IS the action
 *
 * Plan 10-08 Task 1.
 */

import { Link } from "react-router-dom";
import { ArrowLeft } from "lucide-react";
import type { Space } from "@/lib/types";

export interface ParentContextBannerProps {
  parent: Space;
}

export function ParentContextBanner({ parent }: ParentContextBannerProps) {
  return (
    <div className="flex items-center gap-2 px-4 py-2 rounded-lg bg-bg-secondary border border-border-primary text-sm text-text-secondary">
      <ArrowLeft size={16} className="text-text-tertiary flex-shrink-0" />
      <span>Sub-space of</span>
      <Link
        to={`/spaces/${parent.id}`}
        className="font-medium text-accent-primary hover:text-accent-hover transition-colors"
      >
        {parent.name}
      </Link>
    </div>
  );
}
