import { useState, useMemo } from "react";
import { Link } from "react-router-dom";
import { LayoutGrid, List, ArrowUpDown, FolderOpen, RefreshCw, Loader2, Lock } from "lucide-react";
import { useSpaces, useReclusterSpaces } from "../hooks/useTauri";
import { toast } from "sonner";
import { resolveIcon } from "../lib/icons";
import { cn } from "../lib/utils";
import { formatRelativeTime } from "../lib/format";
import { SpaceCard } from "../components/spaces/SpaceCard";
import { SpaceLabelingIndicator } from "../components/spaces/SpaceLabelingIndicator";
import type { Space } from "../lib/types";

type ViewMode = "grid" | "list";
type SortKey = "documentCount" | "name" | "lastUpdated";

function SkeletonGrid() {
  return (
    <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
      {Array.from({ length: 6 }).map((_, i) => (
        <div key={i} className="card p-6 animate-pulse">
          <div className="flex items-start gap-3">
            <div className="h-10 w-10 rounded-lg bg-bg-tertiary" />
            <div className="flex-1 space-y-2">
              <div className="h-4 w-24 rounded bg-bg-tertiary" />
              <div className="h-3 w-16 rounded bg-bg-tertiary" />
            </div>
          </div>
        </div>
      ))}
    </div>
  );
}

function SpaceRow({ space }: { space: Space }) {
  const Icon = resolveIcon(space.icon);
  return (
    <Link
      to={`/spaces/${space.id}`}
      className="flex items-center gap-4 px-4 py-3 rounded-lg border border-border-primary bg-bg-secondary hover:bg-bg-tertiary transition-colors"
    >
      <div className="p-2 rounded-lg" style={{ backgroundColor: `${space.color}15` }}>
        <Icon size={20} style={{ color: space.color }} />
      </div>
      <span className="font-medium text-text-primary flex-1">{space.name}</span>
      {/* D-15: Lock icon in list view when user has manually locked the space label */}
      {space.userLocked && (
        <Lock size={12} className="text-text-tertiary" aria-label="Label locked by user" />
      )}
      <span className="text-sm text-text-tertiary w-24 text-right">
        {space.documentCount} docs
      </span>
      <span className="text-sm text-text-tertiary w-20 text-right">
        {formatRelativeTime(space.lastUpdated)}
      </span>
    </Link>
  );
}

export default function SpacesPage() {
  const { data: spaces, isLoading, isError } = useSpaces();
  const recluster = useReclusterSpaces();
  const handleRecluster = () => {
    recluster.mutate(undefined, {
      onSuccess: (next) =>
        toast.success(
          next.length === 0
            ? "Re-clustered: no spaces yet — index more documents."
            : `Re-clustered into ${next.length} Smart Space${next.length === 1 ? "" : "s"}.`,
        ),
      onError: (err) => toast.error(`Re-cluster failed: ${String(err)}`),
    });
  };
  const [viewMode, setViewMode] = useState<ViewMode>("grid");
  const [sortKey, setSortKey] = useState<SortKey>("documentCount");

  const sortedSpaces = useMemo(() => {
    if (!spaces) return [];
    const copy = [...spaces];
    switch (sortKey) {
      case "name":
        return copy.sort((a, b) => a.name.localeCompare(b.name));
      case "lastUpdated":
        return copy.sort(
          (a, b) => new Date(b.lastUpdated).getTime() - new Date(a.lastUpdated).getTime(),
        );
      case "documentCount":
      default:
        return copy.sort((a, b) => b.documentCount - a.documentCount);
    }
  }, [spaces, sortKey]);

  if (isError) {
    return (
      <div className="flex items-center justify-center min-h-[60vh]">
        <div className="text-center space-y-2">
          <p className="text-text-primary font-medium">Failed to load spaces</p>
          <p className="text-text-secondary text-sm">Please try again later.</p>
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="page-title text-text-primary">Smart Spaces</h1>
          <p className="text-text-secondary text-sm mt-1">
            Auto-organized virtual folders based on document content.
          </p>
        </div>
        <div className="flex items-center gap-2">
          {/* SpaceLabelingIndicator: shows batch-labeling progress chip in header right side */}
          <SpaceLabelingIndicator />
          <button
            onClick={handleRecluster}
            disabled={recluster.isPending}
            className="flex items-center gap-1.5 px-3 py-1.5 text-sm rounded-md bg-accent-primary text-white hover:bg-accent-hover disabled:opacity-60 disabled:cursor-not-allowed transition-colors"
          >
            {recluster.isPending ? <Loader2 size={14} className="animate-spin" /> : <RefreshCw size={14} />}
            {recluster.isPending ? "Re-clustering…" : "Re-cluster"}
          </button>
          {/* Sort */}
          <div className="flex items-center gap-1 mr-2">
            <ArrowUpDown size={14} className="text-text-tertiary" />
            <select
              value={sortKey}
              onChange={(e) => setSortKey(e.target.value as SortKey)}
              className="text-sm bg-bg-secondary border border-border-primary rounded-md px-2 py-1 text-text-secondary focus:outline-none focus:ring-1 focus:ring-accent-primary"
            >
              <option value="documentCount">By count</option>
              <option value="name">By name</option>
              <option value="lastUpdated">By updated</option>
            </select>
          </div>
          {/* View toggle */}
          <button
            onClick={() => setViewMode("grid")}
            className={cn(
              "p-2 rounded-md transition-colors",
              viewMode === "grid"
                ? "bg-accent-subtle text-accent-primary"
                : "text-text-tertiary hover:text-text-secondary",
            )}
            aria-label="Grid view"
          >
            <LayoutGrid size={18} />
          </button>
          <button
            onClick={() => setViewMode("list")}
            className={cn(
              "p-2 rounded-md transition-colors",
              viewMode === "list"
                ? "bg-accent-subtle text-accent-primary"
                : "text-text-tertiary hover:text-text-secondary",
            )}
            aria-label="List view"
          >
            <List size={18} />
          </button>
        </div>
      </div>

      {/* Content */}
      {isLoading ? (
        <SkeletonGrid />
      ) : sortedSpaces.length === 0 ? (
        <div className="flex flex-col items-center justify-center min-h-[40vh] text-center space-y-4">
          <div className="p-4 rounded-full bg-bg-secondary">
            <FolderOpen size={40} className="text-text-tertiary" />
          </div>
          <div className="space-y-2">
            <p className="text-text-primary font-medium">No Smart Spaces discovered yet</p>
            <p className="text-text-secondary text-sm max-w-sm">
              Add watched folders and index documents to auto-generate spaces.
            </p>
          </div>
        </div>
      ) : viewMode === "grid" ? (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          {sortedSpaces.map((space) => (
            <SpaceCard key={space.id} space={space} />
          ))}
        </div>
      ) : (
        <div className="space-y-2">
          {sortedSpaces.map((space) => (
            <SpaceRow key={space.id} space={space} />
          ))}
        </div>
      )}
    </div>
  );
}
