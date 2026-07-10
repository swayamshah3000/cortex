/**
 * Tests for useBackfillProgress hook (Plan 06-07 Task 2 - Tests 4-5)
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { renderHook, waitFor, act } from "@testing-library/react";
import React from "react";
import { useBackfillStore } from "@/lib/stores";

// Mock @tauri-apps/api/event
const mockUnlisten = vi.fn();
const mockListen = vi.fn(() => Promise.resolve(mockUnlisten));

vi.mock("@tauri-apps/api/event", () => ({
  listen: mockListen,
}));

// Mock isTauri
let isTauriResult = true;
vi.mock("@/lib/tauri", () => ({
  isTauri: () => isTauriResult,
  tauriInvoke: vi.fn(),
}));

describe("useBackfillProgress (06-07 Task 2)", () => {
  beforeEach(() => {
    mockListen.mockClear();
    mockUnlisten.mockClear();
    useBackfillStore.getState().reset();
    isTauriResult = true;
  });

  it("Test 4: mounts a Tauri event listener for entity-backfill-progress", async () => {
    const { useBackfillProgress } = await import("./useBackfillProgress");
    const { unmount } = renderHook(() => useBackfillProgress());

    await waitFor(() => {
      expect(mockListen).toHaveBeenCalledWith(
        "entity-backfill-progress",
        expect.any(Function),
      );
    });

    unmount();
  });

  it("Test 4: calls unlisten on unmount", async () => {
    const { useBackfillProgress } = await import("./useBackfillProgress");
    const { unmount } = renderHook(() => useBackfillProgress());

    await waitFor(() => expect(mockListen).toHaveBeenCalled());

    unmount();
    // unlisten called on cleanup
    expect(mockUnlisten).toHaveBeenCalled();
  });

  it("Test 5: does NOT call listen when isTauri() returns false", async () => {
    isTauriResult = false;
    const { useBackfillProgress } = await import("./useBackfillProgress");
    renderHook(() => useBackfillProgress());

    // Wait a tick to let async effects settle
    await new Promise((r) => setTimeout(r, 50));
    expect(mockListen).not.toHaveBeenCalled();
  });

  it("Test 4: payload from listen writes to useBackfillStore via setProgress", async () => {
    let capturedCallback: ((event: { payload: unknown }) => void) | null = null;
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    (mockListen as any).mockImplementation((_event: unknown, cb: (e: { payload: unknown }) => void) => {
      capturedCallback = cb;
      return Promise.resolve(mockUnlisten);
    });

    const { useBackfillProgress } = await import("./useBackfillProgress");
    renderHook(() => useBackfillProgress());

    await waitFor(() => expect(capturedCallback).not.toBeNull());

    // Fire the event callback
    act(() => {
      capturedCallback!({
        payload: { processed: 50, total: 200, status: "running", error: null },
      });
    });

    const state = useBackfillStore.getState();
    expect(state.processed).toBe(50);
    expect(state.total).toBe(200);
    expect(state.status).toBe("running");
  });
});
