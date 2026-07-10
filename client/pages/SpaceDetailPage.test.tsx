/**
 * Tests for SpaceDetailPage (Plan 09-07):
 * 1. Save flow — Enter key calls useRenameSpace with correct payload + success toast
 * 2. Cancel edit — Escape reverts to original label, no mutation called
 * 3. Locked regenerate — SpaceLocked error fires info toast (not error toast)
 * 4. Clear override — visible when userLocked=true, calls useClearSpaceOverride on click
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import React from "react";
import { MemoryRouter } from "react-router-dom";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";

// ---------------------------------------------------------------------------
// Hoist mocks so they can be referenced in vi.mock factories
// ---------------------------------------------------------------------------
const {
  mockRenameMutate,
  mockClearMutate,
  mockRelabelMutate,
  mockToastSuccess,
  mockToastInfo,
  mockToastError,
} = vi.hoisted(() => ({
  mockRenameMutate: vi.fn(),
  mockClearMutate: vi.fn(),
  mockRelabelMutate: vi.fn(),
  mockToastSuccess: vi.fn(),
  mockToastInfo: vi.fn(),
  mockToastError: vi.fn(),
}));

// ---------------------------------------------------------------------------
// Module mocks
// ---------------------------------------------------------------------------

vi.mock("sonner", () => ({
  toast: {
    success: mockToastSuccess,
    info: mockToastInfo,
    error: mockToastError,
  },
  Toaster: () => null,
}));

// Space factory allows per-test override via mockSpacesReturnValue
let mockSpacesData: any[] = [];

vi.mock("../hooks/useTauri", () => ({
  useSpaces: () => ({ data: mockSpacesData, isLoading: false }),
  useSpaceDocuments: () => ({ data: [], isLoading: false }),
  useRenameSpace: () => ({ mutate: mockRenameMutate, isPending: false }),
  useClearSpaceOverride: () => ({ mutate: mockClearMutate, isPending: false }),
  useRelabelSpace: () => ({ mutate: mockRelabelMutate, isPending: false }),
}));

vi.mock("react-router-dom", async () => {
  const actual = await vi.importActual<typeof import("react-router-dom")>("react-router-dom");
  return {
    ...actual,
    useParams: () => ({ id: "space-1" }),
  };
});

vi.mock("../lib/icons", () => ({
  resolveIcon: () => () => React.createElement("span", null, "Icon"),
}));

vi.mock("../components/documents/DocumentRow", () => ({
  DocumentRow: ({ doc }: { doc: { name: string } }) =>
    React.createElement("div", { "data-testid": "doc-row" }, doc.name),
}));

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

import type { Space } from "@/lib/types";

const baseSpace: Space = {
  id: "space-1",
  name: "Property Tax",
  icon: "Home",
  color: "#6D28D9",
  documentCount: 5,
  lastUpdated: new Date(Date.now() - 60_000).toISOString(),
  subSpaces: [],
  sampleFiles: [],
};

function createWrapper() {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return function Wrapper({ children }: { children: React.ReactNode }) {
    return (
      <QueryClientProvider client={queryClient}>
        <MemoryRouter initialEntries={["/spaces/space-1"]}>
          {children}
        </MemoryRouter>
      </QueryClientProvider>
    );
  };
}

// Import after mocks
import SpaceDetailPage from "./SpaceDetailPage";

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("SpaceDetailPage (09-07) — inline edit + regenerate + lock", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockSpacesData = [{ ...baseSpace }];
  });

  it("Test 1: Save flow — Enter key calls useRenameSpace with correct payload and shows success toast", async () => {
    // Arrange: rename succeeds immediately
    mockRenameMutate.mockImplementation((_vars: unknown, callbacks: { onSuccess?: () => void; onError?: (e: unknown) => void }) => {
      callbacks?.onSuccess?.();
    });

    render(<SpaceDetailPage />, { wrapper: createWrapper() });

    // Click the Edit2 trigger to enter editing mode
    fireEvent.click(screen.getByRole("button", { name: /edit space label/i }));

    // The input should now be present
    const input = screen.getByRole("textbox");
    expect(input).toBeDefined();

    // Type new label
    fireEvent.change(input, { target: { value: "New Label" } });

    // Press Enter to save
    fireEvent.keyDown(input, { key: "Enter" });

    // Mutation called with correct payload
    expect(mockRenameMutate).toHaveBeenCalledWith(
      { spaceId: "space-1", label: "New Label" },
      expect.objectContaining({
        onSuccess: expect.any(Function),
        onError: expect.any(Function),
      }),
    );

    // Success toast with exact copy
    await waitFor(() => {
      expect(mockToastSuccess).toHaveBeenCalledWith(
        "Label saved and locked. Cortex won't overwrite this name.",
        { duration: 4000 },
      );
    });
  });

  it("Test 2: Cancel edit — Escape reverts input, restores h1 with original label, no mutation called", () => {
    render(<SpaceDetailPage />, { wrapper: createWrapper() });

    // Enter editing mode
    fireEvent.click(screen.getByRole("button", { name: /edit space label/i }));

    const input = screen.getByRole("textbox");

    // Type something new
    fireEvent.change(input, { target: { value: "Changed Name" } });

    // Press Escape to cancel
    fireEvent.keyDown(input, { key: "Escape" });

    // Input should be gone (view state restored)
    expect(screen.queryByRole("textbox")).toBeNull();

    // Original label should be visible (h1 and breadcrumb both show space.name)
    expect(screen.getAllByText("Property Tax").length).toBeGreaterThan(0);

    // No mutation should have been called
    expect(mockRenameMutate).not.toHaveBeenCalled();
  });

  it("Test 3: Locked regenerate — SpaceLocked error fires info toast with locked copy", async () => {
    // Arrange: relabel throws SpaceLocked error
    mockRelabelMutate.mockImplementation((
      _spaceId: string,
      callbacks: { onSuccess?: () => void; onError?: (e: unknown) => void },
    ) => {
      callbacks?.onError?.(new Error("SpaceLocked(space-1): space label is user-locked"));
    });

    render(<SpaceDetailPage />, { wrapper: createWrapper() });

    // Click Regenerate label button (visible in view state, not editing)
    fireEvent.click(screen.getByRole("button", { name: /regenerate label/i }));

    // Should fire info toast, not error toast
    await waitFor(() => {
      expect(mockToastInfo).toHaveBeenCalledWith(
        `Label for "Property Tax" is locked. Clear the override first to allow regeneration.`,
        { duration: 6000 },
      );
    });
    expect(mockToastError).not.toHaveBeenCalled();
  });

  it("Test 4: Clear override — button visible when userLocked=true, calls useClearSpaceOverride on click", async () => {
    // Arrange: space is user-locked
    mockSpacesData = [{ ...baseSpace, userLocked: true }];

    render(<SpaceDetailPage />, { wrapper: createWrapper() });

    // Lock icon should be visible in view state
    expect(screen.getByLabelText("Label locked by user")).toBeDefined();

    // Enter editing mode to access Clear override button
    fireEvent.click(screen.getByRole("button", { name: /edit space label/i }));

    // Clear override button should be present (space is userLocked)
    const clearBtn = screen.getByRole("button", { name: /clear override/i });
    expect(clearBtn).toBeDefined();

    // Click Clear override
    fireEvent.click(clearBtn);

    // useClearSpaceOverride.mutate called with the space id
    expect(mockClearMutate).toHaveBeenCalledWith(
      "space-1",
      expect.objectContaining({ onSuccess: expect.any(Function) }),
    );
  });
});
