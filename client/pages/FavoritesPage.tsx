import { useState, useMemo } from "react";
import { Link } from "react-router-dom";
import {
  Star,
  FileText,
  FileSpreadsheet,
  FileImage,
  File,
  FileCode,
  ArrowUpDown,
} from "lucide-react";
import { safeDistance } from "../lib/utils";
import { useFavoriteDocuments, useToggleFavorite } from "../hooks/useTauri";
import { DocumentContextMenu } from "../components/documents/DocumentContextMenu";
import type { Document } from "../lib/types";

const fileTypeIcons: Record<string, typeof FileText> = {
  pdf: FileText,
  docx: FileText,
  txt: FileCode,
  md: FileCode,
  xlsx: FileSpreadsheet,
  csv: FileSpreadsheet,
  png: FileImage,
  jpg: FileImage,
};

function getFileIcon(docType: string) {
  return fileTypeIcons[docType] ?? File;
}

type SortKey = "name" | "modifiedAt" | "size";

function sortDocuments(docs: Document[], sortBy: SortKey): Document[] {
  return [...docs].sort((a, b) => {
    switch (sortBy) {
      case "name":
        return a.name.localeCompare(b.name);
      case "modifiedAt":
        return new Date(b.modifiedAt).getTime() - new Date(a.modifiedAt).getTime();
      case "size":
        return b.size - a.size;
      default:
        return 0;
    }
  });
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

export default function FavoritesPage() {
  const { data: documents, isLoading } = useFavoriteDocuments();
  const { mutate: toggleFavorite } = useToggleFavorite();
  const [sortBy, setSortBy] = useState<SortKey>("modifiedAt");

  const sorted = useMemo(() => {
    if (!documents) return [];
    return sortDocuments(documents, sortBy);
  }, [documents, sortBy]);

  if (isLoading) {
    return (
      <div className="space-y-6">
        <div className="space-y-2">
          <h1 className="page-title text-text-primary">Favorites</h1>
          <p className="text-text-secondary">Loading your starred documents...</p>
        </div>
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          {Array.from({ length: 6 }).map((_, i) => (
            <div key={i} className="card p-5 animate-pulse">
              <div className="space-y-3">
                <div className="w-10 h-10 rounded-lg bg-bg-tertiary" />
                <div className="h-4 w-32 rounded bg-bg-tertiary" />
                <div className="h-3 w-24 rounded bg-bg-tertiary" />
              </div>
            </div>
          ))}
        </div>
      </div>
    );
  }

  if (!documents || documents.length === 0) {
    return (
      <div className="flex items-center justify-center min-h-[60vh]">
        <div className="text-center space-y-4">
          <div className="mx-auto w-16 h-16 rounded-full bg-bg-tertiary flex items-center justify-center">
            <Star size={32} className="text-text-tertiary" />
          </div>
          <h2 className="text-xl font-semibold text-text-primary">No favorites yet</h2>
          <p className="text-text-secondary max-w-sm">
            Star documents to find them quickly here.
          </p>
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div className="space-y-2">
          <h1 className="page-title text-text-primary">Favorites</h1>
          <p className="text-text-secondary">
            {sorted.length} starred document{sorted.length !== 1 ? "s" : ""}
          </p>
        </div>
        <div className="flex items-center gap-2">
          <ArrowUpDown size={14} className="text-text-tertiary" />
          <select
            value={sortBy}
            onChange={(e) => setSortBy(e.target.value as SortKey)}
            className="text-sm bg-bg-secondary border border-border-primary rounded-lg px-3 py-1.5 text-text-primary focus:outline-none focus:ring-1 focus:ring-accent-primary"
          >
            <option value="modifiedAt">Date Modified</option>
            <option value="name">Name</option>
            <option value="size">Size</option>
          </select>
        </div>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
        {sorted.map((doc) => {
          const Icon = getFileIcon(doc.docType);
          return (
            <DocumentContextMenu key={doc.id} doc={doc}>
              <div className="card p-5 hover:border-accent-primary/50 transition-all group relative">
                <button
                  type="button"
                  onClick={(e) => {
                    e.preventDefault();
                    toggleFavorite(doc.id);
                  }}
                  className="absolute top-3 right-3 p-1.5 rounded-lg hover:bg-bg-tertiary transition-colors"
                  title="Remove from favorites"
                >
                  <Star size={16} className="text-yellow-500 fill-yellow-500" />
                </button>
                <Link to={`/document/${doc.id}`} className="block space-y-3">
                  <div className="p-2.5 rounded-lg bg-accent-subtle text-accent-primary inline-flex">
                    <Icon size={22} />
                  </div>
                  <div>
                    <p className="font-medium text-text-primary text-sm truncate pr-8">
                      {doc.name}
                    </p>
                    {doc.spaceIds.length > 0 && (
                      <p className="text-xs text-text-tertiary mt-1 truncate">
                        {doc.spaceIds.map((s) => s.replace("space-", "")).join(", ")}
                      </p>
                    )}
                  </div>
                  <div className="flex items-center justify-between text-xs text-text-tertiary">
                    <span>
                      {safeDistance(doc.modifiedAt)}
                    </span>
                    <span>{formatSize(doc.size)}</span>
                  </div>
                </Link>
              </div>
            </DocumentContextMenu>
          );
        })}
      </div>
    </div>
  );
}
