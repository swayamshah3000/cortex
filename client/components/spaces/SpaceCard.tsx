/**
 * SpaceCard — Grid card for a single Smart Space.
 *
 * Extracted from SpacesPage.tsx (Plan 09-06) and extended with Phase 9 UI states:
 *
 *   §1 Shimmer (D-14, LLML-05):
 *     When space.labelStatus === 'generating', shows Skeleton placeholders and
 *     "Generating label…" sub-line instead of the real name/count.
 *
 *   §2 Description tooltip (D-16, LLML-02):
 *     Wraps the Link in a Radix Tooltip. When space.description is non-empty,
 *     shows the first 100 chars on hover. Truncation is done in JS (not CSS).
 *
 *   §3 Lock icon (D-15):
 *     When space.userLocked === true, renders a 12px Lock icon in the header row
 *     alongside the timestamp.
 *
 *   §4 Canonical entity hint chip (D-17):
 *     When space.canonicalEntityHint is non-null AND labelStatus !== 'generating',
 *     renders EntityHintChip below the sample files list.
 *
 * Spacing: all values are 4px-grid-aligned. No px-2.5 / py-0.5 / gap-1.5.
 *
 * Plan 09-06 Task 1.
 */

import { Link } from "react-router-dom";
import { Lock, FileText } from "lucide-react";
import { Skeleton } from "@/components/ui/skeleton";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { resolveIcon } from "@/lib/icons";
import { formatRelativeTime } from "@/lib/format";
import { EntityHintChip } from "./EntityHintChip";
import { useSpaceLabelingStore } from "@/lib/stores";
import type { Space } from "@/lib/types";

/**
 * Truncates description to 100 chars with an ellipsis.
 * Uses JS slice — NOT CSS truncate class — per 09-UI-SPEC.md §2.
 */
function truncateAt100(text: string): string {
  return text.length > 100 ? text.slice(0, 100) + "…" : text;
}

interface SpaceCardProps {
  space: Space;
}

export function SpaceCard({ space }: SpaceCardProps) {
  const Icon = resolveIcon(space.icon);
  const generatingIds = useSpaceLabelingStore((s) => s.generatingSpaceIds);
  // Generating when backend signals it via Space.labelStatus OR the Tauri event
  // "space-labeling-progress" (which updates generatingSpaceIds in the store).
  const isGenerating =
    space.labelStatus === "generating" || generatingIds.has(space.id);

  return (
    <TooltipProvider>
      <Tooltip>
        <TooltipTrigger asChild>
          <Link
            to={`/spaces/${space.id}`}
            className="card p-6 hover:shadow-lg hover:border-accent-primary/50 transition-all border-l-4"
            style={{ borderLeftColor: space.color }}
          >
            <div className="space-y-3">
              {/* Header row: space icon + (lock indicator + timestamp) */}
              <div className="flex items-start justify-between">
                <div className="p-2 rounded-lg bg-accent-subtle text-accent-primary">
                  <Icon size={24} />
                </div>
                <div className="flex items-center gap-1">
                  {space.userLocked && (
                    <Lock
                      size={12}
                      className="text-text-tertiary"
                      aria-label="Label locked by user"
                    />
                  )}
                  <span className="text-xs text-text-tertiary">
                    {formatRelativeTime(space.lastUpdated)}
                  </span>
                </div>
              </div>

              {/* Label region — shimmer while generating, prose when ready */}
              {isGenerating ? (
                <div>
                  <Skeleton className="h-5 w-36" />
                  <Skeleton className="h-4 w-24 mt-1" />
                  <span className="text-xs text-text-tertiary mt-1 block">
                    Generating label…
                  </span>
                </div>
              ) : (
                <div>
                  <p className="font-semibold text-text-primary text-lg">
                    {space.name}
                  </p>
                  <p className="text-text-tertiary text-sm">
                    {space.documentCount} documents
                  </p>
                </div>
              )}

              {/* Sample files list */}
              {space.sampleFiles.length > 0 && (
                <div className="space-y-1 pt-1">
                  {space.sampleFiles.slice(0, 3).map((file) => (
                    <div
                      key={file}
                      className="flex items-center gap-1 text-xs text-text-tertiary"
                    >
                      <FileText size={12} className="flex-shrink-0" />
                      <span className="truncate">{file}</span>
                    </div>
                  ))}
                </div>
              )}

              {/* Canonical entity hint chip — hidden while label is generating */}
              {space.canonicalEntityHint && !isGenerating && (
                <div className="pt-1">
                  <EntityHintChip hint={space.canonicalEntityHint} />
                </div>
              )}
            </div>
          </Link>
        </TooltipTrigger>

        {/* Description tooltip — only rendered when description is present */}
        {space.description ? (
          <TooltipContent
            side="bottom"
            sideOffset={4}
            className="max-w-[240px] text-xs leading-relaxed"
          >
            {truncateAt100(space.description)}
          </TooltipContent>
        ) : null}
      </Tooltip>
    </TooltipProvider>
  );
}
