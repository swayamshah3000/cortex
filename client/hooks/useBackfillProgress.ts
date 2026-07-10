/**
 * useBackfillProgress — Mounts a single Tauri event listener for
 * "entity-backfill-progress" and routes payloads into useBackfillStore.
 *
 * Should be called once at AppShell level. No-ops when not in Tauri runtime.
 *
 * Plan 06-07 Task 2
 */

import { useEffect } from "react";
import { isTauri } from "@/lib/tauri";
import { useBackfillStore } from "@/lib/stores";
import type { EntityBackfillProgress } from "@/lib/types";

export function useBackfillProgress() {
  useEffect(() => {
    if (!isTauri()) return;

    let unlisten: (() => void) | undefined;

    (async () => {
      const { listen } = await import("@tauri-apps/api/event");
      unlisten = await listen<EntityBackfillProgress>(
        "entity-backfill-progress",
        (event) => {
          useBackfillStore.getState().setProgress(event.payload);
        },
      );
    })();

    return () => {
      unlisten?.();
    };
  }, []);
}
