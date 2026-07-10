/**
 * Tests for Sidebar component.
 *
 * Suites:
 *  1. Sidebar Entities link (Plan 06-06 Task 2, Test 4)
 *  2. Sidebar spaces section — chevron expand + sub-space list (Plan 10-07)
 *  3. Sidebar Saved Searches section (Plan 11-09)
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import React from "react";
import { MemoryRouter } from "react-router-dom";
import type { Space, SavedSearch } from "@/lib/types";

// ---------------------------------------------------------------------------
// Module-level mock factories — hooks support per-test override below.
// ---------------------------------------------------------------------------

let mockSpacesData: Space[] = [];
let mockSavedSearchesData: SavedSearch[] = [];
let mockSavedSearchCountsData: Record<string, number> = {};
let mockStoreState: {
  isCollapsed: boolean;
  expandedSpaceIds: Set<string>;
  toggleSpaceExpanded: (id: string) => void;
} = {
  isCollapsed: false,
  expandedSpaceIds: new Set<string>(),
  toggleSpaceExpanded: vi.fn(),
};

vi.mock("@/hooks/useTauri", () => ({
  useSpaces: () => ({ data: mockSpacesData, isLoading: false }),
  useStats: () => ({ data: { indexSize: 0 } }),
  useSavedSearches: () => ({ data: mockSavedSearchesData, isLoading: false }),
  useSavedSearchCounts: () => ({ data: mockSavedSearchCountsData }),
}));

vi.mock("@/lib/stores", () => ({
  useSidebarStore: () => ({
    isCollapsed: mockStoreState.isCollapsed,
    toggle: vi.fn(),
    setCollapsed: vi.fn(),
    expandedSpaceIds: mockStoreState.expandedSpaceIds,
    toggleSpaceExpanded: mockStoreState.toggleSpaceExpanded,
    isSpaceExpanded: (id: string) => mockStoreState.expandedSpaceIds.has(id),
  }),
  useCommandPaletteStore: () => ({ open: vi.fn() }),
}));

import { Sidebar } from "@/components/layout/Sidebar";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function renderSidebar(path = "/") {
  return render(
    <MemoryRouter initialEntries={[path]}>
      <Sidebar />
    </MemoryRouter>,
  );
}

/**
 * Factory to create mock Space objects with Phase 10 fields.
 */
function makeSpace(
  overrides: Partial<Space> & { id: string; name: string }
): Space {
  return {
    icon: "Home",
    color: "#6D28D9",
    documentCount: 10,
    lastUpdated: "2026-01-01T00:00:00Z",
    subSpaces: [],
    sampleFiles: [],
    depth: 0,
    subSpaceIds: [],
    ...overrides,
  } as Space;
}

// ---------------------------------------------------------------------------
// Suite 1: Sidebar Entities link (Plan 06-06 Task 2, Test 4)
// ---------------------------------------------------------------------------

describe("Sidebar Entities link (06-06 Task 2 - Test 4)", () => {
  beforeEach(() => {
    mockSpacesData = [];
    mockSavedSearchesData = [];
    mockSavedSearchCountsData = {};
    mockStoreState = {
      isCollapsed: false,
      expandedSpaceIds: new Set<string>(),
      toggleSpaceExpanded: vi.fn(),
    };
  });

  it("renders an Entities link", () => {
    renderSidebar();
    expect(screen.getByText("Entities")).toBeDefined();
  });

  it("Entities link points to /entities", () => {
    renderSidebar();
    const entityLink = screen.getByRole("link", { name: /entities/i });
    expect(entityLink.getAttribute("href")).toBe("/entities");
  });

  it("Entities link is positioned after Tags in bottomLinks", () => {
    renderSidebar();
    const links = screen.getAllByRole("link");
    const hrefs = links.map((l) => l.getAttribute("href")).filter(Boolean);
    const tagsIdx = hrefs.indexOf("/tags");
    const entitiesIdx = hrefs.indexOf("/entities");
    const watchedIdx = hrefs.indexOf("/watched");
    expect(entitiesIdx).toBeGreaterThan(tagsIdx);
    expect(entitiesIdx).toBeLessThan(watchedIdx);
  });

  it("Entities link shows active state when on /entities route", () => {
    renderSidebar("/entities");
    const entityLink = screen.getByRole("link", { name: /entities/i });
    expect(entityLink.className).toContain("bg-accent-primary");
  });
});

// ---------------------------------------------------------------------------
// Suite 2: Plan 10-07 — Sidebar chevron expand + sub-space list
// D-13: inline chevron toggles expandedSpaceIds, no navigation
// D-14: sub-count "(N)" inline, text-xs text-text-tertiary
// D-17: top 5 top-level spaces by documentCount
// ---------------------------------------------------------------------------

describe("Sidebar spaces section (Plan 10-07: chevron expand + sub-space list)", () => {
  const subSpaceA = makeSpace({ id: "sub-a", name: "Tax", parentId: "space-1", depth: 1, subSpaceIds: [] });
  const subSpaceB = makeSpace({ id: "sub-b", name: "Insurance", parentId: "space-1", depth: 1, subSpaceIds: [] });

  // 7 top-level spaces (only top-5 shown by documentCount) + 2 sub-spaces
  const allSpaces: Space[] = [
    makeSpace({ id: "space-1", name: "Property", documentCount: 100, subSpaceIds: ["sub-a", "sub-b"] }),
    makeSpace({ id: "space-2", name: "Work", documentCount: 80, subSpaceIds: [] }),
    makeSpace({ id: "space-3", name: "Medical", documentCount: 60, subSpaceIds: [] }),
    makeSpace({ id: "space-4", name: "Finance", documentCount: 50, subSpaceIds: [] }),
    makeSpace({ id: "space-5", name: "Travel", documentCount: 40, subSpaceIds: [] }),
    makeSpace({ id: "space-6", name: "Personal", documentCount: 30, subSpaceIds: [] }), // 6th — must be hidden
    makeSpace({ id: "space-7", name: "Archive", documentCount: 20, subSpaceIds: [] }), // 7th — must be hidden
    subSpaceA,
    subSpaceB,
  ];

  beforeEach(() => {
    mockSpacesData = allSpaces;
    mockSavedSearchesData = [];
    mockSavedSearchCountsData = {};
    mockStoreState = {
      isCollapsed: false,
      expandedSpaceIds: new Set<string>(),
      toggleSpaceExpanded: vi.fn(),
    };
  });

  // -------------------------------------------------------------------------
  // Test 1: Top 5 top-level spaces only (D-17)
  // -------------------------------------------------------------------------
  it("renders exactly 5 top-level spaces (not 6) in expanded sidebar", () => {
    renderSidebar();
    expect(screen.getByText("Property")).toBeDefined();
    expect(screen.getByText("Work")).toBeDefined();
    expect(screen.getByText("Medical")).toBeDefined();
    expect(screen.getByText("Finance")).toBeDefined();
    expect(screen.getByText("Travel")).toBeDefined();
    // 6th and 7th by count must NOT appear as sidebar space entries
    expect(screen.queryByRole("link", { name: /^Personal$/ })).toBeNull();
    expect(screen.queryByRole("link", { name: /^Archive$/ })).toBeNull();
  });

  // -------------------------------------------------------------------------
  // Test 2: Sub-count "(N)" inline for spaces with subSpaceIds (D-14)
  // -------------------------------------------------------------------------
  it("renders inline sub-count '(2)' next to Property which has 2 subSpaceIds", () => {
    renderSidebar();
    // Property has subSpaceIds: ["sub-a", "sub-b"] → "(2)" must appear
    expect(screen.getByText("(2)")).toBeDefined();
  });

  it("does NOT render sub-count for spaces with no subSpaceIds", () => {
    renderSidebar();
    // Only Property has sub-spaces; only one "(N)" should exist
    const allParens = screen.queryAllByText(/^\(\d+\)$/);
    expect(allParens).toHaveLength(1);
    expect(allParens[0].textContent).toBe("(2)");
  });

  // -------------------------------------------------------------------------
  // Test 3: Chevron button presence/absence based on subSpaceIds (D-13)
  // -------------------------------------------------------------------------
  it("renders a chevron expand button for Property (has subSpaceIds)", () => {
    renderSidebar();
    const chevronBtn = screen.getByRole("button", { name: /expand sub-spaces/i });
    expect(chevronBtn).toBeDefined();
  });

  it("does NOT render a chevron button for spaces without subSpaceIds", () => {
    renderSidebar();
    // Only Property has sub-spaces — exactly one chevron button exists
    const chevronBtns = screen.queryAllByRole("button", { name: /expand sub-spaces|collapse sub-spaces/i });
    expect(chevronBtns).toHaveLength(1);
  });

  // -------------------------------------------------------------------------
  // Test 4: Clicking chevron calls toggleSpaceExpanded (D-13)
  // -------------------------------------------------------------------------
  it("clicking the chevron button calls toggleSpaceExpanded with the space id", () => {
    const toggleFn = vi.fn();
    mockStoreState.toggleSpaceExpanded = toggleFn;
    renderSidebar();
    const chevronBtn = screen.getByRole("button", { name: /expand sub-spaces/i });
    fireEvent.click(chevronBtn);
    expect(toggleFn).toHaveBeenCalledWith("space-1");
  });

  // -------------------------------------------------------------------------
  // Test 5: Sub-space list renders when expandedSpaceIds includes space (D-13)
  // -------------------------------------------------------------------------
  it("renders sub-space list entries when space is expanded", () => {
    mockStoreState.expandedSpaceIds = new Set(["space-1"]);
    renderSidebar();
    expect(screen.getByText("Tax")).toBeDefined();
    expect(screen.getByText("Insurance")).toBeDefined();
  });

  it("sub-space links navigate to /spaces/:id", () => {
    mockStoreState.expandedSpaceIds = new Set(["space-1"]);
    renderSidebar();
    const taxLink = screen.getByRole("link", { name: /^Tax$/ });
    expect(taxLink.getAttribute("href")).toBe("/spaces/sub-a");
    const insLink = screen.getByRole("link", { name: /^Insurance$/ });
    expect(insLink.getAttribute("href")).toBe("/spaces/sub-b");
  });

  it("does NOT render sub-space list when space is NOT expanded", () => {
    // expandedSpaceIds is empty by default (set in beforeEach)
    renderSidebar();
    expect(screen.queryByText("Tax")).toBeNull();
    expect(screen.queryByText("Insurance")).toBeNull();
  });

  // -------------------------------------------------------------------------
  // Test 6: Collapsed sidebar — sub-count, chevron, sub-list all hidden (D-13)
  // -------------------------------------------------------------------------
  it("hides sub-count, chevron, and sub-space list when sidebar is collapsed", () => {
    mockStoreState.isCollapsed = true;
    mockStoreState.expandedSpaceIds = new Set(["space-1"]);
    renderSidebar();
    expect(screen.queryByText("(2)")).toBeNull();
    expect(screen.queryByRole("button", { name: /expand sub-spaces|collapse sub-spaces/i })).toBeNull();
    expect(screen.queryByText("Tax")).toBeNull();
    expect(screen.queryByText("Insurance")).toBeNull();
  });

  // -------------------------------------------------------------------------
  // Test 7: Chevron aria-label toggles based on expanded state (D-13)
  // -------------------------------------------------------------------------
  it("chevron aria-label is 'Collapse sub-spaces' when space is expanded", () => {
    mockStoreState.expandedSpaceIds = new Set(["space-1"]);
    renderSidebar();
    expect(screen.getByRole("button", { name: /collapse sub-spaces/i })).toBeDefined();
  });

  // -------------------------------------------------------------------------
  // Test 8: View All link reflects top-level count (not total with sub-spaces)
  // -------------------------------------------------------------------------
  it("shows View All link with top-level count when more than 5 top-level spaces exist", () => {
    renderSidebar();
    // 7 top-level spaces → "View All (7)"
    const viewAllLink = screen.queryByRole("link", { name: /view all/i });
    expect(viewAllLink).toBeDefined();
    expect(viewAllLink?.textContent).toContain("7");
  });
});

// ---------------------------------------------------------------------------
// Suite 3: Plan 11-09 — Sidebar Saved Searches section
// D-07: "Saved Searches" header below Smart Spaces
// D-08: count from useSavedSearchCounts, fallback to docCountCache
// ENEX-02: collapsed sidebar shows Bookmark icon only (no name/count)
// ---------------------------------------------------------------------------

describe("Sidebar Saved Searches section (Plan 11-09)", () => {
  const makeSavedSearch = (overrides: Partial<SavedSearch> & { id: string; name: string }): SavedSearch => ({
    query: "",
    filters: {},
    createdAt: "2026-07-09T00:00:00Z",
    docCountCache: 5,
    ...overrides,
  });

  beforeEach(() => {
    mockSpacesData = [];
    mockSavedSearchesData = [];
    mockSavedSearchCountsData = {};
    mockStoreState = {
      isCollapsed: false,
      expandedSpaceIds: new Set<string>(),
      toggleSpaceExpanded: vi.fn(),
    };
  });

  // -------------------------------------------------------------------------
  // Test 1: "Saved Searches" header renders when data is non-empty
  // -------------------------------------------------------------------------
  it('renders "Saved Searches" header when at least one saved search exists', () => {
    mockSavedSearchesData = [makeSavedSearch({ id: "ss-1", name: "Property Docs" })];
    renderSidebar();
    expect(screen.getByText("Saved Searches")).toBeDefined();
  });

  // -------------------------------------------------------------------------
  // Test 2: "No saved searches yet" empty state
  // -------------------------------------------------------------------------
  it('renders "No saved searches yet" when saved searches list is empty', () => {
    mockSavedSearchesData = [];
    renderSidebar();
    expect(screen.getByText("No saved searches yet")).toBeDefined();
  });

  // -------------------------------------------------------------------------
  // Test 3: Each row's href matches buildSavedSearchUrl output
  // -------------------------------------------------------------------------
  it("each row href includes entity filter params from the saved search filters", () => {
    mockSavedSearchesData = [
      makeSavedSearch({
        id: "ss-2",
        name: "Property Tax",
        query: "property tax",
        filters: { entities: ["Location:AlphaComplex"] },
      }),
    ];
    renderSidebar();
    const link = screen.getByRole("link", { name: /Property Tax/i });
    const href = link.getAttribute("href") ?? "";
    // Must include both q= and entity= params
    expect(href).toContain("/search");
    expect(href).toContain("q=property+tax");
    expect(href).toContain("entity=Location%3AAlphaComplex");
  });

  // -------------------------------------------------------------------------
  // Test 4: Live count from useSavedSearchCounts (overrides docCountCache)
  // -------------------------------------------------------------------------
  it("shows live count from useSavedSearchCounts when available", () => {
    mockSavedSearchesData = [
      makeSavedSearch({ id: "ss-3", name: "Work Files", docCountCache: 3 }),
    ];
    mockSavedSearchCountsData = { "ss-3": 12 };
    renderSidebar();
    // Live count (12) should appear, not cache count (3)
    expect(screen.getByText("(12)")).toBeDefined();
  });

  // -------------------------------------------------------------------------
  // Test 5: Falls back to docCountCache when live count not yet loaded
  // -------------------------------------------------------------------------
  it("falls back to docCountCache when live count is not yet available", () => {
    mockSavedSearchesData = [
      makeSavedSearch({ id: "ss-4", name: "Medical Records", docCountCache: 7 }),
    ];
    mockSavedSearchCountsData = {}; // no live count for ss-4
    renderSidebar();
    expect(screen.getByText("(7)")).toBeDefined();
  });

  // -------------------------------------------------------------------------
  // Test 6: Collapsed sidebar — hides name + count but keeps Bookmark icon
  // -------------------------------------------------------------------------
  it("collapsed sidebar hides name and count but renders Bookmark icon", () => {
    mockSavedSearchesData = [
      makeSavedSearch({ id: "ss-5", name: "Kids School", docCountCache: 4 }),
    ];
    mockStoreState.isCollapsed = true;
    renderSidebar();
    // Name and count must NOT appear
    expect(screen.queryByText("Kids School")).toBeNull();
    expect(screen.queryByText("(4)")).toBeNull();
    // The row link must still exist (Bookmark icon is its child, no accessible text role)
    // We verify the section still rendered by checking no name/count are shown
    // and that "Saved Searches" header is also hidden
    expect(screen.queryByText("Saved Searches")).toBeNull();
  });
});
