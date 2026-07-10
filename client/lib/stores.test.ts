/**
 * Tests for Zustand stores.
 *
 * Covers:
 *  - useBackfillStore (Plan 06-07 Task 2 - Tests 1-3)
 *  - useAiBannerStore (Plan 07-04 Task 1)
 *  - useSidebarStore Phase 10 hierarchical state (Plan 10-02 Task 2)
 */

import { describe, it, expect, beforeEach } from "vitest";
import { useBackfillStore } from "./stores";

describe("useBackfillStore (06-07 Task 2)", () => {
  beforeEach(() => {
    // Reset store state before each test
    useBackfillStore.getState().reset();
  });

  it("Test 1: initial state is idle with zeroed values", () => {
    const state = useBackfillStore.getState();
    expect(state.status).toBe("idle");
    expect(state.processed).toBe(0);
    expect(state.total).toBe(0);
    expect(state.error).toBeNull();
  });

  it("Test 2: setProgress({ status: 'running', processed: 25, total: 100 }) updates those fields", () => {
    const store = useBackfillStore.getState();
    store.setProgress({ status: "running", processed: 25, total: 100 });

    const updated = useBackfillStore.getState();
    expect(updated.status).toBe("running");
    expect(updated.processed).toBe(25);
    expect(updated.total).toBe(100);
  });

  it("Test 3: reset() returns store to initial state", () => {
    useBackfillStore.getState().setProgress({ status: "running", processed: 50, total: 100 });
    useBackfillStore.getState().reset();

    const state = useBackfillStore.getState();
    expect(state.status).toBe("idle");
    expect(state.processed).toBe(0);
    expect(state.total).toBe(0);
    expect(state.error).toBeNull();
  });

  it("setProgress with partial fields merges correctly", () => {
    useBackfillStore.getState().setProgress({ status: "running", processed: 10, total: 100 });
    useBackfillStore.getState().setProgress({ processed: 20 });

    const state = useBackfillStore.getState();
    expect(state.status).toBe("running"); // preserved from previous call
    expect(state.processed).toBe(20);
    expect(state.total).toBe(100);
  });

  it("setProgress with error updates error field", () => {
    useBackfillStore.getState().setProgress({ status: "error", error: "NER failed" });
    const state = useBackfillStore.getState();
    expect(state.status).toBe("error");
    expect(state.error).toBe("NER failed");
  });
});

// === Phase 7: useAiBannerStore (Plan 07-04 Task 1) ===

import { useAiBannerStore } from "./stores";

describe("useAiBannerStore", () => {
  it("starts with isDismissed = false", () => {
    useAiBannerStore.setState({ isDismissed: false });
    const initial = useAiBannerStore.getState();
    expect(initial.isDismissed).toBe(false);
  });

  it("dismiss() flips isDismissed to true", () => {
    useAiBannerStore.setState({ isDismissed: false });
    useAiBannerStore.getState().dismiss();
    expect(useAiBannerStore.getState().isDismissed).toBe(true);
  });

  it("does not auto-rehydrate dismissed state across new store consumer mounts", () => {
    // Zustand persist middleware rehydrates state on store creation from storage.
    // Without persist, programmatically resetting state (simulating app reload)
    // returns to the initial value.
    useAiBannerStore.getState().dismiss();
    expect(useAiBannerStore.getState().isDismissed).toBe(true);

    useAiBannerStore.setState({ isDismissed: false });
    expect(useAiBannerStore.getState().isDismissed).toBe(false);

    // Confirm no `.persist` API surface — only persist-wrapped stores expose it.
    const api = useAiBannerStore as unknown as { persist?: unknown };
    expect(api.persist).toBeUndefined();
  });
});

// === Phase 10 Plan 02: useSidebarStore hierarchical state (Task 2) ===

import { readFileSync } from "fs";
import path from "path";
import { useSidebarStore } from "./stores";

describe("useSidebarStore Phase 10 hierarchical state", () => {
  beforeEach(() => {
    // Reset store to clean initial state before each test.
    // Matches useAiBannerStore reset pattern used above.
    useSidebarStore.setState({ expandedSpaceIds: new Set<string>(), isCollapsed: false });
  });

  it("Test 1 (initial state): expandedSpaceIds is an empty Set", () => {
    const state = useSidebarStore.getState();
    expect(state.expandedSpaceIds).toBeInstanceOf(Set);
    expect(state.expandedSpaceIds.size).toBe(0);
  });

  it("Test 2 (toggle adds): toggleSpaceExpanded adds id; isSpaceExpanded returns true", () => {
    useSidebarStore.getState().toggleSpaceExpanded("space-a");
    const state = useSidebarStore.getState();
    expect(state.expandedSpaceIds.has("space-a")).toBe(true);
    expect(state.isSpaceExpanded("space-a")).toBe(true);
  });

  it("Test 3 (toggle removes): calling toggleSpaceExpanded twice removes the id; size returns to 0", () => {
    useSidebarStore.getState().toggleSpaceExpanded("space-a");
    useSidebarStore.getState().toggleSpaceExpanded("space-a");
    const state = useSidebarStore.getState();
    expect(state.expandedSpaceIds.size).toBe(0);
    expect(state.isSpaceExpanded("space-a")).toBe(false);
  });

  it("Test 4 (independence): toggling a then b leaves both present; toggling a removes only a", () => {
    useSidebarStore.getState().toggleSpaceExpanded("space-a");
    useSidebarStore.getState().toggleSpaceExpanded("space-b");
    expect(useSidebarStore.getState().expandedSpaceIds.size).toBe(2);

    useSidebarStore.getState().toggleSpaceExpanded("space-a");
    const final = useSidebarStore.getState();
    expect(final.expandedSpaceIds.has("space-a")).toBe(false);
    expect(final.expandedSpaceIds.has("space-b")).toBe(true);
    expect(final.expandedSpaceIds.size).toBe(1);
  });

  it("Test 5 (no-persist contract): useSidebarStore is not wrapped in persist middleware", () => {
    // Confirm no `.persist` API surface — only persist-wrapped stores expose it.
    // This matches the useAiBannerStore no-persist assertion pattern above (Plan 07-04).
    // Per D-13 and UI-SPEC §Data Type Extensions: expandedSpaceIds is session-only,
    // NOT persisted to localStorage. Reset to empty on app restart.
    const api = useSidebarStore as unknown as { persist?: unknown };
    expect(api.persist).toBeUndefined();

    // Secondary source-level assertion: verify the useSidebarStore declaration
    // does not contain a persist() call wrapper (complements the API surface check).
    const src = readFileSync(path.resolve(__dirname, "stores.ts"), "utf8");
    // Extract the useSidebarStore export block
    const useSidebarStoreDecl = src.match(/export const useSidebarStore[\s\S]*?\)\);\n/)?.[0] ?? "";
    expect(useSidebarStoreDecl).not.toMatch(/persist\s*\(/);
  });
});
