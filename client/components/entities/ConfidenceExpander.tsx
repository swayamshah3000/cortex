/**
 * ConfidenceExpander — Collapsible wrapper for low-confidence (< 0.7) entities.
 *
 * Shows "Also found ({count})" trigger with ChevronDown icon.
 * Expands via shadcn Collapsible to reveal low-confidence entities rendered
 * in muted italic style (OCR-tolerance signal to the user).
 *
 * Parent (DocumentPage) supplies renderEntity so the expander stays layout-agnostic.
 *
 * Null-guard: returns null when no entities have confidence < 0.7.
 *
 * UI-SPEC §ConfidenceExpander:
 *   - trigger: ChevronDown + "Also found ({count})"
 *   - inner chips: italic text-text-tertiary styling
 *   - aria-label: "Low-confidence entities — may contain OCR errors"
 */

import React, { useState } from "react";
import { ChevronDown } from "lucide-react";
import {
  Collapsible,
  CollapsibleTrigger,
  CollapsibleContent,
} from "@/components/ui/collapsible";
import { cn } from "@/lib/utils";
import type { ExtractedEntity } from "@/lib/types";

interface ConfidenceExpanderProps {
  entities: ExtractedEntity[];
  renderEntity: (e: ExtractedEntity) => React.ReactNode;
}

export function ConfidenceExpander({ entities, renderEntity }: ConfidenceExpanderProps) {
  const [open, setOpen] = useState(false);

  // Filter to entities that have an explicit confidence below the 0.7 threshold (D-15)
  // Entities without a confidence value are treated as high-confidence (no expander)
  const low = entities.filter(
    (e) => e.confidence != null && e.confidence < 0.7,
  );

  if (low.length === 0) return null;

  return (
    <Collapsible open={open} onOpenChange={setOpen}>
      <CollapsibleTrigger
        className="flex items-center gap-1"
        aria-label="Low-confidence entities — may contain OCR errors"
      >
        <ChevronDown
          size={12}
          className={cn(
            "text-text-tertiary transition-transform",
            open && "rotate-180",
          )}
        />
        <span className="text-xs text-text-tertiary">Also found ({low.length})</span>
      </CollapsibleTrigger>
      <CollapsibleContent className="mt-2">
        <div className="flex flex-wrap gap-1 italic text-text-tertiary">
          {low.map((e) => renderEntity(e))}
        </div>
      </CollapsibleContent>
    </Collapsible>
  );
}
