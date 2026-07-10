import type { Document } from "@/lib/types";
import { PdfPreview } from "./PdfPreview";
import { ImagePreview } from "./ImagePreview";
import { TextPreview } from "./TextPreview";
import { MarkdownPreview } from "./MarkdownPreview";
import { UnsupportedPreview } from "./UnsupportedPreview";

export interface HighlightRange {
  start: number;
  end: number;
}

interface FilePreviewProps {
  doc: Document;
  /**
   * Optional character-offset range to scroll to and visually mark.
   * Only honored by TextPreview and MarkdownPreview (Phase 11.7 Plan 03,
   * RAGCH-07). Ignored by PdfPreview, ImagePreview, and UnsupportedPreview —
   * character-offset → page/position mapping for PDFs is a v1.3 problem.
   */
  highlightRange?: HighlightRange;
}

/**
 * FilePreview — dispatcher component that renders the appropriate preview
 * renderer based on doc.docType.
 *
 * PAGE-13 / D-13, D-14, D-15: Preview dispatcher.
 */
export function FilePreview({ doc, highlightRange }: FilePreviewProps) {
  switch (doc.docType) {
    case "pdf":
      return <PdfPreview doc={doc} />;
    case "png":
    case "jpg":
      return <ImagePreview doc={doc} />;
    case "md":
      return <MarkdownPreview doc={doc} highlightRange={highlightRange} />;
    case "txt":
    case "csv":
      return <TextPreview doc={doc} highlightRange={highlightRange} />;
    default:
      // docx, xlsx, and other unsupported types
      return <UnsupportedPreview doc={doc} />;
  }
}
