import { useEffect, useRef, useState } from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { Loader2, AlertCircle } from "lucide-react";
import { openPath } from "@tauri-apps/plugin-opener";
import { toast } from "sonner";
import { isTauri } from "@/lib/tauri";
import type { Document } from "@/lib/types";
import { usePreview } from "@/hooks/usePreview";
import { SizeGuardCard } from "./SizeGuardCard";
import { UnsupportedPreview } from "./UnsupportedPreview";
import type { HighlightRange } from "./FilePreview";

const TEXT_SIZE_LIMIT = 5 * 1024 * 1024; // 5 MB

interface MarkdownPreviewProps {
  doc: Document;
  highlightRange?: HighlightRange;
}

export function MarkdownPreview({ doc, highlightRange }: MarkdownPreviewProps) {
  const [forceLoad, setForceLoad] = useState(false);

  // Browser-dev fallback
  if (!isTauri()) {
    return <UnsupportedPreview doc={doc} />;
  }

  // Size guard: show SizeGuardCard before calling usePreview to defer the network request
  if (doc.size > TEXT_SIZE_LIMIT && !forceLoad) {
    const sizeMB = Math.round(doc.size / (1024 * 1024));
    const handleOpenExternal = async () => {
      try {
        await openPath(doc.path);
      } catch {
        toast.error("Could not open file. Open it manually from the file manager.");
      }
    };
    return (
      <SizeGuardCard
        sizeMB={sizeMB}
        onLoad={() => setForceLoad(true)}
        onOpenExternal={handleOpenExternal}
      />
    );
  }

  return <MarkdownPreviewContent doc={doc} highlightRange={highlightRange} />;
}

function MarkdownPreviewContent({
  doc,
  highlightRange,
}: {
  doc: Document;
  highlightRange?: HighlightRange;
}) {
  const { data, isLoading, isError, refetch } = usePreview(doc.id);
  const containerRef = useRef<HTMLDivElement>(null);
  const text = data?.text ?? "";

  // T-11.7-06: any invalid range is treated as if highlightRange were undefined.
  // Markdown source is parsed (not sliced) — the range cannot be split into
  // spans without corrupting markdown syntax boundaries, so this branch marks
  // the whole rendered block and scrolls it into view (D-17 discretion: "OR
  // scroll-to via data-chunk-start").
  const validRange =
    highlightRange &&
    Number.isFinite(highlightRange.start) &&
    Number.isFinite(highlightRange.end) &&
    highlightRange.start >= 0 &&
    highlightRange.start < highlightRange.end &&
    highlightRange.end <= text.length
      ? highlightRange
      : undefined;

  useEffect(() => {
    if (!validRange) return;
    const el = containerRef.current?.querySelector(".chat-citation-highlight");
    // scrollIntoView is absent in some test/edge environments (e.g. jsdom) —
    // guard defensively so a missing implementation never crashes the preview.
    if (el && typeof el.scrollIntoView === "function") {
      el.scrollIntoView({ block: "center", behavior: "auto" });
    }
  }, [validRange, text]);

  if (isLoading) {
    return (
      <div className="h-full w-full flex flex-col items-center justify-center gap-2 bg-bg-primary">
        <Loader2 size={16} className="animate-spin text-text-secondary" />
        <span className="text-sm text-text-secondary">Reading file…</span>
      </div>
    );
  }

  if (isError) {
    return (
      <div className="h-full w-full flex flex-col items-center justify-center gap-3 bg-bg-primary p-6">
        <AlertCircle size={24} className="text-red-400" />
        <span className="text-sm text-text-secondary">Could not read file</span>
        <button
          type="button"
          onClick={() => refetch()}
          className="btn-secondary text-sm px-3 py-1.5"
        >
          Retry
        </button>
      </div>
    );
  }

  if (!data || data.text === null) {
    return (
      <div className="h-full w-full flex items-center justify-center bg-bg-primary">
        <span className="text-sm text-text-secondary">Empty file</span>
      </div>
    );
  }

  return (
    <div ref={containerRef} className="h-full w-full overflow-auto bg-bg-primary">
      {/*
       * SECURITY: The HTML-escaping plugin is intentionally omitted.
       * No custom urlTransform prop is set.
       * react-markdown's default behavior escapes HTML and blocks javascript: URLs.
       * See T-06-MD-XSS in the plan threat model.
       */}
      <div
        className={
          validRange
            ? "chat-citation-highlight prose prose-invert dark:prose-invert max-w-none p-6 bg-yellow-200/20 rounded-lg animate-citation-pulse"
            : "prose prose-invert dark:prose-invert max-w-none p-6"
        }
      >
        <ReactMarkdown remarkPlugins={[remarkGfm]}>
          {data.text}
        </ReactMarkdown>
      </div>
    </div>
  );
}
