/**
 * Zustand stores for UI state management.
 *
 * Stores:
 *  - useSidebarStore: sidebar collapsed/expanded state + expandedSpaceIds (Phase 10)
 *  - useCommandPaletteStore: command palette open/close
 *  - useIndexingStore: background indexing progress
 *  - useOnboardingStore: onboarding completion (persisted to localStorage)
 */

import { create } from "zustand";
import { persist } from "zustand/middleware";
import type { SpaceLabelingProgress } from "./types";

// --- Sidebar Store -----------------------------------------------------------
// Phase 10 Plan 02: expandedSpaceIds is session-only (no persist middleware).
// Per D-13 and UI-SPEC §Data Type Extensions: resets to empty Set on app restart.
// Follows useAiBannerStore and useSpaceLabelingStore no-persist convention.
// DO NOT wrap this store in persist(...).

interface SidebarState {
  isCollapsed: boolean;
  toggle: () => void;
  setCollapsed: (collapsed: boolean) => void;
  /** Phase 10: Set of space IDs whose sub-space list is expanded in the sidebar. D-13. */
  expandedSpaceIds: Set<string>;
  /** Phase 10: Toggle a space's expanded state. Idempotent — second call collapses it. D-13. */
  toggleSpaceExpanded: (spaceId: string) => void;
  /** Phase 10: Returns true if the given space is currently expanded in the sidebar. D-13. */
  isSpaceExpanded: (spaceId: string) => boolean;
}

export const useSidebarStore = create<SidebarState>((set, get) => ({
  isCollapsed: false,
  toggle: () => set((s) => ({ isCollapsed: !s.isCollapsed })),
  setCollapsed: (collapsed: boolean) => set({ isCollapsed: collapsed }),
  expandedSpaceIds: new Set<string>(),
  toggleSpaceExpanded: (spaceId) =>
    set((s) => {
      const next = new Set(s.expandedSpaceIds);
      if (next.has(spaceId)) {
        next.delete(spaceId);
      } else {
        next.add(spaceId);
      }
      return { expandedSpaceIds: next };
    }),
  isSpaceExpanded: (spaceId) => get().expandedSpaceIds.has(spaceId),
}));

// --- Command Palette Store ---------------------------------------------------

interface CommandPaletteState {
  isOpen: boolean;
  open: () => void;
  close: () => void;
  toggle: () => void;
}

export const useCommandPaletteStore = create<CommandPaletteState>((set) => ({
  isOpen: false,
  open: () => set({ isOpen: true }),
  close: () => set({ isOpen: false }),
  toggle: () => set((s) => ({ isOpen: !s.isOpen })),
}));

// --- Indexing Store ----------------------------------------------------------

interface IndexingState {
  isIndexing: boolean;
  currentFile: string;
  filesProcessed: number;
  totalFiles: number;
  setProgress: (progress: {
    currentFile?: string;
    filesProcessed?: number;
    totalFiles?: number;
    isIndexing?: boolean;
  }) => void;
  reset: () => void;
}

export const useIndexingStore = create<IndexingState>((set) => ({
  isIndexing: false,
  currentFile: "",
  filesProcessed: 0,
  totalFiles: 0,
  setProgress: (progress) =>
    set((s) => ({
      isIndexing: progress.isIndexing ?? s.isIndexing,
      currentFile: progress.currentFile ?? s.currentFile,
      filesProcessed: progress.filesProcessed ?? s.filesProcessed,
      totalFiles: progress.totalFiles ?? s.totalFiles,
    })),
  reset: () =>
    set({
      isIndexing: false,
      currentFile: "",
      filesProcessed: 0,
      totalFiles: 0,
    }),
}));

// --- Backfill Store (Plan 06-07, extended Plan 08-07) ------------------------

interface BackfillState {
  status: "idle" | "running" | "complete" | "error";
  processed: number;
  total: number;
  error: string | null;
  /**
   * ETA in seconds for the current backfill run (Pass-2 latency × remaining docs).
   * Non-null when two-pass extraction is active (LLM available).
   * Null when running Pass-1-only (no active provider) — used to detect mode in BackfillIndicator.
   * Plan 08-07 addition (D-25).
   */
  etaSeconds: number | null;
  /**
   * Number of documents that fell back to Pass-1-only after Pass-2 failures.
   * Set when status transitions to "complete". Used for D-29 completion toast.
   * Plan 08-07 addition (D-29).
   */
  fallbacks: number | null;
  setProgress: (p: Partial<Omit<BackfillState, "setProgress" | "reset">>) => void;
  reset: () => void;
}

export const useBackfillStore = create<BackfillState>((set) => ({
  status: "idle",
  processed: 0,
  total: 0,
  error: null,
  etaSeconds: null,
  fallbacks: null,
  setProgress: (p) =>
    set((s) => ({ ...s, ...p })),
  reset: () =>
    set({ status: "idle", processed: 0, total: 0, error: null, etaSeconds: null, fallbacks: null }),
}));

// --- Onboarding Store (persisted) --------------------------------------------

interface OnboardingState {
  isCompleted: boolean;
  setCompleted: (completed: boolean) => void;
  reset: () => void;
}

export const useOnboardingStore = create<OnboardingState>()(
  persist(
    (set) => ({
      isCompleted: false,
      setCompleted: (completed: boolean) => set({ isCompleted: completed }),
      reset: () => set({ isCompleted: false }),
    }),
    {
      name: "cortex-onboarding",
    },
  ),
);

// === Phase 7: AI Banner Store (session-only, NO persist middleware) ===
// Banner dismissed state resets on each app launch — returns until provider connected (D-15).
// DO NOT wrap in persist(...). The absence of persist is the contract enforced by stores.test.ts.

interface AiBannerState {
  isDismissed: boolean;
  dismiss: () => void;
}

export const useAiBannerStore = create<AiBannerState>((set) => ({
  isDismissed: false,
  dismiss: () => set({ isDismissed: true }),
}));

// === Phase 9 Plan 05: Space Labeling Store (session-only, NO persist middleware) ===
// Tracks in-progress LLM space labeling batch. Resets on each app launch (session-scoped).
// DO NOT wrap in persist(...) — matches useAiBannerStore no-persist convention.
// Consumed by useSpaceLabelingProgress (event listener) and SpaceCard shimmer (Plan 06).

export interface SpaceLabelingState {
  isActive: boolean;
  processed: number;
  total: number;
  status: "idle" | "labeling" | "complete" | "error";
  lastError?: string;
  /** Set of space IDs currently being labeled by the LLM (D-14 per-space shimmer). */
  generatingSpaceIds: Set<string>;
  setProgress: (p: SpaceLabelingProgress) => void;
  clear: () => void;
}

export const useSpaceLabelingStore = create<SpaceLabelingState>((set) => ({
  isActive: false,
  processed: 0,
  total: 0,
  status: "idle",
  lastError: undefined,
  generatingSpaceIds: new Set<string>(),
  setProgress: (p) =>
    set((s) => {
      const next = new Set(s.generatingSpaceIds);
      if (p.spaceId) {
        if (p.status === "labeling") {
          next.add(p.spaceId);
        } else {
          // "complete" or "error" — remove from generating set
          next.delete(p.spaceId);
        }
      }
      // WR-05: derive isActive from generatingSpaceIds set size, not from
      // the current event's status. A single "complete" event from a sub-cluster
      // mid-batch would previously set isActive=false even while other spaces
      // (top-level or sub) were still being labeled, causing UI flicker.
      // isActive=true as long as ANY space is still in-flight.
      const isActive = next.size > 0 || p.status === "labeling";
      return {
        isActive,
        processed: p.processed,
        total: p.total,
        status: p.status,
        lastError: p.error,
        generatingSpaceIds: next,
      };
    }),
  clear: () =>
    set({
      isActive: false,
      processed: 0,
      total: 0,
      status: "idle",
      lastError: undefined,
      generatingSpaceIds: new Set<string>(),
    }),
}));
