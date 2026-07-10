import { useEffect, useRef } from "react";
import { Outlet, useLocation, useNavigate } from "react-router-dom";
import { Sidebar } from "./Sidebar";
import { TopBar } from "./TopBar";
import { CommandPalette } from "./CommandPalette";
import {
  useOnboardingStore,
  useSidebarStore,
  useCommandPaletteStore,
  useIndexingStore,
  useAiBannerStore,
} from "@/lib/stores";
import { useWatchedFolders, useReclusterSpaces, useProviders, useStats, useSpaces } from "@/hooks/useTauri";
import { AiNoProviderBanner } from "./AiNoProviderBanner";
import { useQueryClient } from "@tanstack/react-query";
import { queryKeys } from "@/hooks/useTauri";
import { toast } from "sonner";
import { useBackfillProgress } from "@/hooks/useBackfillProgress";
import { useSpaceLabelingProgress } from "@/hooks/useSpaceLabelingProgress";
import { useTheme } from "next-themes";
import { cn } from "@/lib/utils";
import { isTauri } from "@/lib/tauri";

export function AppShell() {
  const location = useLocation();
  const navigate = useNavigate();
  const { isCompleted: onboardingCompleted } = useOnboardingStore();
  const { isCollapsed, toggle: toggleSidebar } = useSidebarStore();
  const { data: watchedFolders } = useWatchedFolders();
  const { theme, setTheme } = useTheme();
  const queryClient = useQueryClient();
  const recluster = useReclusterSpaces();

  // Pitfall 6 guard — banner only after onboarding completes AND no active provider AND not dismissed
  const { isDismissed: bannerDismissed } = useAiBannerStore();
  const { data: providers } = useProviders();
  const hasActiveProvider = providers?.some((p) => p.isActive && p.authenticated) ?? false;
  const showBanner = onboardingCompleted && !hasActiveProvider && !bannerDismissed;

  // Mount NER backfill progress listener (Plan 06-07 KG-05)
  useBackfillProgress();
  // Mount space labeling progress listener (Plan 09-06)
  useSpaceLabelingProgress();

  // Debounce query invalidations during heavy indexing runs.
  // Every "indexed" event previously fired 2 invalidateQueries → React Query refetch
  // storms (2000+ docs = 4000+ refetches) → main-thread UI freeze / blank pages.
  // Batch invalidations at max 1 Hz so backfill can keep firing events at full speed
  // but the UI only refreshes ~once a second regardless of doc count.
  const invalidateThrottleRef = useRef<{
    timer: ReturnType<typeof setTimeout> | null;
    lastFireMs: number;
  }>({ timer: null, lastFireMs: 0 });

  const scheduleInvalidate = () => {
    const now = Date.now();
    const state = invalidateThrottleRef.current;
    const MIN_INTERVAL_MS = 1000; // 1 Hz max — smooth without freezing
    const elapsed = now - state.lastFireMs;
    if (elapsed >= MIN_INTERVAL_MS) {
      state.lastFireMs = now;
      queryClient.invalidateQueries({ queryKey: queryKeys.recentDocuments });
      queryClient.invalidateQueries({ queryKey: queryKeys.stats });
      return;
    }
    if (state.timer !== null) return; // already scheduled
    state.timer = setTimeout(() => {
      state.timer = null;
      state.lastFireMs = Date.now();
      queryClient.invalidateQueries({ queryKey: queryKeys.recentDocuments });
      queryClient.invalidateQueries({ queryKey: queryKeys.stats });
    }, MIN_INTERVAL_MS - elapsed);
  };

  // Incremental recluster during large scans so users see Smart Spaces
  // form progressively instead of waiting for the entire scan to complete.
  // Trigger at first-space threshold (20 docs), then every 100 docs.
  // Phase 9's 20% Jaccard shift gate reuses cached labels when membership
  // hasn't changed, so incremental reclusters only cost LLM calls on
  // actually-changed spaces.
  const reclusterMilestoneRef = useRef<{
    lastCount: number;
    inFlight: boolean;
  }>({ lastCount: 0, inFlight: false });

  const maybeIncrementalRecluster = (docCount: number) => {
    const state = reclusterMilestoneRef.current;
    if (state.inFlight) return;
    const shouldFire =
      (state.lastCount < 20 && docCount >= 20) ||
      (state.lastCount >= 20 && docCount >= state.lastCount + 100);
    if (!shouldFire) return;
    state.inFlight = true;
    state.lastCount = docCount;
    recluster.mutate(undefined, {
      onSettled: () => {
        state.inFlight = false;
      },
      onSuccess: () => {
        queryClient.invalidateQueries({ queryKey: queryKeys.spaces });
      },
    });
  };

  // Bridge Tauri "index-progress" events to useIndexingStore (BREAK 2 fix)
  useEffect(() => {
    if (!isTauri()) return;
    let unlisten: (() => void) | undefined;

    (async () => {
      const { listen } = await import("@tauri-apps/api/event");
      unlisten = await listen<{
        filePath: string;
        status: "indexing" | "indexed" | "skipped" | "error" | "removed" | "complete";
        docId: string | null;
        error: string | null;
        folderId: string | null;
      }>("index-progress", (event) => {
        const { filePath, status } = event.payload;
        const store = useIndexingStore.getState();
        if (status === "indexing") {
          store.setProgress({
            isIndexing: true,
            currentFile: filePath,
          });
        } else if (status === "indexed" || status === "skipped") {
          // Count completed files. totalFiles unknown until backend emits it.
          const nextCount = store.filesProcessed + 1;
          store.setProgress({
            isIndexing: true,
            filesProcessed: nextCount,
            currentFile: filePath,
          });
          // Throttled — max 1 Hz regardless of doc-event rate.
          scheduleInvalidate();
          // Progressive Smart Spaces — first 20 docs, then every 100 docs.
          maybeIncrementalRecluster(nextCount);
        } else if (status === "complete") {
          store.setProgress({ isIndexing: false });
          // Auto-recluster Smart Spaces after a scan finishes (per phase 6 UAT fix)
          recluster.mutate(undefined, {
            onSuccess: (spaces) => {
              queryClient.invalidateQueries({ queryKey: queryKeys.spaces });
              if (spaces && spaces.length > 0) {
                toast.success(`Discovered ${spaces.length} Smart Space${spaces.length === 1 ? "" : "s"}`);
              }
            },
          });
          setTimeout(() => {
            useIndexingStore.getState().reset();
          }, 2000);
        } else if (status === "error") {
          store.setProgress({ isIndexing: false });
          setTimeout(() => {
            useIndexingStore.getState().reset();
          }, 2000);
        }
      });
    })();

    return () => {
      unlisten?.();
    };
  }, []);

  // Redirect to onboarding if not completed and no watched folders
  useEffect(() => {
    if (
      !onboardingCompleted &&
      watchedFolders !== undefined &&
      watchedFolders.length === 0 &&
      location.pathname !== "/onboarding"
    ) {
      navigate("/onboarding");
    }
  }, [onboardingCompleted, watchedFolders, location.pathname, navigate]);

  // Auto-recluster after boot when docs exist but SpaceManager is empty.
  // SpaceManager is in-memory only — restarts wipe cluster assignments even
  // though vectors + labels persist. Without this, user sees "0 spaces" after
  // every Rust restart until a scan completes or they manually recluster.
  // Fires exactly once per session when the condition first holds.
  const bootReclusterRef = useRef(false);
  const { data: bootStats } = useStats();
  const { data: bootSpaces } = useSpaces();
  useEffect(() => {
    if (bootReclusterRef.current) return;
    if (!bootStats || bootSpaces === undefined) return;
    if (bootStats.totalDocuments < 3) return; // need enough docs to cluster
    if (bootSpaces.length > 0) return; // already have spaces, nothing to do
    bootReclusterRef.current = true;
    recluster.mutate(undefined, {
      onSuccess: (spaces) => {
        queryClient.invalidateQueries({ queryKey: queryKeys.spaces });
        if (spaces && spaces.length > 0) {
          toast.success(
            `Restored ${spaces.length} Smart Space${spaces.length === 1 ? "" : "s"}`,
          );
        }
      },
    });
  }, [bootStats, bootSpaces, recluster, queryClient]);

  // Global keyboard shortcuts (UX-02)
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      const meta = e.metaKey || e.ctrlKey;

      // Cmd+K is handled by CommandPalette itself
      if (meta && e.key === "k") return;

      // Cmd+1 -> Dashboard
      if (meta && e.key === "1") {
        e.preventDefault();
        navigate("/");
        return;
      }
      // Cmd+2 -> Spaces
      if (meta && e.key === "2") {
        e.preventDefault();
        navigate("/spaces");
        return;
      }
      // Cmd+3 -> Search
      if (meta && e.key === "3") {
        e.preventDefault();
        navigate("/search");
        return;
      }
      // Cmd+, -> Settings
      if (meta && e.key === ",") {
        e.preventDefault();
        navigate("/settings");
        return;
      }
      // Cmd+D -> Toggle dark mode
      if (meta && e.key === "d") {
        e.preventDefault();
        setTheme(theme === "dark" ? "light" : "dark");
        return;
      }
      // Cmd+\ -> Toggle sidebar
      if (meta && e.key === "\\") {
        e.preventDefault();
        toggleSidebar();
        return;
      }
      // Escape -> Close command palette
      if (e.key === "Escape") {
        useCommandPaletteStore.getState().close();
        return;
      }
      // / -> Navigate to search and focus input (when not in input/textarea)
      if (
        e.key === "/" &&
        !meta &&
        !(e.target instanceof HTMLInputElement) &&
        !(e.target instanceof HTMLTextAreaElement)
      ) {
        e.preventDefault();
        navigate("/search");
        // Focus search input after navigation renders
        setTimeout(() => {
          const searchInput = document.querySelector<HTMLInputElement>('input[placeholder*="Search"]') ??
            document.querySelector<HTMLInputElement>('input[type="search"]') ??
            document.querySelector<HTMLInputElement>('.search-input');
          searchInput?.focus();
        }, 100);
        return;
      }
    };

    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [navigate, setTheme, theme, toggleSidebar]);

  return (
    <div className="h-screen bg-bg-primary">
      {/* Command Palette overlay */}
      <CommandPalette />

      {/* Sidebar - fixed positioned */}
      <Sidebar />

      {/* Main content area with margin to account for fixed sidebar */}
      <div
        className={cn(
          "h-screen flex flex-col transition-all duration-250",
          isCollapsed ? "ml-20" : "ml-60",
        )}
      >
        {/* AI no-provider banner — session-only, Pitfall 6 guard (shown only after onboarding).
            Mounted inside the offset column so the fixed Sidebar does not overlay it. */}
        {showBanner && <AiNoProviderBanner />}
        <TopBar />
        <main className="flex-1 overflow-auto">
          <div className="p-6 md:p-8">
            <Outlet />
          </div>
        </main>
      </div>
    </div>
  );
}
