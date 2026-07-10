/**
 * OwnershipPage — /ownership/:personId
 *
 * "All my assets" view for a person entity, grouped by AssetType
 * (Property, Vehicle, Investment, Business, Financial, Other).
 * Backed by useAllOwnedBy (Plan 06). Empty asset-type sections are hidden
 * by OwnershipAssetSection itself (returns null when assets.length === 0).
 *
 * Phase 11.5 Plan 07, Task 3. D-18.
 */

import { useParams, Link } from "react-router-dom";
import { Users, Folder, Settings } from "lucide-react";
import { useAllOwnedBy } from "@/hooks/useTauri";
import { OwnershipAssetSection } from "@/components/relations/OwnershipAssetSection";
import { Skeleton } from "@/components/ui/skeleton";
import type { AssetType } from "@/lib/types";

const ASSET_TYPE_ORDER: AssetType[] = [
  "Property",
  "Vehicle",
  "Investment",
  "Business",
  "Financial",
  "Other",
];

function LoadingSkeleton() {
  return (
    <div className="space-y-8" data-testid="ownership-page-loading">
      <div className="flex items-center gap-4 animate-pulse">
        <Skeleton className="h-12 w-12 rounded-lg" />
        <div className="space-y-2">
          <Skeleton className="h-7 w-48" />
          <Skeleton className="h-4 w-32" />
        </div>
      </div>
      <div className="space-y-3">
        {Array.from({ length: 2 }).map((_, i) => (
          <Skeleton key={i} className="h-24 w-full rounded-lg" />
        ))}
      </div>
    </div>
  );
}

interface ErrorStateProps {
  message: string;
  onRetry: () => void;
}

function ErrorState({ message, onRetry }: ErrorStateProps) {
  return (
    <div
      className="flex flex-col items-center justify-center min-h-[40vh] text-center space-y-4 py-16"
      data-testid="ownership-page-error"
    >
      <h2 className="text-lg font-semibold text-text-primary">Could not load ownership data</h2>
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

interface EmptyOwnershipStateProps {
  personName: string;
}

function EmptyOwnershipState({ personName }: EmptyOwnershipStateProps) {
  return (
    <div
      className="flex flex-col items-center justify-center min-h-[40vh] text-center space-y-4 py-16"
      data-testid="ownership-page-empty"
    >
      <div className="flex items-center justify-center h-16 w-16 rounded-full bg-bg-secondary">
        <Users size={32} className="text-purple-400" />
      </div>
      <div className="space-y-2">
        <h2 className="text-lg font-semibold text-text-primary">
          No owned assets yet for {personName}
        </h2>
        <p className="text-sm text-text-secondary max-w-[400px]">
          Re-run entity extraction from Settings → AI to discover ownership from your documents.
        </p>
      </div>
      <div className="flex items-center gap-3 pt-2">
        <Link
          to="/settings"
          className="inline-flex items-center gap-2 text-sm text-accent-primary hover:text-accent-hover transition-colors"
        >
          <Settings size={14} />
          Re-extract
        </Link>
        <span className="text-text-tertiary">&middot;</span>
        <Link
          to="/watched"
          className="inline-flex items-center gap-2 text-sm text-accent-primary hover:text-accent-hover transition-colors"
        >
          <Folder size={14} />
          Manage watched folders
        </Link>
      </div>
    </div>
  );
}

export default function OwnershipPage() {
  const { personId = "" } = useParams<{ personId: string }>();
  const { data, isLoading, isError, error, refetch } = useAllOwnedBy(personId);

  if (isLoading) {
    return <LoadingSkeleton />;
  }

  if (isError || !data) {
    const msg = error instanceof Error ? error.message : "Unknown error";
    return <ErrorState message={msg} onRetry={() => refetch()} />;
  }

  const { person, assetsByType, totalAssets } = data;

  if (totalAssets === 0) {
    return <EmptyOwnershipState personName={person.canonicalName} />;
  }

  return (
    <div className="space-y-8">
      {/* Header */}
      <div className="flex items-center gap-4">
        <div className="flex items-center justify-center h-12 w-12 rounded-lg bg-accent-subtle flex-shrink-0">
          <Users size={24} className="text-purple-400" />
        </div>
        <div className="space-y-1">
          <h1 className="page-title text-text-primary">{person.canonicalName}</h1>
          <p className="text-sm text-text-secondary">
            {totalAssets} owned {totalAssets === 1 ? "asset" : "assets"}
          </p>
        </div>
      </div>

      {/* Asset-type sections */}
      <div className="space-y-8">
        {ASSET_TYPE_ORDER.map((assetType) => (
          <OwnershipAssetSection
            key={assetType}
            assetType={assetType}
            assets={assetsByType[assetType] ?? []}
          />
        ))}
      </div>
    </div>
  );
}
