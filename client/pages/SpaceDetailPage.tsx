import { useMemo, useState } from "react";
import { Link, useParams } from "react-router-dom";
import { Edit2, FolderOpen, Loader2, Lock, RefreshCw } from "lucide-react";
import { toast } from "sonner";
import { useSpaces, useSpaceDocuments, useRenameSpace, useClearSpaceOverride, useRelabelSpace } from "../hooks/useTauri";
import { resolveIcon } from "../lib/icons";
import { DocumentRow } from "../components/documents/DocumentRow";
import { Input } from "../components/ui/input";
import { Button } from "../components/ui/button";
import { Skeleton } from "../components/ui/skeleton";
import { Breadcrumb, BreadcrumbList, BreadcrumbItem, BreadcrumbLink, BreadcrumbPage, BreadcrumbSeparator } from "../components/ui/breadcrumb";
import { SubSpaceCard } from "../components/spaces/SubSpaceCard";
import { ParentContextBanner } from "../components/spaces/ParentContextBanner";
import type { Space } from "../lib/types";


function formatRelativeTime(iso: string): string {
  const ms = Date.now() - new Date(iso).getTime();
  const minutes = Math.floor(ms / 60_000);
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.floor(hours / 24);
  return `${days}d ago`;
}


function SkeletonDetail() {
  return (
    <div className="space-y-6 animate-pulse">
      <div className="flex items-center gap-4">
        <div className="h-12 w-12 rounded-lg bg-bg-tertiary" />
        <div className="space-y-2">
          <div className="h-6 w-40 rounded bg-bg-tertiary" />
          <div className="h-4 w-24 rounded bg-bg-tertiary" />
        </div>
      </div>
      <div className="space-y-2">
        {Array.from({ length: 4 }).map((_, i) => (
          <div key={i} className="h-12 rounded-lg bg-bg-tertiary" />
        ))}
      </div>
    </div>
  );
}

export default function SpaceDetailPage() {
  const { id } = useParams<{ id: string }>();
  const { data: spaces, isLoading: spacesLoading } = useSpaces();
  const { data: documents, isLoading: docsLoading } = useSpaceDocuments(id ?? "");

  const [isEditing, setIsEditing] = useState(false);
  const [editValue, setEditValue] = useState("");

  const rename = useRenameSpace();
  const clearOverride = useClearSpaceOverride();
  const relabel = useRelabelSpace();

  const space = useMemo(() => {
    if (!spaces || !id) return undefined;
    // D-07: flat lookup — all spaces (including sub-spaces) are top-level entries in the flat store
    return spaces.find((s) => s.id === id);
  }, [spaces, id]);

  const parentSpace = useMemo(() => {
    if (!spaces || !space?.parentId) return undefined;
    // Guard: a space cannot be its own parent (corrupt data / race condition).
    // If parentId === id, skip the lookup to prevent an infinite navigation
    // loop where the breadcrumb links back to the current page (CR-03).
    if (space.parentId === space.id) return undefined;
    return spaces.find((s) => s.id === space.parentId);
  }, [spaces, space]);

  // D-07: flat filter — sub-spaces are stored as top-level entries with parentId set
  const subSpaces = useMemo(() => {
    if (!spaces || !space) return [];
    return spaces.filter((s) => s.parentId === space.id);
  }, [spaces, space]);

  // WR-04: use id-based sentinel check for Misc sub-space detection.
  // The Rust side always generates "{parent_id}-misc" as the ID sentinel (D-04).
  // Checking s.name === "Misc" is fragile: an LLM could return "Misc" as a label
  // for a real sub-space, and a user renaming a Misc space would break the check.
  const isMiscSubSpace = (s: Space) => s.id.endsWith("-misc");

  // Sort: labeled sub-spaces first (by documentCount desc), Misc last
  const sortedSubSpaces = useMemo(() => {
    const labeled = subSpaces
      .filter((s) => !isMiscSubSpace(s))
      .sort((a, b) => b.documentCount - a.documentCount);
    const misc = subSpaces.filter(isMiscSubSpace);
    return [...labeled, ...misc];
  }, [subSpaces]);

  // Related spaces: those that share documents with this space
  const relatedSpaces = useMemo(() => {
    if (!spaces || !documents || !id) return [];
    const relatedIds = new Set<string>();
    for (const doc of documents) {
      for (const sid of doc.spaceIds) {
        if (sid !== id) relatedIds.add(sid);
      }
    }
    return spaces.filter((s) => relatedIds.has(s.id));
  }, [spaces, documents, id]);

  const isLoading = spacesLoading || docsLoading;

  if (isLoading) return <SkeletonDetail />;

  if (!space) {
    return (
      <div className="flex items-center justify-center min-h-[60vh]">
        <div className="text-center space-y-2">
          <p className="text-text-primary font-medium">Space not found</p>
          <Link to="/spaces" className="text-sm text-accent-primary hover:text-accent-hover">
            Back to Spaces
          </Link>
        </div>
      </div>
    );
  }

  const Icon = resolveIcon(space.icon);

  function handleStartEdit() {
    setEditValue(space!.name);
    setIsEditing(true);
  }

  function handleSave(value: string) {
    const trimmed = value.trim();
    if (!trimmed || trimmed === space!.name) {
      setIsEditing(false);
      return;
    }
    rename.mutate(
      { spaceId: space!.id, label: trimmed },
      {
        onSuccess: () => {
          toast.success("Label saved and locked. Cortex won't overwrite this name.", { duration: 4000 });
          setIsEditing(false);
        },
        onError: (err) => toast.error(`Failed to save label. ${String(err)}`, { duration: 6000 }),
      },
    );
  }

  function handleCancelEdit() {
    setIsEditing(false);
    setEditValue(space!.name);
  }

  function handleClearOverride() {
    clearOverride.mutate(space!.id, {
      onSuccess: () =>
        toast.info("Override cleared. This space will be re-labeled on next recluster.", { duration: 5000 }),
      onError: (err) => toast.error(`Failed to clear override. ${String(err)}`, { duration: 6000 }),
    });
  }

  function handleRegenerate() {
    relabel.mutate(space!.id, {
      onSuccess: () => toast.success(`Label regenerated for ${space!.name}.`, { duration: 4000 }),
      onError: (err) => {
        const msg = String(err);
        if (msg.includes("SpaceLocked")) {
          toast.info(
            `Label for "${space!.name}" is locked. Clear the override first to allow regeneration.`,
            { duration: 6000 },
          );
        } else {
          toast.error(`Failed to regenerate label for "${space!.name}". ${msg}`, { duration: 6000 });
        }
      },
    });
  }

  return (
    <div className="space-y-6">
      {/* Breadcrumb — shadcn Breadcrumb primitive (D-15, HSPC-02) */}
      <Breadcrumb>
        <BreadcrumbList>
          <BreadcrumbItem>
            <BreadcrumbLink asChild>
              <Link to="/spaces">Spaces</Link>
            </BreadcrumbLink>
          </BreadcrumbItem>
          <BreadcrumbSeparator />
          {parentSpace && (
            <>
              <BreadcrumbItem>
                <BreadcrumbLink asChild>
                  <Link to={`/spaces/${parentSpace.id}`}>{parentSpace.name}</Link>
                </BreadcrumbLink>
              </BreadcrumbItem>
              <BreadcrumbSeparator />
            </>
          )}
          <BreadcrumbItem>
            <BreadcrumbPage>{space.name}</BreadcrumbPage>
          </BreadcrumbItem>
        </BreadcrumbList>
      </Breadcrumb>

      {/* Parent Context Banner — only when space is a sub-space AND parent resolves (D-16, UI-SPEC §3) */}
      {space.parentId && parentSpace && (
        <ParentContextBanner parent={parentSpace} />
      )}

      {/* Header */}
      <div className="flex items-start gap-4">
        <div
          className="p-3 rounded-lg"
          style={{ backgroundColor: `${space.color}15`, color: space.color }}
        >
          <Icon size={28} />
        </div>
        <div className="flex-1">
          {/* View state */}
          {!isEditing && (
            <div className="flex items-center gap-2 group">
              <h1 className="page-title text-text-primary">{space.name}</h1>
              {space.userLocked && (
                <Lock size={14} className="text-text-tertiary" aria-label="Label locked by user" />
              )}
              <button
                onClick={handleStartEdit}
                className="opacity-0 group-hover:opacity-100 transition-opacity p-1 rounded hover:bg-bg-tertiary"
                aria-label="Edit space label"
                style={{ minWidth: 44, minHeight: 44 }}
              >
                <Edit2 size={16} className="text-text-tertiary" />
              </button>
            </div>
          )}

          {/* Editing state */}
          {isEditing && (
            <>
              <Input
                value={editValue}
                onChange={(e) => setEditValue(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") handleSave(editValue);
                  if (e.key === "Escape") handleCancelEdit();
                }}
                onBlur={() => handleSave(editValue)}
                autoFocus
                className="text-2xl font-semibold h-auto py-1 px-2 w-full max-w-[400px] bg-bg-secondary border-border-primary focus-visible:ring-accent-primary"
              />
              <div className="flex items-center gap-2 mt-2">
                <Button
                  variant="default"
                  size="sm"
                  onClick={() => handleSave(editValue)}
                  disabled={rename.isPending}
                >
                  Save label
                </Button>
                <Button variant="ghost" size="sm" onClick={handleCancelEdit}>
                  Cancel edit
                </Button>
                {space.userLocked && (
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={handleClearOverride}
                    className="text-text-tertiary hover:text-destructive"
                  >
                    Clear override
                  </Button>
                )}
              </div>
            </>
          )}

          {/* Doc count + Regenerate label */}
          <div className="flex items-center gap-4 mt-1">
            <p className="text-sm text-text-secondary">
              {space.documentCount} documents — Updated {formatRelativeTime(space.lastUpdated)}
            </p>
            {!isEditing && (
              <Button
                variant="ghost"
                size="sm"
                onClick={handleRegenerate}
                disabled={relabel.isPending || space.labelStatus === "generating"}
                className="flex items-center gap-2 text-text-secondary hover:text-text-primary"
              >
                {relabel.isPending
                  ? <Loader2 size={14} className="animate-spin" />
                  : <RefreshCw size={14} />}
                {relabel.isPending ? "Regenerating…" : "Regenerate label"}
              </Button>
            )}
          </div>
        </div>
      </div>

      {/* Description block */}
      {space.labelStatus === "generating" && !space.description && (
        <Skeleton className="h-4 w-72" />
      )}
      {space.labelStatus !== "generating" && space.description && (
        <p className="text-sm text-text-secondary max-w-prose leading-relaxed">
          {space.description}
        </p>
      )}
      {space.labelStatus !== "generating" && !space.description && space.canonicalEntityHint && (
        <p className="text-sm text-text-tertiary italic max-w-prose leading-relaxed">
          Space organized around {space.canonicalEntityHint} documents.
        </p>
      )}

      {/* Sub-spaces — D-07 flat filter; sortedSubSpaces: labeled first by documentCount desc, Misc last */}
      {sortedSubSpaces.length > 0 && (
        <div className="space-y-3">
          <h2 className="section-header text-text-primary">Sub-Spaces</h2>
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
            {sortedSubSpaces.map((sub) =>
              sub.labelStatus === "generating" ? (
                <div key={sub.id} className="card p-4 border-l-4" style={{ borderLeftColor: sub.color }}>
                  <div className="flex items-center gap-3">
                    <div className="p-2 rounded-lg bg-accent-subtle">
                      <Skeleton className="h-4 w-4" />
                    </div>
                    <div className="space-y-2 flex-1">
                      <Skeleton className="h-4 w-28" />
                      <Skeleton className="h-3 w-16" />
                    </div>
                  </div>
                </div>
              ) : (
                <SubSpaceCard key={sub.id} space={sub} isMisc={isMiscSubSpace(sub)} />
              )
            )}
          </div>
        </div>
      )}

      {/* Documents */}
      <div className="space-y-3">
        <h2 className="section-header text-text-primary">
          Documents {documents && `(${documents.length})`}
        </h2>
        {!documents || documents.length === 0 ? (
          <div className="flex flex-col items-center justify-center py-12 text-center space-y-3">
            <FolderOpen size={32} className="text-text-tertiary" />
            <p className="text-text-secondary text-sm">No documents in this space yet.</p>
          </div>
        ) : (
          <div className="space-y-2">
            {documents.map((doc) => (
              <DocumentRow key={doc.id} doc={doc} />
            ))}
          </div>
        )}
      </div>

      {/* Related Spaces — peer spaces that share documents with this space; isMisc=false (never dashed for related) */}
      {relatedSpaces.length > 0 && (
        <div className="space-y-3">
          <h2 className="section-header text-text-primary">Related Spaces</h2>
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
            {relatedSpaces.map((rs) => (
              <SubSpaceCard key={rs.id} space={rs} isMisc={false} />
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
