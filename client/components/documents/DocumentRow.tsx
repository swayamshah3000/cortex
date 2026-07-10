import { Link } from "react-router-dom";
import { FileText } from "lucide-react";
import { DocumentContextMenu } from "./DocumentContextMenu";
import type { Document } from "@/lib/types";

/**
 * Color mapping for doc type icons — preserved from SpaceDetailPage.tsx DocTypeIcon.
 */
const docTypeColorMap: Record<string, string> = {
  pdf: "text-red-400",
  docx: "text-blue-400",
  xlsx: "text-green-400",
  csv: "text-green-400",
  txt: "text-text-tertiary",
  md: "text-text-tertiary",
  png: "text-amber-400",
  jpg: "text-amber-400",
};

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function formatDate(iso: string): string {
  return new Date(iso).toLocaleDateString("en-US", {
    month: "short",
    day: "numeric",
    year: "numeric",
  });
}

/**
 * Shared document row component used across SpaceDetailPage, RecentPage,
 * FavoritesPage, and SearchPage surfaces. Wraps the link in DocumentContextMenu
 * so right-clicking opens the native actions menu (UX-06).
 *
 * Extracted from SpaceDetailPage.tsx lines 46-59 with CSS preserved byte-for-byte.
 */
export function DocumentRow({ doc }: { doc: Document }) {
  return (
    <DocumentContextMenu doc={doc}>
      <Link
        to={`/document/${doc.id}`}
        className="flex items-center gap-4 px-4 py-3 rounded-lg border border-border-primary bg-bg-secondary hover:bg-bg-tertiary transition-colors"
      >
        <FileText size={16} className={docTypeColorMap[doc.docType] ?? "text-text-tertiary"} />
        <span className="font-medium text-text-primary flex-1 truncate">{doc.name}</span>
        <span className="text-xs text-text-tertiary uppercase w-12 text-center">{doc.docType}</span>
        <span className="text-sm text-text-tertiary w-20 text-right">{formatBytes(doc.size)}</span>
        <span className="text-sm text-text-tertiary w-24 text-right">{formatDate(doc.modifiedAt)}</span>
      </Link>
    </DocumentContextMenu>
  );
}
