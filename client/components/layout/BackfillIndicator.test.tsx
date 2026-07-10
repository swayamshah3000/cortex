/**
 * Tests for BackfillIndicator component (Plan 06-07 Task 2 - Tests 6-9)
 * Extended Plan 08-07 Task 2: completion toast + two-pass/pass-1-only tooltip variants
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, act } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import React from "react";
import { TooltipProvider } from "@/components/ui/tooltip";
import { useBackfillStore } from "@/lib/stores";

function withTooltip(ui: React.ReactElement) {
  return <TooltipProvider>{ui}</TooltipProvider>;
}

// --- Mock sonner toast -------------------------------------------------------

vi.mock("sonner", () => ({
  toast: {
    info: vi.fn(),
    success: vi.fn(),
    error: vi.fn(),
    warning: vi.fn(),
  },
}));

// --- Mock useBackfillStore ---------------------------------------------------
// Mock @/lib/stores with a real zustand store so setState() works in tests.
// Added etaSeconds + fallbacks fields for Plan 08-07.

vi.mock("@/lib/stores", () => {
  const { create } = require("zustand");
  const store = create(() => ({
    status: "idle",
    processed: 0,
    total: 0,
    error: null,
    etaSeconds: null,
    fallbacks: null,
    setProgress: vi.fn((p: Record<string, unknown>) =>
      store.setState((s: Record<string, unknown>) => ({ ...s, ...p })),
    ),
    reset: vi.fn(() =>
      store.setState({
        status: "idle",
        processed: 0,
        total: 0,
        error: null,
        etaSeconds: null,
        fallbacks: null,
      }),
    ),
  }));
  return {
    useBackfillStore: store,
    useSidebarStore: () => ({ isCollapsed: false, toggle: vi.fn() }),
    useCommandPaletteStore: () => ({ open: vi.fn(), isOpen: false }),
    useIndexingStore: () => ({ isIndexing: false }),
    useOnboardingStore: () => ({ isCompleted: true }),
    useAiBannerStore: () => ({ isDismissed: false, dismiss: vi.fn() }),
  };
});

import { toast } from "sonner";

describe("BackfillIndicator (06-07 Task 2)", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    // Reset store state
    useBackfillStore.setState({
      status: "idle",
      processed: 0,
      total: 0,
      error: null,
      etaSeconds: null,
      fallbacks: null,
    });
  });

  it("Test 6: renders nothing when status is idle", async () => {
    const { BackfillIndicator } = await import("./BackfillIndicator");
    const { container } = render(<BackfillIndicator />);
    expect(container.firstChild).toBeNull();
  });

  it("Test 7: renders chip with Extracting entities label when status=running", async () => {
    useBackfillStore.setState({ status: "running", processed: 25, total: 100 });
    const { BackfillIndicator } = await import("./BackfillIndicator");
    render(withTooltip(<BackfillIndicator />));
    expect(screen.getByText(/extracting entities/i)).toBeInTheDocument();
  });

  it("Test 7: shows processed/total count when running", async () => {
    useBackfillStore.setState({ status: "running", processed: 25, total: 100 });
    const { BackfillIndicator } = await import("./BackfillIndicator");
    render(withTooltip(<BackfillIndicator />));
    expect(screen.getByText("25/100")).toBeInTheDocument();
  });

  it("Test 8: shows Done extracting entities when status=complete", async () => {
    useBackfillStore.setState({ status: "complete", processed: 100, total: 100 });
    const { BackfillIndicator } = await import("./BackfillIndicator");
    render(withTooltip(<BackfillIndicator />));
    expect(screen.getByText(/done extracting entities/i)).toBeInTheDocument();
  });

  it("Test 9: shows error state with AlertCircle when status=error", async () => {
    useBackfillStore.setState({
      status: "error",
      processed: 10,
      total: 100,
      error: "NER failed",
    });
    const { BackfillIndicator } = await import("./BackfillIndicator");
    render(withTooltip(<BackfillIndicator />));
    // Chip should be visible in error state
    const chip = screen.getByRole("button");
    expect(chip).toBeInTheDocument();
  });

  it("Test 9: clicking error chip calls reset", async () => {
    const resetMock = vi.fn();
    useBackfillStore.setState({
      status: "error",
      processed: 10,
      total: 100,
      error: "NER failed",
      reset: resetMock,
    });
    const { BackfillIndicator } = await import("./BackfillIndicator");
    const user = userEvent.setup();
    render(withTooltip(<BackfillIndicator />));
    const chip = screen.getByRole("button");
    await user.click(chip);
    expect(resetMock).toHaveBeenCalled();
  });

  // --- Plan 08-07: D-29 completion-with-fallbacks toast ---

  it("D-29: fires warning toast on complete when fallbacks > 0", async () => {
    const { BackfillIndicator } = await import("./BackfillIndicator");
    const { rerender } = render(withTooltip(<BackfillIndicator />));

    // Transition: idle → running → complete with fallbacks
    act(() => {
      useBackfillStore.setState({ status: "running", processed: 8, total: 10 });
    });
    act(() => {
      useBackfillStore.setState({
        status: "complete",
        processed: 10,
        total: 10,
        fallbacks: 3,
      });
    });
    rerender(withTooltip(<BackfillIndicator />));

    expect(toast.warning).toHaveBeenCalledWith(
      expect.stringContaining("3 of 10 docs used pattern extraction only"),
      expect.objectContaining({ duration: 8000 }),
    );
  });

  it("D-29: does NOT fire toast on complete when fallbacks is 0 (silent success)", async () => {
    const { BackfillIndicator } = await import("./BackfillIndicator");
    const { rerender } = render(withTooltip(<BackfillIndicator />));

    act(() => {
      useBackfillStore.setState({ status: "running", processed: 10, total: 10 });
    });
    act(() => {
      useBackfillStore.setState({
        status: "complete",
        processed: 10,
        total: 10,
        fallbacks: 0,
      });
    });
    rerender(withTooltip(<BackfillIndicator />));

    expect(toast.warning).not.toHaveBeenCalled();
  });
});
