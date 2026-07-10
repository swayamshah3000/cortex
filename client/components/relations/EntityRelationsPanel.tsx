/**
 * EntityRelationsPanel — outgoing + incoming triples for a canonical entity.
 *
 * Rendered below the Documents section on EntityDetailPage11 (Task 2).
 * Backed by useEntityRelations (Plan 06). Renders nothing (null) when the
 * entity has zero relations, or when the query errors (the outer page's
 * error state is authoritative).
 *
 * Phase 11.5 Plan 07, Task 1.
 */

import { EntityChip } from "@/components/entities/EntityChip";
import { DeleteTripleButton } from "@/components/relations/DeleteTripleButton";
import { Badge } from "@/components/ui/badge";
import { Skeleton } from "@/components/ui/skeleton";
import { useEntityRelations } from "@/hooks/useTauri";
import type { TripleWithEntities } from "@/lib/types";

export interface EntityRelationsPanelProps {
  entityId: string;
}

/** "owned_by" -> "owned by" */
function formatPredicate(p: string): string {
  return p.replace(/_/g, " ");
}

function TripleRow({ triple: t }: { triple: TripleWithEntities }) {
  return (
    <div className="flex items-center gap-2 flex-wrap text-sm py-1.5" data-testid="triple-row">
      <EntityChip
        entity={{
          value: t.subject.canonicalName,
          entityType: t.subject.entityType,
          class: t.subject.entityType,
        }}
      />
      <span className="text-accent-primary">&rarr;</span>
      <span className="text-text-secondary">{formatPredicate(t.triple.predicate)}</span>
      {t.triple.userAdded && (
        <Badge variant="secondary" className="text-xs">
          manual
        </Badge>
      )}
      <span className="text-accent-primary">&rarr;</span>
      <EntityChip
        entity={{
          value: t.object.canonicalName,
          entityType: t.object.entityType,
          class: t.object.entityType,
        }}
      />
      <Badge variant="outline" className="text-xs">
        {t.triple.docIds.length} {t.triple.docIds.length === 1 ? "doc" : "docs"}
      </Badge>
      <DeleteTripleButton
        tripleId={t.triple.id}
        affectedEntityIds={[t.triple.subjectId, t.triple.objectId]}
      />
    </div>
  );
}

export function EntityRelationsPanel({ entityId }: EntityRelationsPanelProps) {
  const { data, isLoading, isError } = useEntityRelations(entityId);

  if (isLoading) {
    return (
      <div className="space-y-2 animate-pulse" data-testid="entity-relations-loading">
        <Skeleton className="h-6 w-32" />
        <Skeleton className="h-10 w-full" />
        <Skeleton className="h-10 w-full" />
      </div>
    );
  }

  if (isError || !data) {
    // eslint-disable-next-line no-console
    console.error("EntityRelationsPanel: failed to load relations for", entityId);
    return null;
  }

  const { outgoing, incoming } = data;
  const total = outgoing.length + incoming.length;

  if (total === 0) {
    return null;
  }

  return (
    <section className="space-y-3" data-testid="entity-relations-section">
      <h2 className="text-lg font-semibold text-text-primary">Relations ({total})</h2>

      {outgoing.length > 0 && (
        <div className="space-y-1" data-testid="entity-relations-outgoing">
          <div className="text-xs text-text-tertiary uppercase tracking-wider">Outgoing</div>
          <div className="space-y-1">
            {outgoing.map((t) => (
              <TripleRow key={t.triple.id} triple={t} />
            ))}
          </div>
        </div>
      )}

      {incoming.length > 0 && (
        <div className="space-y-1" data-testid="entity-relations-incoming">
          <div className="text-xs text-text-tertiary uppercase tracking-wider">Incoming</div>
          <div className="space-y-1">
            {incoming.map((t) => (
              <TripleRow key={t.triple.id} triple={t} />
            ))}
          </div>
        </div>
      )}
    </section>
  );
}
