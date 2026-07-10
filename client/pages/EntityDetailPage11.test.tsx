/**
 * Tests for EntityDetailPage11 (Plan 11-08 Task 1)
 *
 * Test 1: Loading state — renders skeletons when isLoading=true
 * Test 2: Empty state — totalDocumentCount=0 renders the "No documents" heading + links
 * Test 3: Full state — totalDocumentCount=3, documents.length=3 renders 3 document Links + aliases section
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import React from "react";
import { MemoryRouter } from "react-router-dom";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { EntityPageData, CanonicalEntity, Document } from "@/lib/types";

// ---------------------------------------------------------------------------
// Mocks (hoisted so they can be referenced inside vi.mock factories)
// ---------------------------------------------------------------------------

const { mockUseEntityPageData, mockUseEntityRelations, mockToastError } = vi.hoisted(() => ({
  mockUseEntityPageData: vi.fn(),
  mockUseEntityRelations: vi.fn(),
  mockToastError: vi.fn(),
}));

vi.mock("../hooks/useTauri", () => ({
  useEntityPageData: (...args: unknown[]) => mockUseEntityPageData(...args),
  useEntityRelations: (...args: unknown[]) => mockUseEntityRelations(...args),
}));

// Mock EntityRelationsPanel to keep this test file focused on EntityDetailPage11's
// own rendering — the panel itself is covered by its own component tests.
vi.mock("../components/relations/EntityRelationsPanel", () => ({
  EntityRelationsPanel: () => null,
}));

// Mock EntityChip to simplify rendering in tests
vi.mock("../components/entities/EntityChip", () => ({
  EntityChip: ({ entity }: { entity: { value: string } }) => (
    <div data-testid="entity-chip">{entity.value}</div>
  ),
}));

// Mock sonner to avoid Toaster setup
vi.mock("sonner", () => ({
  toast: {
    error: mockToastError,
    success: vi.fn(),
  },
  Toaster: () => null,
}));

// ---------------------------------------------------------------------------
// Mock data
// ---------------------------------------------------------------------------

const mockCanonical: CanonicalEntity = {
  id: "entity-001",
  canonicalName: "Alex Doe",
  entityType: "Person",
  aliases: ["Alex Doe", "G. Shah", "Shah Alex"],
  documentCount: 3,
};

const mockDocs: Document[] = [
  {
    id: "doc-001",
    name: "Contract.pdf",
    path: "/home/user/Documents/Contract.pdf",
    docType: "pdf",
    size: 1024,
    createdAt: "2024-01-01T00:00:00Z",
    modifiedAt: "2024-01-02T00:00:00Z",
    spaceIds: [],
    tags: [],
    isFavorite: false,
    extractedEntities: [],
  },
  {
    id: "doc-002",
    name: "Invoice.pdf",
    path: "/home/user/Documents/Invoice.pdf",
    docType: "pdf",
    size: 2048,
    createdAt: "2024-02-01T00:00:00Z",
    modifiedAt: "2024-02-02T00:00:00Z",
    spaceIds: [],
    tags: [],
    isFavorite: false,
    extractedEntities: [],
  },
  {
    id: "doc-003",
    name: "Report.docx",
    path: "/home/user/Documents/Report.docx",
    docType: "docx",
    size: 4096,
    createdAt: "2024-03-01T00:00:00Z",
    modifiedAt: "2024-03-02T00:00:00Z",
    spaceIds: [],
    tags: [],
    isFavorite: false,
    extractedEntities: [],
  },
];

const mockFullEntityPageData: EntityPageData = {
  canonical: mockCanonical,
  documents: mockDocs,
  totalDocumentCount: 3,
  coOccurringEntities: [
    { class: "Organization", value: "Acme Corp", coDocCount: 2 },
    { class: "Location", value: "Mumbai", coDocCount: 1 },
  ],
  page: 0,
  pageSize: 20,
};

const mockEmptyEntityPageData: EntityPageData = {
  canonical: {
    ...mockCanonical,
    aliases: ["Alex Doe"],
    documentCount: 0,
  },
  documents: [],
  totalDocumentCount: 0,
  coOccurringEntities: [],
  page: 0,
  pageSize: 20,
};

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

function createWrapper(initialPath = "/entity/Person/Alex%20Shah") {
  const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return function Wrapper({ children }: { children: React.ReactNode }) {
    return (
      <QueryClientProvider client={queryClient}>
        <MemoryRouter initialEntries={[initialPath]}>{children}</MemoryRouter>
      </QueryClientProvider>
    );
  };
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("EntityDetailPage11 (11-08 Task 1)", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockUseEntityRelations.mockReturnValue({
      data: { entity: mockCanonical, outgoing: [], incoming: [] },
      isLoading: false,
      isError: false,
    });
  });

  it("Test 1: renders loading skeletons when isLoading=true", async () => {
    mockUseEntityPageData.mockReturnValue({
      data: undefined,
      isLoading: true,
      isError: false,
      error: null,
      refetch: vi.fn(),
    });

    const { default: EntityDetailPage11 } = await import("./EntityDetailPage11");
    const { container } = render(<EntityDetailPage11 />, { wrapper: createWrapper() });

    // The loading state has data-testid="entity-page-loading"
    expect(screen.getByTestId("entity-page-loading")).toBeDefined();

    // Skeleton elements are present
    const skeletons = container.querySelectorAll('[class*="animate-pulse"]');
    expect(skeletons.length).toBeGreaterThan(0);
  });

  it("Test 2: renders empty state when totalDocumentCount === 0 with correct heading and links", async () => {
    mockUseEntityPageData.mockReturnValue({
      data: mockEmptyEntityPageData,
      isLoading: false,
      isError: false,
      error: null,
      refetch: vi.fn(),
    });

    const { default: EntityDetailPage11 } = await import("./EntityDetailPage11");
    render(<EntityDetailPage11 />, { wrapper: createWrapper() });

    // D-18 empty state heading — exact copy from UI-SPEC §7
    expect(screen.getByText(/No documents mention/)).toBeDefined();

    // Links to /watched and /settings
    const watchedLink = screen.getByRole("link", { name: /manage watched folders/i });
    expect(watchedLink).toBeDefined();
    expect(watchedLink.getAttribute("href")).toBe("/watched");

    const settingsLink = screen.getByRole("link", { name: /settings/i });
    expect(settingsLink).toBeDefined();
    expect(settingsLink.getAttribute("href")).toBe("/settings");

    // Empty state testid present
    expect(screen.getByTestId("entity-page-empty")).toBeDefined();
  });

  it("Test 3: full state renders 3 document Links and the aliases section", async () => {
    mockUseEntityPageData.mockReturnValue({
      data: mockFullEntityPageData,
      isLoading: false,
      isError: false,
      error: null,
      refetch: vi.fn(),
    });

    const { default: EntityDetailPage11 } = await import("./EntityDetailPage11");
    render(<EntityDetailPage11 />, { wrapper: createWrapper() });

    // Documents section: 3 document links (one per doc)
    expect(screen.getByText("Contract.pdf")).toBeDefined();
    expect(screen.getByText("Invoice.pdf")).toBeDefined();
    expect(screen.getByText("Report.docx")).toBeDefined();

    // Document links point to /document/:id
    const docLinks = screen.getAllByRole("link").filter((l) =>
      (l.getAttribute("href") ?? "").startsWith("/document/"),
    );
    expect(docLinks).toHaveLength(3);

    // Aliases section renders (3 aliases - 1 canonical = 2 real aliases)
    expect(screen.getByTestId("aliases-section")).toBeDefined();
    expect(screen.getByText(/also known as/i)).toBeDefined();

    // Alias badge shows "+2 aliases"
    expect(screen.getByText(/\+2 aliases/)).toBeDefined();
  });

  it("Test 4: co-occurring entities are rendered as EntityChips", async () => {
    mockUseEntityPageData.mockReturnValue({
      data: mockFullEntityPageData,
      isLoading: false,
      isError: false,
      error: null,
      refetch: vi.fn(),
    });

    const { default: EntityDetailPage11 } = await import("./EntityDetailPage11");
    render(<EntityDetailPage11 />, { wrapper: createWrapper() });

    const chips = screen.getAllByTestId("entity-chip");
    expect(chips.length).toBe(2);
    expect(screen.getByText("Acme Corp")).toBeDefined();
    expect(screen.getByText("Mumbai")).toBeDefined();
  });
});
