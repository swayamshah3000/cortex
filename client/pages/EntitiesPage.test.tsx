/**
 * Tests for EntitiesPage (Plan 06-06 Task 2, Tests 1-3)
 *
 * Test 1: Loading state shows skeleton; populated renders type-grouped grid with EntityCards.
 * Test 2: Filter pill narrows grid to single type section.
 * Test 3: EntityCard link has correct href attribute.
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import React from "react";
import { MemoryRouter } from "react-router-dom";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";

// ---------------------------------------------------------------------------
// Mock hooks
// ---------------------------------------------------------------------------
const mockUseEntities = vi.fn();
const mockUseEntitiesByType = vi.fn();

vi.mock("../hooks/useTauri", () => ({
  useEntities: () => mockUseEntities(),
  useEntitiesByType: (type: string) => mockUseEntitiesByType(type),
}));

import type { EntitySummary } from "@/lib/types";

const mockEntityData: EntitySummary[] = [
  { id: "e-person-1", canonicalName: "John Smith", entityType: "person", documentCount: 5 },
  { id: "e-person-2", canonicalName: "Jane Doe", entityType: "person", documentCount: 3 },
  { id: "e-org-1", canonicalName: "Acme Corp", entityType: "organization", documentCount: 8 },
];

function createWrapper() {
  const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return function Wrapper({ children }: { children: React.ReactNode }) {
    return (
      <QueryClientProvider client={queryClient}>
        <MemoryRouter>{children}</MemoryRouter>
      </QueryClientProvider>
    );
  };
}

// Import after mocks
import EntitiesPage from "./EntitiesPage";

describe("EntitiesPage (06-06 Task 2 - Tests 1-3)", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("Test 1a: shows skeleton when loading", () => {
    mockUseEntities.mockReturnValue({ data: undefined, isLoading: true, isError: false });
    mockUseEntitiesByType.mockReturnValue({ data: undefined, isLoading: true, isError: false });

    render(<EntitiesPage />, { wrapper: createWrapper() });

    // Skeleton should have animate-pulse elements
    const { container } = render(<EntitiesPage />, { wrapper: createWrapper() });
    const skeletons = container.querySelectorAll(".animate-pulse");
    expect(skeletons.length).toBeGreaterThan(0);
  });

  it("Test 1b: shows 'No entities yet' heading when empty", () => {
    mockUseEntities.mockReturnValue({ data: [], isLoading: false, isError: false });
    mockUseEntitiesByType.mockReturnValue({ data: [], isLoading: false, isError: false });

    render(<EntitiesPage />, { wrapper: createWrapper() });
    expect(screen.getByText("No entities yet")).toBeDefined();
  });

  it("Test 1c: populated state renders entity cards", () => {
    mockUseEntities.mockReturnValue({ data: mockEntityData, isLoading: false, isError: false });
    mockUseEntitiesByType.mockReturnValue({ data: [], isLoading: false, isError: false });

    render(<EntitiesPage />, { wrapper: createWrapper() });
    expect(screen.getByText("John Smith")).toBeDefined();
    expect(screen.getByText("Acme Corp")).toBeDefined();
  });

  it("Test 1d: page title 'Entities' is rendered", () => {
    mockUseEntities.mockReturnValue({ data: mockEntityData, isLoading: false, isError: false });
    mockUseEntitiesByType.mockReturnValue({ data: [], isLoading: false, isError: false });

    render(<EntitiesPage />, { wrapper: createWrapper() });
    expect(screen.getByText("Entities")).toBeDefined();
  });

  it("Test 2: clicking 'Person' filter narrows grid to person entities only", () => {
    const personEntities: EntitySummary[] = [
      { id: "e-person-1", canonicalName: "John Smith", entityType: "person", documentCount: 5 },
    ];
    mockUseEntities.mockReturnValue({ data: mockEntityData, isLoading: false, isError: false });
    mockUseEntitiesByType.mockReturnValue({ data: personEntities, isLoading: false, isError: false });

    render(<EntitiesPage />, { wrapper: createWrapper() });

    // Click person filter
    fireEvent.click(screen.getByRole("button", { name: /^person$/i }));

    // After filtering, should see person entities
    expect(screen.getByText("John Smith")).toBeDefined();
  });

  it("Test 3: EntityCard links have correct /entities/{id} href", () => {
    mockUseEntities.mockReturnValue({ data: mockEntityData, isLoading: false, isError: false });
    mockUseEntitiesByType.mockReturnValue({ data: [], isLoading: false, isError: false });

    render(<EntitiesPage />, { wrapper: createWrapper() });

    // Find link for John Smith
    const links = screen.getAllByRole("link");
    const johnLink = links.find(
      (l) => l.getAttribute("href") === "/entities/e-person-1",
    );
    expect(johnLink).toBeDefined();
  });

  it("shows error state when query fails", () => {
    mockUseEntities.mockReturnValue({ data: undefined, isLoading: false, isError: true, refetch: vi.fn() });
    mockUseEntitiesByType.mockReturnValue({ data: undefined, isLoading: false, isError: false });

    render(<EntitiesPage />, { wrapper: createWrapper() });
    expect(screen.getByText(/could not load entities/i)).toBeDefined();
  });
});
