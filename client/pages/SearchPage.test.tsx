/**
 * Tests for SearchPage URL-driven entity filters (Phase 11 Plan 07).
 *
 * Covers must_have truths from 11-07-PLAN.md:
 * 1. ?entity=Person:Bob renders one EntityFilterPill with text "Person: Bob"
 * 2. Clicking the pill's X removes the entity param from the URL
 * 3. Save button disabled when both query and rawEntityParams are empty
 * 4. Save button enabled when ?entity=Person:Bob is present (even without query)
 *
 * Also covers T-11-22 threat: malformed ?entity= param (no colon) is discarded.
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import React from "react";
import { MemoryRouter, Routes, Route, useLocation } from "react-router-dom";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";

// ---------------------------------------------------------------------------
// Mock functions — defined at module scope (Bun-compatible, no vi.hoisted needed)
// ---------------------------------------------------------------------------
// eslint-disable-next-line @typescript-eslint/no-explicit-any
const mockUseDocumentSearch = vi.fn<any>(() => ({ data: undefined, isLoading: false, isFetching: false }));
// eslint-disable-next-line @typescript-eslint/no-explicit-any
const mockUseSpaces = vi.fn<any>(() => ({ data: [] }));
// eslint-disable-next-line @typescript-eslint/no-explicit-any
const mockUseRecordSearchClick = vi.fn<any>(() => ({ mutate: vi.fn() }));
// eslint-disable-next-line @typescript-eslint/no-explicit-any
const mockUseTopics = vi.fn<any>(() => ({ data: [] }));
// eslint-disable-next-line @typescript-eslint/no-explicit-any
const mockMutateAsync = vi.fn<any>().mockResolvedValue({ id: "ss-1", name: "Test", filters: {}, query: "", createdAt: new Date().toISOString(), docCountCache: 0 });

vi.mock("../hooks/useTauri", () => ({
  // Wrapper functions delay variable access until factory runs (after const declarations are live).
  useDocumentSearch: (q: unknown, f: unknown) => mockUseDocumentSearch(q, f),
  useSpaces: () => mockUseSpaces(),
  useRecordSearchClick: () => mockUseRecordSearchClick(),
  useTags: () => ({ data: [] }),
  useTopics: () => mockUseTopics(),
  useSaveSearch: () => ({ mutateAsync: mockMutateAsync, isPending: false }),
}));

vi.mock("sonner", () => ({
  toast: { success: vi.fn(), error: vi.fn() },
  Toaster: () => null,
}));

vi.mock("../lib/tauri", () => ({
  isTauri: vi.fn(() => false),
  tauriInvoke: vi.fn(),
}));

// Import after mocks to ensure they are in place
import SearchPage from "./SearchPage";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------
function makeQC() {
  return new QueryClient({ defaultOptions: { queries: { retry: false } } });
}

function renderSearchPage(initialUrl = "/search") {
  const qc = makeQC();
  return render(
    <QueryClientProvider client={qc}>
      <MemoryRouter initialEntries={[initialUrl]}>
        <Routes>
          <Route path="/search" element={<SearchPage />} />
        </Routes>
      </MemoryRouter>
    </QueryClientProvider>,
  );
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("SearchPage — entity filter pills from URL params", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockUseDocumentSearch.mockReturnValue({ data: undefined, isLoading: false, isFetching: false });
    mockUseSpaces.mockReturnValue({ data: [] });
    mockUseTopics.mockReturnValue({ data: [] });
  });

  it("renders one EntityFilterPill with 'Person: Bob' when ?entity=Person:Bob is in URL", () => {
    renderSearchPage("/search?entity=Person:Bob");
    // EntityFilterBar renders "Filtering by:" label when filters are active
    expect(screen.getByText("Filtering by:")).toBeDefined();
    // The pill displays "cls: value" format
    expect(screen.getByText("Person: Bob")).toBeDefined();
  });

  it("renders two pills for two entity params", () => {
    renderSearchPage("/search?entity=Person:Bob&entity=Location:London");
    expect(screen.getByText("Filtering by:")).toBeDefined();
    expect(screen.getByText("Person: Bob")).toBeDefined();
    expect(screen.getByText("Location: London")).toBeDefined();
  });

  it("discards malformed ?entity= param with no colon (T-11-22)", () => {
    // "INVALID" has no colon — should be silently discarded, bar not shown
    renderSearchPage("/search?entity=INVALID");
    expect(screen.queryByText("Filtering by:")).toBeNull();
  });

  it("renders 'Clear all' button when multiple entity params are present", () => {
    renderSearchPage("/search?entity=Person:Bob&entity=Location:London");
    expect(screen.getByText("Clear all")).toBeDefined();
  });

  it("does NOT render 'Clear all' when only one entity param is present", () => {
    renderSearchPage("/search?entity=Person:Bob");
    expect(screen.getByText("Filtering by:")).toBeDefined();
    expect(screen.queryByText("Clear all")).toBeNull();
  });
});

describe("SearchPage — Save button enabled/disabled state", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockUseDocumentSearch.mockReturnValue({ data: undefined, isLoading: false, isFetching: false });
    mockUseSpaces.mockReturnValue({ data: [] });
    mockUseTopics.mockReturnValue({ data: [] });
  });

  it("Save button is disabled when both query and rawEntityParams are empty", () => {
    renderSearchPage("/search");
    const saveBtn = screen.getByRole("button", { name: /save this search/i });
    expect(saveBtn).toHaveProperty("disabled", true);
  });

  it("Save button is enabled when ?entity=Person:Bob is in URL (entity-only filter saveable per D-09)", () => {
    renderSearchPage("/search?entity=Person:Bob");
    const saveBtn = screen.getByRole("button", { name: /save this search/i });
    expect(saveBtn).toHaveProperty("disabled", false);
  });
});

describe("SearchPage — Remove entity pill from URL", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockUseDocumentSearch.mockReturnValue({ data: undefined, isLoading: false, isFetching: false });
    mockUseSpaces.mockReturnValue({ data: [] });
    mockUseTopics.mockReturnValue({ data: [] });
  });

  /**
   * LocationSpy: reads the current search string via useLocation.
   * Lets us assert URL changes without direct router access.
   */
  function LocationSpy() {
    const loc = useLocation();
    return <div data-testid="location-search">{loc.search}</div>;
  }

  it("clicking the X on an EntityFilterPill removes that entity param from the URL", async () => {
    const qc = makeQC();

    render(
      <QueryClientProvider client={qc}>
        <MemoryRouter initialEntries={["/search?entity=Person:Bob"]}>
          <Routes>
            <Route path="/search" element={<SearchPage />} />
          </Routes>
          {/* Spy reads current URL search string */}
          <LocationSpy />
        </MemoryRouter>
      </QueryClientProvider>,
    );

    // Pill should be present
    expect(screen.getByText("Person: Bob")).toBeDefined();

    // Find and click the remove button (aria-label from EntityFilterPill spec)
    const removeBtn = screen.getByRole("button", { name: /remove person: bob filter/i });
    fireEvent.click(removeBtn);

    // URL should no longer contain entity= param
    await waitFor(() => {
      const locationSearch = screen.getByTestId("location-search").textContent ?? "";
      expect(locationSearch).not.toContain("entity=");
    });
  });
});
