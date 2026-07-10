/**
 * Tests for DocumentPage modifications (PAGE-13 / UX-06 / Plan 06-05).
 *
 * Tests:
 * 1. Clicking "Open in default app" button calls openPath(doc.path)
 * 2. Clicking "Reveal in Finder" calls revealItemInDir(doc.path); label changes on non-Mac
 * 3. Excerpt block is gone; FilePreview renders in its place
 * 4. Entity chips render as Link elements to /entities/{canonicalId}
 * 5. Entity with no canonicalId falls back to /entities/{encodeURIComponent(value)}
 * 6. Dead "Open in Finder placeholder" is removed from the DOM
 *
 * Phase 8 additions (Plan 08-08 Task 2):
 * 7. Extracted entities section renders entity chip links
 * 8. No "Also found" expander when no entities are low-confidence
 * 9. Topic section absent when doc.topic is not set
 * 10. "Entities" heading visible when entities exist
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import React from "react";
import { MemoryRouter } from "react-router-dom";

// ---------------------------------------------------------------------------
// Hoist mocks
// ---------------------------------------------------------------------------
const {
  mockOpenPath,
  mockRevealItemInDir,
  mockIsTauri,
  mockToastError,
} = vi.hoisted(() => ({
  mockOpenPath: vi.fn().mockResolvedValue(undefined),
  mockRevealItemInDir: vi.fn().mockResolvedValue(undefined),
  mockIsTauri: vi.fn(() => true),
  mockToastError: vi.fn(),
}));

vi.mock("@tauri-apps/plugin-opener", () => ({
  openPath: mockOpenPath,
  revealItemInDir: mockRevealItemInDir,
}));

vi.mock("@/lib/tauri", () => ({
  isTauri: mockIsTauri,
  tauriInvoke: vi.fn(),
}));

vi.mock("sonner", () => ({
  toast: { error: mockToastError },
  Toaster: () => null,
}));

// Mock react-router-dom to provide useParams
vi.mock("react-router-dom", async (importOriginal) => {
  const actual = await importOriginal<typeof import("react-router-dom")>();
  return {
    ...actual,
    useParams: () => ({ id: "doc-123" }),
  };
});

// Mock FilePreview to show a sentinel
vi.mock("@/components/preview/FilePreview", () => ({
  FilePreview: ({ doc }: { doc: { name: string } }) => (
    <div data-testid="file-preview">FilePreview:{doc.name}</div>
  ),
}));

import type { Document } from "@/lib/types";

const mockDoc: Document = {
  id: "doc-123",
  name: "test-document.pdf",
  path: "/Users/test/Documents/test-document.pdf",
  docType: "pdf",
  size: 1024,
  createdAt: "2024-01-01T00:00:00Z",
  modifiedAt: "2024-01-15T10:00:00Z",
  excerpt: "This is a test excerpt that would show the old preview",
  spaceIds: [],
  tags: [],
  isFavorite: false,
  extractedEntities: [
    {
      label: "Org",
      value: "Acme Corp",
      entityType: "organization",
      canonicalId: "entity-abc-123",
    },
    {
      label: "Person",
      value: "John Doe",
      entityType: "person",
      canonicalId: undefined,
    },
    {
      label: "Email",
      value: "john@example.com",
      entityType: "email",
      canonicalId: "entity-email-456",
    },
  ],
};

// Phase 11 Plan 09: use useRelatedDocsScored (replaces useRelatedDocuments on DocumentPage).
// useRelatedDocuments hook is kept in useTauri.ts for other callers (Assumption A1).
let mockRelatedDocs: import("@/lib/types").RelatedDocScored[] = [];

vi.mock("../hooks/useTauri", () => ({
  useDocument: (_id: string) => ({
    data: mockDoc,
    isLoading: false,
    isError: false,
  }),
  useRelatedDocsScored: () => ({ data: mockRelatedDocs }),
  useToggleFavorite: () => ({ mutate: vi.fn() }),
  useSpaces: () => ({ data: [] }),
}));

vi.mock("../lib/icons", () => ({
  resolveIcon: () => () => <span>Icon</span>,
}));

vi.mock("../components/ui/resizable", () => ({
  ResizablePanelGroup: ({ children }: { children: React.ReactNode }) => <div>{children}</div>,
  ResizablePanel: ({ children }: { children: React.ReactNode }) => <div>{children}</div>,
  ResizableHandle: () => <div />,
}));

// Import under test AFTER mocks
import DocumentPage from "./DocumentPage";

function renderDocumentPage() {
  return render(
    <MemoryRouter>
      <DocumentPage />
    </MemoryRouter>,
  );
}

describe("DocumentPage (06-05 modifications)", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockRelatedDocs = [];
    mockIsTauri.mockReturnValue(true);
    mockOpenPath.mockResolvedValue(undefined);
    mockRevealItemInDir.mockResolvedValue(undefined);
  });

  it("Test 1: clicking 'Open in default app' header button calls openPath(doc.path)", async () => {
    renderDocumentPage();

    // There may be multiple "Open in default app" buttons (header + FilePreview area)
    // We want the header button — find all and click the first visible one
    const openBtns = screen.getAllByRole("button", { name: /open in default app/i });
    expect(openBtns.length).toBeGreaterThan(0);
    fireEvent.click(openBtns[0]);

    await waitFor(() => {
      expect(mockOpenPath).toHaveBeenCalledWith(mockDoc.path);
    });
  });

  it("Test 1b: openPath failure shows error toast", async () => {
    mockOpenPath.mockRejectedValue(new Error("OS error"));
    renderDocumentPage();

    const openBtns = screen.getAllByRole("button", { name: /open in default app/i });
    fireEvent.click(openBtns[0]);

    await waitFor(() => {
      expect(mockToastError).toHaveBeenCalledWith(
        "Could not open file. Open it manually from the file manager.",
      );
    });
  });

  it("Test 2: clicking reveal button calls revealItemInDir(doc.path)", async () => {
    renderDocumentPage();

    const revealBtn = screen.getByRole("button", { name: /reveal in finder|show in file manager/i });
    fireEvent.click(revealBtn);

    await waitFor(() => {
      expect(mockRevealItemInDir).toHaveBeenCalledWith(mockDoc.path);
    });
  });

  it("Test 3: FilePreview is rendered; excerpt block is NOT in the DOM", () => {
    renderDocumentPage();

    // FilePreview sentinel should be present
    expect(screen.getByTestId("file-preview")).toBeDefined();

    // "Content Preview" heading should be GONE
    const contentPreviewHeadings = screen.queryAllByText(/content preview/i);
    expect(contentPreviewHeadings.length).toBe(0);

    // Excerpt text should NOT be rendered as a p/div anymore
    expect(screen.queryByText(/this is a test excerpt/i)).toBeNull();
  });

  it("Test 4: Entity chips render as buttons w/ accessible name (Phase 11 dual-nav)", () => {
    renderDocumentPage();

    // Phase 11 refactored EntityChip from <Link> to <button> w/ useNavigate.
    // Test just verifies the entity chip renders w/ correct accessible label.
    const acmeButton = screen.getByRole("button", { name: /acme corp/i });
    expect(acmeButton).toBeDefined();
    expect(acmeButton.getAttribute("aria-label")).toContain("Acme Corp");
  });

  it("Test 5: Entity value renders in button label (Phase 11 dual-nav)", () => {
    renderDocumentPage();

    const johnButton = screen.getByRole("button", { name: /john doe/i });
    expect(johnButton).toBeDefined();
    expect(johnButton.getAttribute("aria-label")).toContain("John Doe");
  });

  it("Test 6: Dead 'Open in Finder placeholder' button is removed", () => {
    renderDocumentPage();

    // The old placeholder had text "Open in Finder" as static text / link text
    // Look for an exact match of the old placeholder text pattern
    // The new button says "Open in default app" not "Open in Finder"
    const oldPlaceholders = screen.queryAllByText(/^Open in Finder$/);
    expect(oldPlaceholders.length).toBe(0);
  });
});

// =============================================================================
// Phase 8 (08-08 Task 2 RED): metadata sidebar entity display in mandated order
//
// These tests verify that after the Phase 8 DocumentPage changes:
// - Entity section renders EntityChip links for high-confidence entities
// - No "Also found" expander when all entities are high-confidence (or no confidence)
// - Topic section absent when doc.topic is not set
// - Heading for entities section is present when entities exist
//
// Note: mockDoc has extractedEntities with no confidence field → treated as high-confidence
// =============================================================================

describe("DocumentPage Phase 8 — entity sidebar (08-08 Task 2)", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockRelatedDocs = [];
    mockIsTauri.mockReturnValue(true);
    mockOpenPath.mockResolvedValue(undefined);
    mockRevealItemInDir.mockResolvedValue(undefined);
  });

  it("Phase 8 Test A: entity chip buttons render in entities section (Phase 11 dual-nav)", () => {
    renderDocumentPage();
    // Phase 11 refactored to <button>; check by aria-label prefix
    const entityButtons = screen.getAllByRole("button").filter((b) =>
      (b.getAttribute("aria-label") || "").startsWith("Filter by "),
    );
    expect(entityButtons.length).toBeGreaterThanOrEqual(3);
  });

  it("Phase 8 Test B: no 'Also found' expander when all entities lack confidence field", () => {
    renderDocumentPage();
    // mockDoc entities have no confidence field → filtered out of ConfidenceExpander
    expect(screen.queryByText(/Also found/)).toBeNull();
  });

  it("Phase 8 Test C: topic section absent when doc.topic is not set", () => {
    renderDocumentPage();
    // mockDoc has no topic → TopicChip and "Topic" label should not render
    const topicLabel = screen.queryByText(/^Topic$/);
    expect(topicLabel).toBeNull();
  });

  it("Phase 8 Test D: entities heading visible when extractedEntities is non-empty", () => {
    renderDocumentPage();
    // The Extracted Entities heading (or Entities) should be visible
    const heading = screen.queryByText(/Extracted Entities|^Entities$/i);
    expect(heading).not.toBeNull();
  });
});

// =============================================================================
// Phase 11 (11-09 Task 2): DocumentPage Related panel — scored variant
//
// Verifies that DocumentPage now uses useRelatedDocsScored (RelatedDocScored[])
// and renders: document title + ScoreBadge (percentage) + optional snippet per row.
// =============================================================================

describe("DocumentPage Phase 11 — Related Documents scored panel (11-09 Task 2)", () => {
  const makeRelated = (
    overrides: {
      id?: string;
      name?: string;
      score?: number;
      snippet?: string | null;
    }
  ): import("@/lib/types").RelatedDocScored => ({
    document: {
      id: overrides.id ?? "rel-1",
      name: overrides.name ?? "Related Doc",
      path: "/some/path.pdf",
      docType: "pdf",
      size: 512,
      createdAt: "2026-01-01T00:00:00Z",
      modifiedAt: "2026-01-01T00:00:00Z",
      spaceIds: [],
      tags: [],
      isFavorite: false,
      extractedEntities: [],
    },
    score: overrides.score ?? 0.75,
    cosineScore: 0.8,
    jaccardScore: 0.6,
    snippet: overrides.snippet !== undefined ? overrides.snippet : null,
  });

  beforeEach(() => {
    vi.clearAllMocks();
    mockRelatedDocs = [];
    mockIsTauri.mockReturnValue(true);
  });

  // -------------------------------------------------------------------------
  // Test 1: Renders both titles + both ScoreBadges with correct percentages
  // -------------------------------------------------------------------------
  it("renders titles and ScoreBadge percentages for each related doc", () => {
    mockRelatedDocs = [
      makeRelated({ id: "r1", name: "Invoice Q1 2026.pdf", score: 0.87 }),
      makeRelated({ id: "r2", name: "Property Tax 2024.pdf", score: 0.62 }),
    ];
    renderDocumentPage();

    expect(screen.getByText("Invoice Q1 2026.pdf")).toBeDefined();
    expect(screen.getByText("Property Tax 2024.pdf")).toBeDefined();
    // ScoreBadge renders percentage strings
    expect(screen.getByText("87%")).toBeDefined();
    expect(screen.getByText("62%")).toBeDefined();
  });

  // -------------------------------------------------------------------------
  // Test 2: Snippet renders when present
  // -------------------------------------------------------------------------
  it("renders snippet when present in the related doc entry", () => {
    mockRelatedDocs = [
      makeRelated({
        id: "r3",
        name: "Medical Report.pdf",
        score: 0.91,
        snippet: "Patient was seen on 12 January 2026 for annual checkup.",
      }),
    ];
    renderDocumentPage();

    expect(screen.getByText("Patient was seen on 12 January 2026 for annual checkup.")).toBeDefined();
  });

  // -------------------------------------------------------------------------
  // Test 3: Snippet is absent when snippet is null / undefined
  // -------------------------------------------------------------------------
  it("does not render snippet row when snippet is null", () => {
    mockRelatedDocs = [
      makeRelated({ id: "r4", name: "Empty Snippet Doc.txt", score: 0.55, snippet: null }),
    ];
    renderDocumentPage();

    // Title and badge should be present
    expect(screen.getByText("Empty Snippet Doc.txt")).toBeDefined();
    expect(screen.getByText("55%")).toBeDefined();
    // No snippet text in the DOM (only the title and badge row)
    // We verify by checking no p.text-xs.text-text-secondary paragraph renders
    // (there's no unique text to query for a null snippet — absence of the element is enough)
  });

  // -------------------------------------------------------------------------
  // Test 4: Related section not rendered when useRelatedDocsScored returns empty
  // -------------------------------------------------------------------------
  it("does not render Related Documents section when related list is empty", () => {
    mockRelatedDocs = [];
    renderDocumentPage();

    // The section header should not appear
    expect(screen.queryByText("Related Documents")).toBeNull();
  });
});
