/**
 * SpaceLabelingIndicator — Progress chip shown in SpacesPage header while a
 * LLM batch labeling run is in flight.
 *
 * Mirrors BackfillIndicator behavioral pattern (Plan 08-07) but uses grid-aligned
 * spacing: px-3 py-1 gap-2 (NOT px-2.5 / py-0.5 / gap-1.5 — those are sub-grid).
 *
 * States (09-UI-SPEC.md §8):
 *   idle     → renders null (not mounted in DOM)
 *   labeling → spinner + "Labeling spaces X/Y"  (accent/10 bg)
 *   complete → "Labels ready"                    (success/10 bg) — auto-dismisses after 3s
 *   error    → "Labeling failed"                 (error/10 bg)   — stays until store cleared
 *
 * Mount once at SpacesPage header. useSpaceLabelingProgress() (mounted at AppShell)
 * feeds the store that this component reads.
 *
 * Plan 09-06 Task 2.
 */

import { useEffect } from "react";
import { Loader2 } from "lucide-react";
import { useSpaceLabelingStore } from "@/lib/stores";

export function SpaceLabelingIndicator() {
  const { status, processed, total } = useSpaceLabelingStore();

  // Auto-dismiss the complete chip after 3 seconds (per 09-UI-SPEC.md §8 States table).
  // Cleanup cancels the timer if the component unmounts before 3s elapses.
  useEffect(() => {
    if (status !== "complete") return;

    const timer = setTimeout(() => {
      useSpaceLabelingStore.getState().clear();
    }, 3000);

    return () => clearTimeout(timer);
  }, [status]);

  if (status === "idle") {
    return null;
  }

  if (status === "labeling") {
    return (
      <div
        className="flex items-center gap-2 px-3 py-1 rounded-full bg-accent-primary/10 text-accent-primary text-xs font-medium"
        aria-live="polite"
      >
        <Loader2 size={12} className="animate-spin" strokeWidth={1.5} />
        Labeling spaces {processed}/{total}
      </div>
    );
  }

  if (status === "complete") {
    return (
      <div
        className="flex items-center gap-2 px-3 py-1 rounded-full bg-success/10 text-success text-xs font-medium"
        aria-live="polite"
      >
        Labels ready
      </div>
    );
  }

  // status === "error"
  return (
    <div
      className="flex items-center gap-2 px-3 py-1 rounded-full bg-error/10 text-error text-xs font-medium"
      aria-live="polite"
    >
      Labeling failed
    </div>
  );
}
