/**
 * EntityDetailPage — /entities/:id
 *
 * Shows: breadcrumb, EntityDetailHeader (with rename), AliasChipList (with split),
 * Documents-mentioning section, Related-entities section.
 *
 * Plan 06-07 Task 1
 */

import { useState } from "react";
import { Link, useParams, useNavigate } from "react-router-dom";
import { ChevronRight, FolderOpen, Network } from "lucide-react";
import { toast } from "sonner";
import {
  useEntity,
  useEntityDocuments,
  useRelatedEntities,
  useRenameEntityCanonical,
  useSplitEntityAlias,
} from "@/hooks/useTauri";
import { EntityDetailHeader } from "@/components/entities/EntityDetailHeader";
import { AliasChipList } from "@/components/entities/AliasChipList";
import { SplitAliasDialog } from "@/components/entities/SplitAliasDialog";
import { RelatedEntityChip } from "@/components/entities/RelatedEntityChip";
import { DocumentRow } from "@/components/documents/DocumentRow";

// --- Skeleton -----------------------------------------------------------------

function SkeletonDetail() {
  return (
    <div className="space-y-8 animate-pulse">
      {/* Header skeleton */}
      <div className="flex items-start gap-4">
        <div className="h-14 w-14 rounded-lg bg-bg-tertiary" />
        <div className="flex-1 space-y-2">
          <div className="h-7 w-48 rounded bg-bg-tertiary" />
          <div className="h-4 w-24 rounded bg-bg-tertiary" />
        </div>
      </div>
      {/* Alias skeleton */}
      <div className="flex flex-wrap gap-2">
        {[1, 2, 3].map((i) => (
          <div key={i} className="h-7 w-24 rounded-full bg-bg-tertiary" />
        ))}
      </div>
      {/* Document rows skeleton */}
      <div className="space-y-2">
        {[1, 2, 3, 4].map((i) => (
          <div key={i} className="h-12 rounded-lg bg-bg-tertiary" />
        ))}
      </div>
    </div>
  );
}

// --- Page ---------------------------------------------------------------------

export default function EntityDetailPage() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();

  const { data: entity, isLoading: entityLoading, isError: entityError } = useEntity(id ?? "");
  const { data: documents, isLoading: docsLoading } = useEntityDocuments(id ?? "");
  const { data: related, isLoading: relatedLoading } = useRelatedEntities(id ?? "");

  const [splitTarget, setSplitTarget] = useState<string | null>(null);

  const renameMut = useRenameEntityCanonical();
  const splitMut = useSplitEntityAlias();

  const isLoading = entityLoading || docsLoading || relatedLoading;

  if (isLoading) return <SkeletonDetail />;

  if (entityError || !entity) {
    return (
      <div className="flex items-center justify-center min-h-[60vh]">
        <div className="text-center space-y-2">
          <p className="text-text-primary font-medium">Entity not found</p>
          <Link
            to="/entities"
            className="text-sm text-accent-primary hover:text-accent-hover transition-colors"
          >
            Back to Entities
          </Link>
        </div>
      </div>
    );
  }

  function handleRename(newName: string) {
    if (!id) return;
    renameMut.mutate(
      { id, newName },
      {
        onSuccess: (updated) => {
          toast.success(`Renamed to '${updated.canonicalName}'`);
        },
        onError: () => {
          toast.error("Could not rename entity. Try again.");
        },
      },
    );
  }

  function handleSplitConfirm() {
    if (!id || !splitTarget) return;
    const alias = splitTarget;
    splitMut.mutate(
      { canonicalId: id, alias },
      {
        onSuccess: (newEntity) => {
          toast.success(`Split '${alias}' into a new entity`, {
            action: {
              label: "View",
              onClick: () => navigate(`/entities/${newEntity.id}`),
            },
          });
          setSplitTarget(null);
        },
        onError: () => {
          toast.error("Could not split alias. Try again.");
          setSplitTarget(null);
        },
      },
    );
  }

  return (
    <div className="space-y-8">
      {/* Breadcrumb */}
      <nav className="flex items-center gap-1 text-sm text-text-tertiary">
        <Link to="/" className="hover:text-text-secondary transition-colors">
          Home
        </Link>
        <ChevronRight size={14} />
        <Link to="/entities" className="hover:text-text-secondary transition-colors">
          Entities
        </Link>
        <ChevronRight size={14} />
        <span className="text-text-primary font-medium">{entity.canonicalName}</span>
      </nav>

      {/* Header with inline rename */}
      <EntityDetailHeader entity={entity} onRename={handleRename} />

      {/* Aliases section */}
      <AliasChipList
        aliases={entity.aliases}
        canonicalName={entity.canonicalName}
        onSplit={(alias) => setSplitTarget(alias)}
      />

      {/* Two-column grid: Documents (7/12) + Related (5/12) */}
      <div className="lg:grid lg:grid-cols-12 gap-6 space-y-6 lg:space-y-0">
        {/* Documents mentioning this */}
        <div className="lg:col-span-7 space-y-3">
          <h2 className="section-header text-text-primary">
            Documents mentioning this{" "}
            {documents && documents.length > 0 && `(${documents.length})`}
          </h2>
          {!documents || documents.length === 0 ? (
            <div className="flex flex-col items-center justify-center py-12 text-center space-y-3">
              <FolderOpen size={32} className="text-text-tertiary" />
              <p className="text-text-secondary text-sm">
                No documents reference this entity.
              </p>
            </div>
          ) : (
            <div className="space-y-2">
              {documents.map((doc) => (
                <DocumentRow key={doc.id} doc={doc} />
              ))}
            </div>
          )}
        </div>

        {/* Related entities */}
        <div className="lg:col-span-5 space-y-3">
          <h2 className="section-header text-text-primary">Related entities</h2>
          {!related || related.length === 0 ? (
            <p className="text-text-secondary text-sm text-center py-8">
              No related entities yet
            </p>
          ) : (
            <div className="flex flex-wrap gap-2">
              {[...related]
                .sort((a, b) => b.coOccurrenceCount - a.coOccurrenceCount)
                .map((r) => (
                  <RelatedEntityChip key={r.entity.id} related={r} />
                ))}
            </div>
          )}
        </div>
      </div>

      {/* Split Alias Dialog */}
      <SplitAliasDialog
        alias={splitTarget ?? ""}
        open={splitTarget !== null}
        onOpenChange={(v) => {
          if (!v) setSplitTarget(null);
        }}
        onConfirm={handleSplitConfirm}
      />
    </div>
  );
}
