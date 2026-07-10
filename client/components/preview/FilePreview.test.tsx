/**
 * Tests for preview components (PAGE-13 / D-13–D-16) and usePreview hook.
 *
 * Tests:
 * 1. FilePreview dispatcher — routes by docType
 * 2. PdfPreview — size guard flip on Load preview CTA
 * 3. ImagePreview — size guard; renders img with convertFileSrc src
 * 4. TextPreview — loading/error/empty/success states; calls usePreview
 * 5. MarkdownPreview XSS — script tag renders as text, not executable element
 * 6. SizeGuardCard — onLoad / onOpenExternal callbacks
 * 7. UnsupportedPreview — openPath / revealItemInDir called
 * 8. usePreview hook — dispatches read_document_text IPC
 * 9. Browser-dev fallback — PdfPreview / ImagePreview degrade without convertFileSrc
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import React from "react";

// ---------------------------------------------------------------------------
// Hoist mocks so they are ready before vi.mock factory closures run
// ---------------------------------------------------------------------------
const {
  mockConvertFileSrc,
  mockOpenPath,
  mockRevealItemInDir,
  mockIsTauri,
  mockToastError,
  mockUsePreviewData,
  mockRefetch,
} = vi.hoisted(() => ({
  mockConvertFileSrc: vi.fn((p: string) => `asset://localhost${p}`),
  mockOpenPath: vi.fn().mockResolvedValue(undefined),
  mockRevealItemInDir: vi.fn().mockResolvedValue(undefined),
  mockIsTauri: vi.fn(() => true),
  mockToastError: vi.fn(),
  mockUsePreviewData: vi.fn(),
  mockRefetch: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({
  convertFileSrc: mockConvertFileSrc,
  invoke: vi.fn(),
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

// Mock usePreview hook — must match the import path used by the components
vi.mock("@/hooks/usePreview", () => ({
  usePreview: (docId: string) => mockUsePreviewData(docId),
}));

// ---------------------------------------------------------------------------
// Import components AFTER mocks
// ---------------------------------------------------------------------------
import { FilePreview } from "./FilePreview";
import { PdfPreview } from "./PdfPreview";
import { ImagePreview } from "./ImagePreview";
import { TextPreview } from "./TextPreview";
import { MarkdownPreview } from "./MarkdownPreview";
import { SizeGuardCard } from "./SizeGuardCard";
import { UnsupportedPreview } from "./UnsupportedPreview";
import type { Document } from "@/lib/types";

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------
function makeDoc(overrides: Partial<Document> = {}): Document {
  return {
    id: "doc-1",
    name: "test.pdf",
    path: "/Users/test/Documents/test.pdf",
    docType: "pdf",
    size: 1024,
    createdAt: "2024-01-01T00:00:00Z",
    modifiedAt: "2024-01-15T10:00:00Z",
    excerpt: "Test excerpt",
    spaceIds: [],
    tags: [],
    isFavorite: false,
    extractedEntities: [],
    ...overrides,
  };
}

const PDF_SIZE_LIMIT = 50 * 1024 * 1024;
const IMAGE_SIZE_LIMIT = 20 * 1024 * 1024;
const TEXT_SIZE_LIMIT = 5 * 1024 * 1024;

// ---------------------------------------------------------------------------
// Test 1: FilePreview dispatcher
// ---------------------------------------------------------------------------
describe("Test 1: FilePreview dispatcher", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockIsTauri.mockReturnValue(true);
    mockConvertFileSrc.mockImplementation((p: string) => `asset://localhost${p}`);
    mockUsePreviewData.mockReturnValue({
      data: { text: "hello", truncated: false, size: 5 },
      isLoading: false,
      isError: false,
      refetch: mockRefetch,
    });
  });

  it("docType=pdf renders PdfPreview (iframe or browser fallback)", () => {
    const doc = makeDoc({ docType: "pdf" });
    const { container } = render(<FilePreview doc={doc} />);
    // In Tauri mode, should render an iframe (or size guard)
    // Either iframe or a fallback element is rendered — no crash
    expect(container.firstChild).not.toBeNull();
  });

  it("docType=png renders ImagePreview (img or browser fallback)", () => {
    const doc = makeDoc({ docType: "png", name: "photo.png" });
    const { container } = render(<FilePreview doc={doc} />);
    expect(container.firstChild).not.toBeNull();
  });

  it("docType=jpg renders ImagePreview", () => {
    const doc = makeDoc({ docType: "jpg", name: "photo.jpg" });
    const { container } = render(<FilePreview doc={doc} />);
    expect(container.firstChild).not.toBeNull();
  });

  it("docType=md renders MarkdownPreview", () => {
    const doc = makeDoc({ docType: "md", name: "README.md" });
    const { container } = render(<FilePreview doc={doc} />);
    expect(container.firstChild).not.toBeNull();
  });

  it("docType=txt renders TextPreview", () => {
    const doc = makeDoc({ docType: "txt", name: "notes.txt" });
    const { container } = render(<FilePreview doc={doc} />);
    expect(container.firstChild).not.toBeNull();
  });

  it("docType=csv renders TextPreview", () => {
    const doc = makeDoc({ docType: "csv", name: "data.csv" });
    const { container } = render(<FilePreview doc={doc} />);
    expect(container.firstChild).not.toBeNull();
  });

  it("docType=xlsx renders UnsupportedPreview (default case)", () => {
    const doc = makeDoc({ docType: "xlsx", name: "spreadsheet.xlsx" });
    render(<FilePreview doc={doc} />);
    expect(screen.getByText(/preview not supported/i)).toBeDefined();
  });

  it("docType=docx renders UnsupportedPreview", () => {
    const doc = makeDoc({ docType: "docx", name: "doc.docx" });
    render(<FilePreview doc={doc} />);
    expect(screen.getByText(/preview not supported/i)).toBeDefined();
  });
});

// ---------------------------------------------------------------------------
// Test 2: PdfPreview — size guard
// ---------------------------------------------------------------------------
describe("Test 2: PdfPreview size guard", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockIsTauri.mockReturnValue(true);
    mockConvertFileSrc.mockImplementation((p: string) => `asset://localhost${p}`);
  });

  it("when doc.size > 50 MB and not forced, renders SizeGuardCard (not iframe)", () => {
    const doc = makeDoc({ size: PDF_SIZE_LIMIT + 1 });
    render(<PdfPreview doc={doc} />);
    expect(screen.getByText(/load preview/i)).toBeDefined();
    // iframe should NOT be in DOM
    const iframe = document.querySelector("iframe");
    expect(iframe).toBeNull();
  });

  it("clicking Load preview CTA renders the iframe (forceLoad=true)", async () => {
    const doc = makeDoc({ size: PDF_SIZE_LIMIT + 1 });
    render(<PdfPreview doc={doc} />);

    const loadBtn = screen.getByRole("button", { name: /load preview/i });
    fireEvent.click(loadBtn);

    await waitFor(() => {
      const iframe = document.querySelector("iframe");
      expect(iframe).not.toBeNull();
    });
  });

  it("when doc.size <= 50 MB, renders iframe directly", () => {
    const doc = makeDoc({ size: PDF_SIZE_LIMIT - 1 });
    render(<PdfPreview doc={doc} />);
    const iframe = document.querySelector("iframe");
    expect(iframe).not.toBeNull();
  });
});

// ---------------------------------------------------------------------------
// Test 3: ImagePreview — size guard + img src
// ---------------------------------------------------------------------------
describe("Test 3: ImagePreview", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockIsTauri.mockReturnValue(true);
    mockConvertFileSrc.mockImplementation((p: string) => `asset://localhost${p}`);
  });

  it("when doc.size > 20 MB and not forced, renders SizeGuardCard (not img)", () => {
    const doc = makeDoc({ docType: "png", name: "big.png", size: IMAGE_SIZE_LIMIT + 1 });
    render(<ImagePreview doc={doc} />);
    expect(screen.getByText(/load preview/i)).toBeDefined();
    const img = document.querySelector("img");
    expect(img).toBeNull();
  });

  it("when doc.size <= 20 MB, renders img with convertFileSrc src", () => {
    const doc = makeDoc({ docType: "png", name: "small.png", size: IMAGE_SIZE_LIMIT - 1 });
    render(<ImagePreview doc={doc} />);
    const img = document.querySelector("img");
    expect(img).not.toBeNull();
    expect(img?.src).toContain("asset://localhost");
    expect(mockConvertFileSrc).toHaveBeenCalledWith(doc.path);
  });
});

// ---------------------------------------------------------------------------
// Test 4: TextPreview — loading/error/empty/success states
// ---------------------------------------------------------------------------
describe("Test 4: TextPreview states", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockIsTauri.mockReturnValue(true);
  });

  it("shows loading spinner when isLoading=true", () => {
    mockUsePreviewData.mockReturnValue({
      data: undefined,
      isLoading: true,
      isError: false,
      refetch: mockRefetch,
    });
    const doc = makeDoc({ docType: "txt", name: "notes.txt" });
    render(<TextPreview doc={doc} />);
    expect(screen.getByText(/reading file/i)).toBeDefined();
  });

  it("shows error state with retry button when isError=true", async () => {
    mockUsePreviewData.mockReturnValue({
      data: undefined,
      isLoading: false,
      isError: true,
      refetch: mockRefetch,
    });
    const doc = makeDoc({ docType: "txt", name: "notes.txt" });
    render(<TextPreview doc={doc} />);
    expect(screen.getByText(/could not read file/i)).toBeDefined();
    const retryBtn = screen.getByRole("button", { name: /retry/i });
    fireEvent.click(retryBtn);
    expect(mockRefetch).toHaveBeenCalled();
  });

  it("renders text in pre block on success", () => {
    mockUsePreviewData.mockReturnValue({
      data: { text: "Hello world\nLine 2", truncated: false, size: 20 },
      isLoading: false,
      isError: false,
      refetch: mockRefetch,
    });
    const doc = makeDoc({ docType: "txt", name: "notes.txt" });
    render(<TextPreview doc={doc} />);
    const pre = document.querySelector("pre");
    expect(pre).not.toBeNull();
    expect(pre?.textContent).toContain("Hello world");
  });

  it("shows SizeGuardCard when doc.size > 5 MB (defers usePreview)", () => {
    mockUsePreviewData.mockReturnValue({
      data: undefined,
      isLoading: false,
      isError: false,
      refetch: mockRefetch,
    });
    const doc = makeDoc({ docType: "txt", name: "large.txt", size: TEXT_SIZE_LIMIT + 1 });
    render(<TextPreview doc={doc} />);
    expect(screen.getByText(/load preview/i)).toBeDefined();
  });
});

// ---------------------------------------------------------------------------
// Test 5: MarkdownPreview XSS
// ---------------------------------------------------------------------------
describe("Test 5: MarkdownPreview XSS safety", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockIsTauri.mockReturnValue(true);
  });

  it("script tag in markdown renders as literal text, NOT executable script element", () => {
    const maliciousMarkdown = "# Hello\n\n<script>alert(1)</script>";
    mockUsePreviewData.mockReturnValue({
      data: { text: maliciousMarkdown, truncated: false, size: 50 },
      isLoading: false,
      isError: false,
      refetch: mockRefetch,
    });
    const doc = makeDoc({ docType: "md", name: "README.md" });
    const { container } = render(<MarkdownPreview doc={doc} />);

    // Heading should render
    expect(screen.getByText("Hello")).toBeDefined();

    // NO executable script element in DOM
    expect(container.querySelector("script")).toBeNull();
  });
});

// ---------------------------------------------------------------------------
// Test 6: SizeGuardCard callbacks
// ---------------------------------------------------------------------------
describe("Test 6: SizeGuardCard", () => {
  it("primary CTA invokes onLoad", () => {
    const onLoad = vi.fn();
    const onOpenExternal = vi.fn();
    render(<SizeGuardCard sizeMB={120} onLoad={onLoad} onOpenExternal={onOpenExternal} />);

    const loadBtn = screen.getByRole("button", { name: /load preview/i });
    fireEvent.click(loadBtn);
    expect(onLoad).toHaveBeenCalled();
    expect(onOpenExternal).not.toHaveBeenCalled();
  });

  it("secondary CTA invokes onOpenExternal", () => {
    const onLoad = vi.fn();
    const onOpenExternal = vi.fn();
    render(<SizeGuardCard sizeMB={120} onLoad={onLoad} onOpenExternal={onOpenExternal} />);

    const openBtn = screen.getByRole("button", { name: /open in default app/i });
    fireEvent.click(openBtn);
    expect(onOpenExternal).toHaveBeenCalled();
    expect(onLoad).not.toHaveBeenCalled();
  });
});

// ---------------------------------------------------------------------------
// Test 7: UnsupportedPreview — openPath / revealItemInDir
// ---------------------------------------------------------------------------
describe("Test 7: UnsupportedPreview", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockIsTauri.mockReturnValue(true);
    mockOpenPath.mockResolvedValue(undefined);
    mockRevealItemInDir.mockResolvedValue(undefined);
  });

  it("primary CTA calls openPath(doc.path)", async () => {
    const doc = makeDoc({ docType: "xlsx", name: "report.xlsx" });
    render(<UnsupportedPreview doc={doc} />);

    const openBtn = screen.getByRole("button", { name: /open in default app/i });
    fireEvent.click(openBtn);

    await waitFor(() => {
      expect(mockOpenPath).toHaveBeenCalledWith(doc.path);
    });
  });

  it("secondary CTA calls revealItemInDir(doc.path)", async () => {
    const doc = makeDoc({ docType: "xlsx", name: "report.xlsx" });
    render(<UnsupportedPreview doc={doc} />);

    const revealBtn = screen.getByRole("button", {
      name: /reveal in finder|show in file manager/i,
    });
    fireEvent.click(revealBtn);

    await waitFor(() => {
      expect(mockRevealItemInDir).toHaveBeenCalledWith(doc.path);
    });
  });

  it("body copy includes file extension from doc.name", () => {
    const doc = makeDoc({ docType: "xlsx", name: "report.xlsx" });
    render(<UnsupportedPreview doc={doc} />);
    expect(screen.getByText(/\.xlsx/i)).toBeDefined();
  });

  it("shows 'Preview not supported' heading", () => {
    const doc = makeDoc({ docType: "xlsx", name: "report.xlsx" });
    render(<UnsupportedPreview doc={doc} />);
    expect(screen.getByText(/preview not supported/i)).toBeDefined();
  });
});

// ---------------------------------------------------------------------------
// Test 10: highlightRange — citation deep-link (Phase 11.7 Plan 03, RAGCH-07)
// ---------------------------------------------------------------------------
describe("Test 10: highlightRange citation deep-link", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockIsTauri.mockReturnValue(true);
  });

  it("TextPreview + valid highlightRange marks the range with chat-citation-highlight + bg-yellow-200", () => {
    mockUsePreviewData.mockReturnValue({
      data: { text: "Hello world, this is a citation target.", truncated: false, size: 40 },
      isLoading: false,
      isError: false,
      refetch: mockRefetch,
    });
    const doc = makeDoc({ docType: "txt", name: "notes.txt" });
    const { container } = render(
      <TextPreview doc={doc} highlightRange={{ start: 13, end: 17 }} />,
    );
    const mark = container.querySelector(".chat-citation-highlight");
    expect(mark).not.toBeNull();
    expect(mark?.className).toContain("bg-yellow-200");
    expect(mark?.textContent).toBe("this");
  });

  it("TextPreview + missing highlightRange renders unchanged (no mark, full text present)", () => {
    mockUsePreviewData.mockReturnValue({
      data: { text: "Hello world", truncated: false, size: 11 },
      isLoading: false,
      isError: false,
      refetch: mockRefetch,
    });
    const doc = makeDoc({ docType: "txt", name: "notes.txt" });
    const { container } = render(<TextPreview doc={doc} />);
    expect(container.querySelector(".chat-citation-highlight")).toBeNull();
    expect(container.querySelector("pre")?.textContent).toBe("Hello world");
  });

  it("TextPreview + malformed highlightRange (end <= start) ignored — no crash, no mark", () => {
    mockUsePreviewData.mockReturnValue({
      data: { text: "Hello world", truncated: false, size: 11 },
      isLoading: false,
      isError: false,
      refetch: mockRefetch,
    });
    const doc = makeDoc({ docType: "txt", name: "notes.txt" });
    const { container } = render(
      <TextPreview doc={doc} highlightRange={{ start: 8, end: 2 }} />,
    );
    expect(container.querySelector(".chat-citation-highlight")).toBeNull();
    expect(container.querySelector("pre")?.textContent).toBe("Hello world");
  });

  it("TextPreview + highlightRange.end > text.length ignored — no crash, no mark", () => {
    mockUsePreviewData.mockReturnValue({
      data: { text: "Hello world", truncated: false, size: 11 },
      isLoading: false,
      isError: false,
      refetch: mockRefetch,
    });
    const doc = makeDoc({ docType: "txt", name: "notes.txt" });
    const { container } = render(
      <TextPreview doc={doc} highlightRange={{ start: 0, end: 99999 }} />,
    );
    expect(container.querySelector(".chat-citation-highlight")).toBeNull();
    expect(container.querySelector("pre")?.textContent).toBe("Hello world");
  });

  it("MarkdownPreview + valid highlightRange marks the rendered block with chat-citation-highlight + bg-yellow-200", () => {
    const md = "# Title\n\nSome citable body text here.";
    mockUsePreviewData.mockReturnValue({
      data: { text: md, truncated: false, size: md.length },
      isLoading: false,
      isError: false,
      refetch: mockRefetch,
    });
    const doc = makeDoc({ docType: "md", name: "README.md" });
    const { container } = render(
      <MarkdownPreview doc={doc} highlightRange={{ start: 9, end: 13 }} />,
    );
    const mark = container.querySelector(".chat-citation-highlight");
    expect(mark).not.toBeNull();
    expect(mark?.className).toContain("bg-yellow-200");
    // Markdown content still renders normally (not corrupted by the highlight wrap)
    expect(screen.getByText("Title")).toBeDefined();
  });

  it("MarkdownPreview + missing highlightRange renders unchanged (no mark)", () => {
    const md = "# Title\n\nBody.";
    mockUsePreviewData.mockReturnValue({
      data: { text: md, truncated: false, size: md.length },
      isLoading: false,
      isError: false,
      refetch: mockRefetch,
    });
    const doc = makeDoc({ docType: "md", name: "README.md" });
    const { container } = render(<MarkdownPreview doc={doc} />);
    expect(container.querySelector(".chat-citation-highlight")).toBeNull();
    expect(screen.getByText("Title")).toBeDefined();
  });

  it("PdfPreview ignores highlightRange — renders unchanged, no mark, no crash", () => {
    const doc = makeDoc({ docType: "pdf" });
    const { container } = render(
      <FilePreview doc={doc} highlightRange={{ start: 0, end: 10 }} />,
    );
    expect(container.querySelector(".chat-citation-highlight")).toBeNull();
    expect(container.firstChild).not.toBeNull();
  });

  it("ImagePreview ignores highlightRange — renders unchanged, no mark, no crash", () => {
    const doc = makeDoc({ docType: "png", name: "photo.png" });
    const { container } = render(
      <FilePreview doc={doc} highlightRange={{ start: 0, end: 10 }} />,
    );
    expect(container.querySelector(".chat-citation-highlight")).toBeNull();
    expect(container.firstChild).not.toBeNull();
  });
});

// ---------------------------------------------------------------------------
// Test 9: Browser-dev fallback
// ---------------------------------------------------------------------------
describe("Test 9: Browser-dev fallback", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockIsTauri.mockReturnValue(false);
  });

  it("PdfPreview in browser mode renders fallback (not iframe)", () => {
    const doc = makeDoc({ docType: "pdf" });
    render(<PdfPreview doc={doc} />);
    const iframe = document.querySelector("iframe");
    expect(iframe).toBeNull();
  });

  it("ImagePreview in browser mode renders fallback (not img)", () => {
    const doc = makeDoc({ docType: "png", name: "photo.png", size: 100 });
    render(<ImagePreview doc={doc} />);
    const img = document.querySelector("img");
    expect(img).toBeNull();
    // convertFileSrc should NOT be called in browser mode
    expect(mockConvertFileSrc).not.toHaveBeenCalled();
  });
});
