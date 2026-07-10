/**
 * BackfillIndicator — TopBar chip showing NER backfill progress.
 *
 * - idle: renders nothing
 * - running (two-pass): chip "Extracting entities X/Y" + tooltip "Two-pass entity extraction"
 * - running (Pass-1-only): chip "Extracting entities X/Y" + tooltip "Pattern extraction (Pass 1)"
 * - complete: "Done extracting entities" for 3s then resets; fires D-29 toast if fallbacks > 0
 * - error: AlertCircle + error tooltip; click to dismiss
 *
 * Two-pass detection: etaSeconds != null && etaSeconds > 0 (LLM latency is tracked when
 * a provider is active). Pass-1-only mode has etaSeconds=null (deterministic, no LLM wait).
 *
 * Plan 06-07 Task 2 + Plan 08-07 Task 2 (copy variants + completion toast D-29)
 *
 * NOTE: Existing px-2.5 spacing preserved per UI-SPEC §Spacing exception (pre-existing deviation).
 */

import { useEffect, useRef } from "react";
import { Brain, AlertCircle } from "lucide-react";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { useBackfillStore } from "@/lib/stores";
import { toast } from "sonner";

export function BackfillIndicator() {
  const { status, processed, total, error, etaSeconds, fallbacks, reset } =
    useBackfillStore();

  // Track previous status to detect the "idle → complete" transition for the D-29 toast.
  const prevStatusRef = useRef<string>("idle");

  // D-29 completion toast + auto-dismiss after 3 seconds on complete
  useEffect(() => {
    if (status !== "complete") {
      prevStatusRef.current = status;
      return;
    }

    // Fire D-29 warning toast only when some docs fell back to Pass-1 only
    if (fallbacks != null && fallbacks > 0 && prevStatusRef.current !== "complete") {
      toast.warning(
        `Backfill complete. ${fallbacks} of ${total} docs used pattern extraction only. Retry after network is healthy.`,
        { duration: 8000 },
      );
    }

    prevStatusRef.current = "complete";

    const timer = setTimeout(() => {
      reset();
    }, 3000);
    return () => clearTimeout(timer);
  }, [status, fallbacks, total, reset]);

  if (status === "idle") {
    return null;
  }

  if (status === "running") {
    // Two-pass mode: etaSeconds is non-null (LLM latency is being tracked)
    const isTwoPass = etaSeconds != null && etaSeconds > 0;

    return (
      <Tooltip>
        <TooltipTrigger asChild>
          <div
            className="flex items-center gap-2 rounded-md bg-accent-primary/10 px-2.5 py-1.5 text-xs text-accent-primary"
            aria-live="polite"
          >
            <Brain size={14} className="animate-pulse flex-shrink-0" />
            <span className="hidden sm:inline">
              Extracting entities{" "}
              <span className="tabular-nums">
                {processed}/{total}
              </span>
            </span>
          </div>
        </TooltipTrigger>
        <TooltipContent>
          {isTwoPass ? (
            <>
              <p className="text-xs">Two-pass entity extraction</p>
              <p className="text-xs text-muted-foreground">
                {processed} of {total} docs — Pass 1 complete, Pass 2 in
                progress (ETA {etaSeconds}s)
              </p>
            </>
          ) : (
            <>
              <p className="text-xs">Pattern extraction (Pass 1)</p>
              <p className="text-xs text-muted-foreground">
                {processed} of {total} docs — AI unavailable, extracting
                dates/amounts/IDs only
              </p>
            </>
          )}
        </TooltipContent>
      </Tooltip>
    );
  }

  if (status === "complete") {
    return (
      <div
        className="flex items-center gap-2 rounded-md bg-accent-primary/10 px-2.5 py-1.5 text-xs text-accent-primary"
        aria-live="polite"
      >
        <Brain size={14} className="flex-shrink-0" />
        <span className="hidden sm:inline">Done extracting entities</span>
      </div>
    );
  }

  // status === "error"
  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <button
          type="button"
          onClick={() => reset()}
          className="flex items-center gap-2 rounded-md bg-red-400/10 px-2.5 py-1.5 text-xs text-red-400 hover:bg-red-400/20 transition-colors"
          aria-label="Entity extraction failed — click to dismiss"
        >
          <AlertCircle size={14} className="flex-shrink-0" />
          <span className="hidden sm:inline">Extraction failed</span>
        </button>
      </TooltipTrigger>
      <TooltipContent>
        <p className="text-xs">Entity extraction failed</p>
        {error && <p className="text-xs text-muted-foreground">{error}</p>}
      </TooltipContent>
    </Tooltip>
  );
}
