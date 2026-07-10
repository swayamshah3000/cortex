import { useMemo } from "react";
import { useParams, useSearchParams, Link } from "react-router-dom";
import {
  ChevronRight,
  FileText,
  Star,
  ExternalLink,
  Calendar,
  HardDrive,
  Clock,
  FolderOpen,
} from "lucide-react";
import { EntityChip } from "@/components/entities/EntityChip";
import { TopicChip } from "@/components/entities/TopicChip";
import { TagChip } from "@/components/entities/TagChip";
import { ConfidenceExpander } from "@/components/entities/ConfidenceExpander";
import {
  ResizablePanelGroup,
  ResizablePanel,
  ResizableHandle,
} from "../components/ui/resizable";
import {
  useDocument,
  useRelatedDocsScored,
  useToggleFavorite,
  useSpaces,
} from "../hooks/useTauri";
import { ScoreBadge } from "../components/search/ScoreBadge";
import { cn } from "../lib/utils";
import { resolveIcon } from "../lib/icons";
import { openPath, revealItemInDir } from "@tauri-apps/plugin-opener";
import { toast } from "sonner";
import { isTauri } from "@/lib/tauri";
import { FilePreview } from "@/components/preview/FilePreview";

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function formatDate(iso: string): string {
  return new Date(iso).toLocaleDateString("en-US", {
    month: "long",
    day: "numeric",
    year: "numeric",
  });
}

/**
 * Returns the OS-appropriate label for "show file location" action.
 * macOS: "Reveal in Finder" | others: "Show in file manager"
 */
function revealLabel(): string {
  if (typeof navigator !== "undefined" && /Mac/i.test(navigator.userAgent)) {
    return "Reveal in Finder";
  }
  return "Show in file manager";
}

function SkeletonDocument() {
  return (
    <div className="h-full animate-pulse p-6 space-y-4">
      <div className="h-6 w-64 rounded bg-bg-tertiary" />
      <div className="h-4 w-48 rounded bg-bg-tertiary" />
      <div className="h-32 rounded bg-bg-tertiary" />
    </div>
  );
}

export default function DocumentPage() {
  const { id } = useParams<{ id: string }>();
  const [searchParams] = useSearchParams();
  const { data: doc, isLoading, isError } = useDocument(id ?? "");
  const { data: related } = useRelatedDocsScored(id ?? "");
  const { data: spaces } = useSpaces();
  const toggleFavorite = useToggleFavorite();

  // RAGCH-07: chat citation deep-link. `?highlight={start}-{end}` — malformed
  // or missing params are silently ignored (T-11.7-06), no toast, no crash.
  const highlightRange = useMemo(() => {
    const raw = searchParams.get("highlight");
    if (!raw) return undefined;
    const m = raw.match(/^(\d+)-(\d+)$/);
    if (!m) return undefined;
    const start = Number(m[1]);
    const end = Number(m[2]);
    if (!Number.isFinite(start) || !Number.isFinite(end) || end <= start) return undefined;
    return { start, end };
  }, [searchParams]);

  if (isLoading) return <SkeletonDocument />;

  if (isError || !doc) {
    return (
      <div className="flex items-center justify-center min-h-[60vh]">
        <div className="text-center space-y-2">
          <p className="text-text-primary font-medium">Document not found</p>
          <Link to="/" className="text-sm text-accent-primary hover:text-accent-hover">
            Back to Dashboard
          </Link>
        </div>
      </div>
    );
  }

  // Find first space for breadcrumb
  const primarySpace = spaces?.find((s) => doc.spaceIds.includes(s.id));

  const handleOpen = async () => {
    if (!isTauri()) return;
    try {
      await openPath(doc.path);
    } catch {
      toast.error("Could not open file. Open it manually from the file manager.");
    }
  };

  const handleReveal = async () => {
    if (!isTauri()) return;
    try {
      await revealItemInDir(doc.path);
    } catch {
      const label = revealLabel();
      toast.error(
        `Could not reveal file in ${label === "Reveal in Finder" ? "Finder" : "file manager"}.`,
      );
    }
  };

  return (
    <div className="h-[calc(100vh-120px)] flex flex-col space-y-4">
      {/* Breadcrumb */}
      <nav className="flex items-center gap-1 text-sm text-text-tertiary">
        <Link to="/" className="hover:text-text-secondary transition-colors">Home</Link>
        {primarySpace && (
          <>
            <ChevronRight size={14} />
            <Link
              to={`/spaces/${primarySpace.id}`}
              className="hover:text-text-secondary transition-colors"
            >
              {primarySpace.name}
            </Link>
          </>
        )}
        <ChevronRight size={14} />
        <span className="text-text-primary font-medium truncate max-w-[200px]">{doc.name}</span>
      </nav>

      {/* Split layout: 65% preview / 35% metadata */}
      <div className="flex-1 min-h-0 rounded-lg border border-border-primary overflow-hidden">
        <ResizablePanelGroup direction="horizontal">
          {/* Preview panel */}
          <ResizablePanel defaultSize={65} minSize={40}>
            <div className="h-full flex flex-col">
              {/* Preview header: title, type badge, path, Open/Reveal buttons */}
              <div className="p-4 border-b border-border-primary space-y-2 flex-shrink-0">
                <div className="flex items-start justify-between gap-4">
                  <h1 className="text-2xl font-bold text-text-primary">{doc.name}</h1>
                  <span className="px-2 py-1 text-xs font-medium uppercase rounded bg-accent-subtle text-accent-primary flex-shrink-0">
                    {doc.docType}
                  </span>
                </div>
                <p className="text-sm text-text-tertiary font-mono">{doc.path}</p>
                {/* Open / Reveal action buttons */}
                <div className="flex items-center gap-2">
                  <button
                    type="button"
                    onClick={handleOpen}
                    className="inline-flex items-center gap-1.5 px-3 py-1.5 bg-accent-primary text-white rounded-lg hover:bg-accent-hover transition-colors text-sm font-medium"
                  >
                    <ExternalLink size={14} />
                    Open in default app
                  </button>
                  <button
                    type="button"
                    onClick={handleReveal}
                    className="inline-flex items-center gap-1.5 px-3 py-1.5 border border-border-primary text-text-secondary rounded-lg hover:bg-bg-tertiary transition-colors text-sm font-medium"
                  >
                    <FolderOpen size={14} />
                    {revealLabel()}
                  </button>
                </div>
              </div>

              {/* File preview — replaces the old excerpt block */}
              <div className="flex-1 min-h-0">
                <FilePreview doc={doc} highlightRange={highlightRange} />
              </div>
            </div>
          </ResizablePanel>

          <ResizableHandle withHandle />

          {/* Metadata sidebar */}
          <ResizablePanel defaultSize={35} minSize={25}>
            <div className="h-full overflow-y-auto p-6 bg-bg-secondary space-y-6">
              {/* Favorite toggle */}
              <button
                onClick={() => toggleFavorite.mutate(doc.id)}
                className={cn(
                  "flex items-center gap-2 px-3 py-2 rounded-lg border transition-colors w-full",
                  doc.isFavorite
                    ? "bg-amber-400/10 border-amber-400/30 text-amber-400"
                    : "bg-bg-tertiary border-border-primary text-text-tertiary hover:text-amber-400",
                )}
              >
                <Star size={16} fill={doc.isFavorite ? "currentColor" : "none"} />
                <span className="text-sm font-medium">
                  {doc.isFavorite ? "Favorited" : "Add to Favorites"}
                </span>
              </button>

              {/* File info */}
              <div className="space-y-3">
                <h3 className="text-xs font-medium text-text-tertiary uppercase tracking-wider">
                  File Info
                </h3>
                <div className="space-y-2">
                  <div className="flex items-center justify-between text-sm">
                    <span className="text-text-tertiary flex items-center gap-1.5">
                      <FileText size={14} /> Type
                    </span>
                    <span className="text-text-primary uppercase">{doc.docType}</span>
                  </div>
                  <div className="flex items-center justify-between text-sm">
                    <span className="text-text-tertiary flex items-center gap-1.5">
                      <HardDrive size={14} /> Size
                    </span>
                    <span className="text-text-primary">{formatBytes(doc.size)}</span>
                  </div>
                  <div className="flex items-center justify-between text-sm">
                    <span className="text-text-tertiary flex items-center gap-1.5">
                      <Calendar size={14} /> Created
                    </span>
                    <span className="text-text-primary">{formatDate(doc.createdAt)}</span>
                  </div>
                  <div className="flex items-center justify-between text-sm">
                    <span className="text-text-tertiary flex items-center gap-1.5">
                      <Clock size={14} /> Modified
                    </span>
                    <span className="text-text-primary">{formatDate(doc.modifiedAt)}</span>
                  </div>
                </div>
              </div>

              {/* Spaces */}
              {doc.spaceIds.length > 0 && spaces && (
                <div className="space-y-3">
                  <h3 className="text-xs font-medium text-text-tertiary uppercase tracking-wider">
                    Spaces
                  </h3>
                  <div className="space-y-1.5">
                    {spaces
                      .filter((s) => doc.spaceIds.includes(s.id))
                      .map((s) => {
                        const Icon = resolveIcon(s.icon);
                        return (
                          <Link
                            key={s.id}
                            to={`/spaces/${s.id}`}
                            className="flex items-center gap-2 px-3 py-2 rounded-lg hover:bg-bg-tertiary transition-colors"
                          >
                            <Icon size={14} style={{ color: s.color }} />
                            <span className="text-sm text-text-primary">{s.name}</span>
                          </Link>
                        );
                      })}
                  </div>
                </div>
              )}

              {/* Tags */}
              {doc.tags.length > 0 && (
                <div className="space-y-3">
                  <h3 className="text-xs font-medium text-text-tertiary uppercase tracking-wider">
                    Tags
                  </h3>
                  <div className="flex flex-wrap gap-1.5">
                    {doc.tags.map((tag) => (
                      <span
                        key={tag}
                        className="px-2 py-0.5 text-xs rounded-full bg-accent-subtle text-accent-primary"
                      >
                        {tag}
                      </span>
                    ))}
                  </div>
                </div>
              )}

              {/* Extracted Entities — Phase 8 display order per UI-SPEC §5:
                  1. Topic chip (if topic set AND != 'other')
                  2. LLM Tags row (if llmTags.length > 0)
                  3. High-confidence EntityChip grid (confidence >= 0.7 OR no confidence field)
                  4. ConfidenceExpander (if any entities < 0.7) */}
              {(() => {
                const hasEntities = doc.extractedEntities.length > 0;
                const hasTopic = !!doc.topic && doc.topic !== "other";
                const hasLlmTags = !!doc.llmTags && doc.llmTags.length > 0;
                const isEmpty = !hasTopic && !hasLlmTags && !hasEntities;

                if (isEmpty) {
                  return (
                    <div className="text-xs text-text-tertiary py-8 text-center">
                      No entities found
                      <span className="mt-1 block">
                        Entities will appear after indexing completes. Connect AI for richer
                        extraction.
                      </span>
                    </div>
                  );
                }

                // High-confidence = no confidence field OR confidence >= 0.7
                const highConfidence = doc.extractedEntities.filter(
                  (e) => e.confidence == null || e.confidence >= 0.7,
                );

                return (
                  <div className="space-y-3">
                    <h3 className="text-xs font-medium text-text-tertiary uppercase tracking-wider">
                      Extracted Entities
                    </h3>

                    {/* 1. Topic chip */}
                    {hasTopic && doc.topic && (
                      <div>
                        <div className="text-xs text-text-tertiary mb-1">Topic</div>
                        <TopicChip topic={doc.topic} />
                      </div>
                    )}

                    {/* 2. LLM Tags row */}
                    {hasLlmTags && doc.llmTags && (
                      <div>
                        <div className="text-xs text-text-tertiary mb-1">Tags</div>
                        <div className="flex flex-wrap gap-1">
                          {doc.llmTags.map((t) => (
                            <TagChip key={t} tag={t} />
                          ))}
                        </div>
                      </div>
                    )}

                    {/* 3. High-confidence EntityChip grid */}
                    {highConfidence.length > 0 && (
                      <div className="flex flex-wrap gap-2">
                        {highConfidence.map((e, i) => (
                          <EntityChip key={`${e.value}-${i}`} entity={e} />
                        ))}
                      </div>
                    )}

                    {/* 4. ConfidenceExpander for low-confidence entities */}
                    <ConfidenceExpander
                      entities={doc.extractedEntities}
                      renderEntity={(e) => <EntityChip key={`low-${e.value}`} entity={e} />}
                    />
                  </div>
                );
              })()}

              {/* Related documents — Phase 11 scored variant (ENEX-03, D-10..D-14)
                  Data source: useRelatedDocsScored (0.6×cosine + 0.4×entity_jaccard).
                  Replaced Phase 3 useRelatedDocuments; useRelatedDocuments hook kept in
                  useTauri.ts for other callers (Assumption A1 in 11-RESEARCH.md). */}
              {related && related.length > 0 && (
                <div className="space-y-3">
                  <h3 className="text-xs font-medium text-text-tertiary uppercase tracking-wider">
                    Related Documents
                  </h3>
                  <div className="space-y-2">
                    {related.map((rel) => (
                      <Link
                        key={rel.document.id}
                        to={`/document/${rel.document.id}`}
                        className="flex items-start gap-2 px-3 py-2 rounded-lg hover:bg-bg-tertiary transition-colors group"
                      >
                        <FileText size={14} className="text-text-tertiary flex-shrink-0 mt-1" />
                        <div className="flex-1 min-w-0 space-y-1">
                          <div className="flex items-center justify-between gap-2">
                            <span className="text-sm text-text-primary truncate">{rel.document.name}</span>
                            <ScoreBadge score={rel.score} />
                          </div>
                          {rel.snippet && (
                            <p className="text-xs text-text-secondary line-clamp-2 leading-relaxed">
                              {rel.snippet}
                            </p>
                          )}
                        </div>
                      </Link>
                    ))}
                  </div>
                </div>
              )}
            </div>
          </ResizablePanel>
        </ResizablePanelGroup>
      </div>
    </div>
  );
}
