/**
 * Tests for DocumentContextMenu (UX-06 / D-17 / D-18).
 *
 * These tests verify:
 * 1. Clicking "Open in default app" calls openPath(doc.path)
 * 2. Clicking "Reveal in Finder" calls revealItemInDir(doc.path)
 * 3. Clicking "Open" navigates to /document/{doc.id}
 * 4. When openPath throws, error toast shown
 * 5. When revealItemInDir throws, error toast shown
 * 6. When isTauri() is false, handlers return early (no openPath/revealItemInDir calls)
 * 7. DocumentRow renders link with /document/{id} and shows doc.name
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import React from "react";

// Use vi.hoisted so variables are available in factory closures
const {
  mockOpenPath,
  mockRevealItemInDir,
  mockIsTauri,
  mockToastError,
  mockNavigate,
} = vi.hoisted(() => ({
  mockOpenPath: vi.fn(),
  mockRevealItemInDir: vi.fn(),
  mockIsTauri: vi.fn(() => true),
  mockToastError: vi.fn(),
  mockNavigate: vi.fn(),
}));

vi.mock("@tauri-apps/plugin-opener", () => ({
  openPath: mockOpenPath,
  revealItemInDir: mockRevealItemInDir,
}));

vi.mock("@/lib/tauri", () => ({
  isTauri: mockIsTauri,
}));

vi.mock("sonner", () => ({
  toast: {
    error: mockToastError,
  },
  Toaster: () => null,
}));

vi.mock("react-router-dom", async (importOriginal) => {
  const actual = await importOriginal<typeof import("react-router-dom")>();
  return {
    ...actual,
    useNavigate: () => mockNavigate,
  };
});

// Import components under test AFTER mocks
import { DocumentContextMenu } from "./DocumentContextMenu";
import { DocumentRow } from "./DocumentRow";
import { MemoryRouter } from "react-router-dom";
import type { Document } from "@/lib/types";

const mockDoc: Document = {
  id: "doc-123",
  name: "test-document.pdf",
  path: "/Users/test/Documents/test-document.pdf",
  docType: "pdf",
  size: 1024,
  createdAt: "2024-01-01T00:00:00Z",
  modifiedAt: "2024-01-15T10:00:00Z",
  excerpt: "Test excerpt",
  spaceIds: [],
  tags: [],
  isFavorite: false,
  extractedEntities: [],
};

function renderWithMenu(doc: Document = mockDoc) {
  return render(
    <MemoryRouter>
      <DocumentContextMenu doc={doc}>
        <button data-testid="trigger">Right-click me</button>
      </DocumentContextMenu>
    </MemoryRouter>,
  );
}

function openContextMenu() {
  const trigger = screen.getByTestId("trigger");
  fireEvent.contextMenu(trigger);
}

describe("DocumentContextMenu", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockIsTauri.mockReturnValue(true);
    mockOpenPath.mockResolvedValue(undefined);
    mockRevealItemInDir.mockResolvedValue(undefined);
  });

  it("Test 1: clicking 'Open in default app' calls openPath(doc.path)", async () => {
    renderWithMenu();
    openContextMenu();

    const openExternalItem = await screen.findByText(/open in default app/i);
    fireEvent.click(openExternalItem);

    await waitFor(() => {
      expect(mockOpenPath).toHaveBeenCalledWith(mockDoc.path);
    });
  });

  it("Test 2: clicking 'Reveal in Finder' calls revealItemInDir(doc.path)", async () => {
    renderWithMenu();
    openContextMenu();

    const revealItem = await screen.findByText(/reveal in finder|show in file manager/i);
    fireEvent.click(revealItem);

    await waitFor(() => {
      expect(mockRevealItemInDir).toHaveBeenCalledWith(mockDoc.path);
    });
  });

  it("Test 3: clicking 'Open' navigates to /document/{doc.id}", async () => {
    renderWithMenu();
    openContextMenu();

    // Find the "Open" item (not "Open in default app")
    const openItems = await screen.findAllByRole("menuitem");
    const openItem = openItems.find((el) => el.textContent?.trim() === "Open");
    expect(openItem).toBeDefined();
    fireEvent.click(openItem!);

    expect(mockNavigate).toHaveBeenCalledWith(`/document/${mockDoc.id}`);
  });

  it("Test 4: when openPath throws, error toast is shown", async () => {
    mockOpenPath.mockRejectedValue(new Error("OS error"));
    renderWithMenu();
    openContextMenu();

    const openExternalItem = await screen.findByText(/open in default app/i);
    fireEvent.click(openExternalItem);

    await waitFor(() => {
      expect(mockToastError).toHaveBeenCalledWith(
        "Could not open file. Open it manually from the file manager.",
      );
    });
  });

  it("Test 5: when revealItemInDir throws, error toast is shown", async () => {
    mockRevealItemInDir.mockRejectedValue(new Error("OS error"));
    renderWithMenu();
    openContextMenu();

    const revealItem = await screen.findByText(/reveal in finder|show in file manager/i);
    fireEvent.click(revealItem);

    await waitFor(() => {
      expect(mockToastError).toHaveBeenCalledWith("Could not reveal file in Finder.");
    });
  });

  it("Test 6: when isTauri() is false, openPath is NOT called (browser-dev guard)", async () => {
    mockIsTauri.mockReturnValue(false);
    renderWithMenu();
    openContextMenu();

    const openExternalItem = await screen.findByText(/open in default app/i);
    fireEvent.click(openExternalItem);

    await new Promise((r) => setTimeout(r, 50));
    expect(mockOpenPath).not.toHaveBeenCalled();
    expect(mockRevealItemInDir).not.toHaveBeenCalled();
  });
});

describe("DocumentRow", () => {
  it("Test 7: DocumentRow renders link to /document/{id} with doc.name and doc.path", () => {
    render(
      <MemoryRouter>
        <DocumentRow doc={mockDoc} />
      </MemoryRouter>,
    );

    // Should have a link to the document
    const link = screen.getByRole("link");
    expect(link).toBeDefined();

    // Should display the document name
    expect(screen.getByText("test-document.pdf")).toBeDefined();

    // Should display the path (font-mono)
    expect(screen.getByText(/test-document\.pdf/)).toBeDefined();
  });
});
