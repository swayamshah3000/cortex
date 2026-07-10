/**
 * useSpaceLabelingProgress — Mounts a single Tauri event listener for
 * "space-labeling-progress" and routes payloads into useSpaceLabelingStore.
 *
 * Should be called once at AppShell level (Plan 06 handles mounting).
 * No-ops cleanly when not in Tauri runtime — mockSpaces still render.
 *
 * On status "complete" or "error", invalidates the `spaces` and `spaceLabels`
 * React Query caches so SpaceCard re-renders with the final LLM-generated label.
 *
 * Cross-references:
 *   - 09-CONTEXT.md D-14: UI while labeling — SpaceCard shimmer + "Generating label…"
 *   - 09-UI-SPEC.md §8: Space labeling progress event subscription
 *
 * Plan 09-05 Task 2 — mirrors useBackfillProgress pattern (Plan 08-07).
 */

import { useEffect } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { isTauri } from "@/lib/tauri";
import { useSpaceLabelingStore } from "@/lib/stores";
import { queryKeys } from "@/hooks/useTauri";
import type { SpaceLabelingProgress } from "@/lib/types";

export function useSpaceLabelingProgress() {
  // Capture queryClient at component scope — safe to use inside async effect closure.
  const queryClient = useQueryClient();

  useEffect(() => {
    if (!isTauri()) return;

    let unlisten: (() => void) | undefined;
    let cancelled = false;

    (async () => {
      const { listen } = await import("@tauri-apps/api/event");
      // Guard: if the component unmounted during the dynamic import await, do not
      // register the listener at all (fixes listener leak on rapid unmount cycles).
      if (cancelled) return;
      unlisten = await listen<SpaceLabelingProgress>(
        "space-labeling-progress",
        (event) => {
          // Push payload into the Zustand store (drives SpaceCard shimmer + progress bar).
          useSpaceLabelingStore.getState().setProgress(event.payload);

          // Invalidate spaces + labels query cache when this labeling round settles
          // so the UI refetches with the final LLM-generated label (no stale names).
          if (
            event.payload.status === "complete" ||
            event.payload.status === "error"
          ) {
            queryClient.invalidateQueries({ queryKey: queryKeys.spaces });
            queryClient.invalidateQueries({ queryKey: queryKeys.spaceLabels });
          }
        },
      );
      // Second guard: component may have unmounted between the first check and
      // the listen() promise resolving — clean up immediately in that case.
      if (cancelled) {
        unlisten();
        unlisten = undefined;
      }
    })();

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [queryClient]);
}
