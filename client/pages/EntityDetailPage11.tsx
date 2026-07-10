/**
 * EntityDetailPage11 — /entity/:class/:value
 *
 * Phase 11 entity detail page backed by get_entity_page_data IPC.
 * Named EntityDetailPage11.tsx to coexist with the Phase 6 EntityDetailPage.tsx
 * (/entities/:id) per Open Q 2 resolution in 11-RESEARCH.md.
 *
 * Structure: header (class icon + value + alias count badge + doc count)
 *            aliases section (if aliases.length > 1)
 *            paginated documents (20/page, ?page=N URL param)
 *            co-occurring entities (top 10 as interactive EntityChips)
 *
 * UI-SPEC §6 + §7 (EntityDetailPage + D-18 empty state).
 * Plan 11-08 Task 1.
 */

import { useEffect } from "react";
import { useParams, useSearchParams, Link } from "react-router-dom";
import {
  Users,
  Building2,
  MapPin,
  Calendar,
  DollarSign,
  Mail,
  Phone,
  Fingerprint,
  Tag,
  FileText,
  Folder,
  Settings,
  ChevronLeft,
  ChevronRight,
} from "lucide-react";
import { toast } from "sonner";
import { useEntityPageData } from "../hooks/useTauri";
import { EntityChip } from "../components/entities/EntityChip";
import { EntityRelationsPanel } from "../components/relations/EntityRelationsPanel";
import { Skeleton } from "../components/ui/skeleton";
import { Badge } from "../components/ui/badge";
import { cn } from "../lib/utils";

// ---------------------------------------------------------------------------
// Inline helpers (not yet in a shared util module)
// ---------------------------------------------------------------------------

/** Shorten a filesystem path for compact display. Shows last 2 segments. */
function shortenPath(path: string): string {
  const parts = path.replace(/\\/g, "/").split("/").filter(Boolean);
  if (parts.length <= 2) return path;
  return "…/" + parts.slice(-2).join("/");
}

/** Return a human-readable relative time string from an ISO date string. */
function formatRelativeTime(isoDate: string): string {
  const date = new Date(isoDate);
  if (isNaN(date.getTime())) return "";
  const now = Date.now();
  const diff = now - date.getTime();
  const seconds = Math.floor(diff / 1000);
  if (seconds < 60) return "just now";
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.floor(hours / 24);
  if (days < 30) return `${days}d ago`;
  const months = Math.floor(days / 30);
  if (months < 12) return `${months}mo ago`;
  return `${Math.floor(months / 12)}y ago`;
}

// ---------------------------------------------------------------------------
// Class icon map (mirrors Phase 8 locked schema from EntityChip.tsx)
// ---------------------------------------------------------------------------

function getClassIcon(cls: string, size = 24): React.ReactNode {
  switch (cls) {
    case "Person":
      return <Users size={size} className="text-purple-400" />;
    case "Organization":
      return <Building2 size={size} className="text-amber-400" />;
    case "Location":
      return <MapPin size={size} className="text-red-400" />;
    case "Date":
      return <Calendar size={size} className="text-blue-400" />;
    case "Amount":
      return <DollarSign size={size} className="text-green-400" />;
    case "Email":
      return <Mail size={size} className="text-cyan-400" />;
    case "Phone":
      return <Phone size={size} className="text-teal-400" />;
    case "Identifier":
      return <Fingerprint size={size} className="text-orange-400" />;
    default:
      return <Tag size={size} className="text-text-tertiary" />;
  }
}

// ---------------------------------------------------------------------------
// Loading skeleton
// ---------------------------------------------------------------------------

function LoadingSkeleton() {
  return (
    <div className="space-y-8" data-testid="entity-page-loading">
      {/* Header skeleton */}
      <div className="flex items-center gap-4 animate-pulse">
        <Skeleton className="h-12 w-12 rounded-lg" />
        <div className="space-y-2">
          <Skeleton className="h-7 w-48" />
          <Skeleton className="h-4 w-32" />
        </div>
      </div>
      {/* Doc list skeleton — 5 rows */}
      <div className="space-y-3">
        {Array.from({ length: 5 }).map((_, i) => (
          <Skeleton key={i} className="h-14 w-full rounded-lg" />
        ))}
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Error state
// ---------------------------------------------------------------------------

interface ErrorStateProps {
  message: string;
  onRetry: () => void;
}

function ErrorState({ message, onRetry }: ErrorStateProps) {
  return (
    <div
      className="flex flex-col items-center justify-center min-h-[40vh] text-center space-y-4 py-16"
      data-testid="entity-page-error"
    >
      <h2 className="text-lg font-semibold text-text-primary">Could not load entity</h2>
      <p className="text-sm text-text-secondary max-w-[360px]">{message}. Try again.</p>
      <button
        type="button"
        onClick={onRetry}
        className="inline-flex items-center gap-2 text-sm text-accent-primary hover:text-accent-hover transition-colors border border-accent-primary/30 rounded-md px-4 py-2"
      >
        Retry
      </button>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Empty state (UI-SPEC §7 / D-18)
// ---------------------------------------------------------------------------

interface EmptyStateProps {
  decodedClass: string;
  decodedValue: string;
}

function EmptyState({ decodedClass, decodedValue }: EmptyStateProps) {
  return (
    <div
      className="flex flex-col items-center justify-center min-h-[40vh] text-center space-y-4 py-16"
      data-testid="entity-page-empty"
    >
      <div className="flex items-center justify-center h-16 w-16 rounded-full bg-bg-secondary">
        {getClassIcon(decodedClass, 32)}
      </div>
      <div className="space-y-2">
        <h2 className="text-lg font-semibold text-text-primary">
          No documents mention {decodedClass}: {decodedValue}
        </h2>
        <p className="text-sm text-text-secondary max-w-[360px]">
          Try syncing your folders or connecting more provider data.
        </p>
      </div>
      <div className="flex items-center gap-3 pt-2">
        <Link
          to="/watched"
          className="inline-flex items-center gap-2 text-sm text-accent-primary hover:text-accent-hover transition-colors"
        >
          <Folder size={14} />
          Manage watched folders
        </Link>
        <span className="text-text-tertiary">·</span>
        <Link
          to="/settings"
          className="inline-flex items-center gap-2 text-sm text-accent-primary hover:text-accent-hover transition-colors"
        >
          <Settings size={14} />
          Settings
        </Link>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Main page component
// ---------------------------------------------------------------------------

export default function EntityDetailPage11() {
  const { class: cls = "", value = "" } = useParams<{ class: string; value: string }>();
  const [searchParams, setSearchParams] = useSearchParams();

  const page = Math.max(0, parseInt(searchParams.get("page") ?? "0", 10) || 0);

  const decodedClass = decodeURIComponent(cls);
  const decodedValue = decodeURIComponent(value);

  const { data, isLoading, isError, error, refetch } = useEntityPageData(
    decodedClass,
    decodedValue,
    page,
  );

  // Fire toast.error once on transition into error state (T-11-27 safe: guards on cls/value)
  useEffect(() => {
    if (isError && error) {
      const msg = error instanceof Error ? error.message : String(error);
      toast.error(`Could not load entity data. ${msg}`, { duration: 6000 });
    }
  }, [isError, error]);

  // --- Loading state ---
  if (isLoading) {
    return <LoadingSkeleton />;
  }

  // --- Error state ---
  if (isError || !data) {
    const msg = error instanceof Error ? error.message : "Unknown error";
    return <ErrorState message={msg} onRetry={() => refetch()} />;
  }

  const { canonical, documents, totalDocumentCount, coOccurringEntities, pageSize } = data;

  // aliases: canonical name is always in the list; "real" aliases = aliases.length > 1
  const aliases = canonical.aliases ?? [];
  const hasAliases = aliases.length > 1;
  // Count of aliases beyond the canonical name itself
  const aliasCount = Math.max(0, aliases.length - 1);

  const totalPages = Math.ceil(totalDocumentCount / pageSize);
  const showPagination = totalDocumentCount > pageSize;

  const handlePrev = () => {
    setSearchParams((prev) => {
      const next = new URLSearchParams(prev);
      next.set("page", String(Math.max(0, page - 1)));
      return next;
    });
  };

  const handleNext = () => {
    // WR-03 fix: guard against double-click or race where disabled state hasn't
    // re-rendered yet — mirrors the disabled condition on the Next button.
    if (page >= totalPages - 1) return;
    setSearchParams((prev) => {
      const next = new URLSearchParams(prev);
      next.set("page", String(page + 1));
      return next;
    });
  };

  // --- Empty state (docs = 0 AND no aliases AND no co-occurring entities) ---
  // EntityRelationsPanel handles its own zero-state (returns null), so we don't
  // have relations info at this level; that's fine — if there ARE relations,
  // the page just shows them under the "No documents" hint below. If there are
  // none, nothing else is rendered and hasNothing is a strong signal to show
  // the original guidance empty state.
  const hasNothing =
    totalDocumentCount === 0 &&
    aliases.length <= 1 &&
    coOccurringEntities.length === 0;
  if (hasNothing) {
    return <EmptyState decodedClass={decodedClass} decodedValue={decodedValue} />;
  }

  // --- Full page ---
  return (
    <div className="space-y-8">
      {/* Header (UI-SPEC §6) */}
      <div className="flex items-center gap-4">
        <div className="flex items-center justify-center h-12 w-12 rounded-lg bg-accent-subtle flex-shrink-0">
          {getClassIcon(decodedClass, 24)}
        </div>
        <div className="space-y-1">
          <div className="flex items-center gap-3 flex-wrap">
            <h1 className="page-title text-text-primary">{decodedValue}</h1>
            {hasAliases && (
              <Badge variant="outline" className="text-xs text-text-tertiary">
                +{aliasCount} {aliasCount === 1 ? "alias" : "aliases"}
              </Badge>
            )}
          </div>
          <p className="text-sm text-text-secondary">
            <span className="capitalize">{decodedClass}</span>
            {" · "}
            {totalDocumentCount} {totalDocumentCount === 1 ? "document" : "documents"}
          </p>
        </div>
      </div>

      {/* Aliases section — only when real aliases exist beyond the canonical name */}
      {hasAliases && (
        <div className="space-y-2" data-testid="aliases-section">
          <div className="text-xs text-text-tertiary uppercase tracking-wider">Also known as</div>
          <div className="flex flex-wrap gap-2">
            {aliases
              .filter((alias) => alias !== canonical.canonicalName)
              .map((alias) => (
                <span
                  key={alias}
                  className="inline-flex items-center bg-transparent border border-border-secondary rounded-full px-2 py-1 text-xs text-text-tertiary"
                >
                  {alias}
                </span>
              ))}
          </div>
        </div>
      )}

      {/* Documents section */}
      <div className="space-y-3">
        <h2 className="text-lg font-semibold text-text-primary">
          Documents ({totalDocumentCount})
        </h2>
        {documents.length > 0 ? (
          <div className="space-y-2">
            {documents.map((doc) => (
              <Link
                key={doc.id}
                to={`/document/${doc.id}`}
                className={cn(
                  "flex items-center gap-3 px-4 py-3 rounded-lg",
                  "hover:bg-bg-secondary transition-colors",
                  "border border-transparent hover:border-border-primary",
                )}
              >
                <FileText size={16} className="text-text-tertiary flex-shrink-0" />
                <div className="flex-1 min-w-0">
                  <p className="text-sm font-medium text-text-primary truncate">{doc.name}</p>
                  <p className="text-xs text-text-tertiary font-mono truncate mt-1">
                    {shortenPath(doc.path)}
                  </p>
                </div>
                <span className="text-xs text-text-tertiary flex-shrink-0">
                  {formatRelativeTime(doc.modifiedAt)}
                </span>
              </Link>
            ))}
          </div>
        ) : (
          <p className="text-sm text-text-tertiary py-2">No documents mention this entity yet.</p>
        )}

        {/* Pagination — hidden when totalDocumentCount <= pageSize */}
        {showPagination && (
          <div className="flex items-center justify-center gap-4 pt-4">
            <button
              type="button"
              onClick={handlePrev}
              disabled={page === 0}
              className={cn(
                "inline-flex items-center gap-1 text-sm px-3 py-1.5 rounded-md border transition-colors",
                page === 0
                  ? "text-text-tertiary border-border-secondary cursor-not-allowed opacity-50"
                  : "text-text-secondary border-border-secondary hover:bg-bg-secondary",
              )}
            >
              <ChevronLeft size={14} />
              Previous
            </button>
            <span className="text-sm text-text-secondary">
              Page {page + 1} of {totalPages}
            </span>
            <button
              type="button"
              onClick={handleNext}
              disabled={page >= totalPages - 1}
              className={cn(
                "inline-flex items-center gap-1 text-sm px-3 py-1.5 rounded-md border transition-colors",
                page >= totalPages - 1
                  ? "text-text-tertiary border-border-secondary cursor-not-allowed opacity-50"
                  : "text-text-secondary border-border-secondary hover:bg-bg-secondary",
              )}
            >
              Next
              <ChevronRight size={14} />
            </button>
          </div>
        )}
      </div>

      {/* Relations section — outgoing + incoming triples for this entity (Phase 11.5) */}
      <EntityRelationsPanel entityId={canonical.id} />

      {/* Co-occurring entities section */}
      {coOccurringEntities.length > 0 && (
        <div className="space-y-3">
          <h2 className="text-lg font-semibold text-text-primary">Related Entities</h2>
          <div className="flex flex-wrap gap-2">
            {coOccurringEntities.map((ref) => (
              <EntityChip
                key={`${ref.class}:${ref.value}`}
                entity={{
                  value: ref.value,
                  entityType: ref.class.toLowerCase(),
                  class: ref.class,
                }}
              />
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
