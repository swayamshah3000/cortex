/**
 * Tests for EntityDetailPage (Plan 06-07 Task 1 - Tests 1, 2, 6)
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import React from "react";
import { MemoryRouter } from "react-router-dom";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { CanonicalEntity, RelatedEntity, Document } from "@/lib/types";

const mockCanonical: CanonicalEntity = {
  id: "entity-001",
  canonicalName: "John Smith",
  entityType: "person",
  aliases: ["J. Smith", "Smith, John", "John Smith"],
  documentCount: 2,
};

const mockDocs: Document[] = [
  {
    id: "doc-001",
    name: "Contract.pdf",
    path: "/docs/Contract.pdf",
    docType: "pdf",
    size: 1024,
    createdAt: "2024-01-01T00:00:00Z",
    modifiedAt: "2024-01-02T00:00:00Z",
    spaceIds: [],
    tags: [],
    isFavorite: false,
    extractedEntities: [],
  },
];

const mockRelated: RelatedEntity[] = [
  {
    entity: {
      id: "entity-002",
      canonicalName: "Jane Doe",
      entityType: "person",
      documentCount: 3,
    },
    coOccurrenceCount: 2,
  },
];

// Mock useTauri hooks
vi.mock("@/hooks/useTauri", () => ({
  useEntity: vi.fn(() => ({ data: mockCanonical, isLoading: false, isError: false })),
  useEntityDocuments: vi.fn(() => ({ data: mockDocs, isLoading: false, isError: false })),
  useRelatedEntities: vi.fn(() => ({ data: mockRelated, isLoading: false, isError: false })),
  useRenameEntityCanonical: vi.fn(() => ({
    mutate: vi.fn(),
    isPending: false,
  })),
  useSplitEntityAlias: vi.fn(() => ({
    mutate: vi.fn(),
    isPending: false,
  })),
}));

vi.mock("react-router-dom", async () => {
  const actual = await vi.importActual("react-router-dom");
  return {
    ...actual,
    useParams: () => ({ id: "entity-001" }),
    useNavigate: () => vi.fn(),
  };
});

vi.mock("@/components/documents/DocumentRow", () => ({
  DocumentRow: ({ doc }: { doc: { name: string } }) => (
    <div data-testid="document-row">{doc.name}</div>
  ),
}));

function createWrapper() {
  const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return function Wrapper({ children }: { children: React.ReactNode }) {
    return (
      <QueryClientProvider client={queryClient}>
        <MemoryRouter initialEntries={["/entities/entity-001"]}>
          {children}
        </MemoryRouter>
      </QueryClientProvider>
    );
  };
}

describe("EntityDetailPage (06-07 Task 1)", () => {
  it("Test 1: renders header + aliases + documents + related sections", async () => {
    const Wrapper = createWrapper();
    const EntityDetailPage = (await import("./EntityDetailPage")).default;
    render(<EntityDetailPage />, { wrapper: Wrapper });

    // Header displays canonical name (appears in both header and breadcrumb)
    expect(screen.getAllByText("John Smith").length).toBeGreaterThanOrEqual(1);
    expect(screen.getByText(/aliases/i)).toBeInTheDocument();
    expect(screen.getByText(/documents mentioning this/i)).toBeInTheDocument();
    expect(screen.getByText(/related entities/i)).toBeInTheDocument();
  });

  it("Test 1: renders document rows from useEntityDocuments", async () => {
    const Wrapper = createWrapper();
    const EntityDetailPage = (await import("./EntityDetailPage")).default;
    render(<EntityDetailPage />, { wrapper: Wrapper });

    expect(screen.getByText("Contract.pdf")).toBeInTheDocument();
  });

  it("Test 1: renders related entity chips", async () => {
    const Wrapper = createWrapper();
    const EntityDetailPage = (await import("./EntityDetailPage")).default;
    render(<EntityDetailPage />, { wrapper: Wrapper });

    expect(screen.getByText("Jane Doe")).toBeInTheDocument();
  });
});
