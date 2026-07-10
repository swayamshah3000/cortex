/**
 * EntitiesPage — /entities route.
 * Displays all canonical entities grouped by type with a 7-pill filter bar.
 * Analog: SpacesPage.tsx
 */

import { useState, useMemo } from "react";
import { Network, AlertCircle } from "lucide-react";
import { useEntities, useEntitiesByType } from "@/hooks/useTauri";
import { EntityCard } from "@/components/entities/EntityCard";
import { EntityTypeFilterBar } from "@/components/entities/EntityTypeFilterBar";
import type { EntitySummary } from "@/lib/types";

const ENTITY_TYPE_ORDER = [
  "person",
  "organization",
  "location",
  "date",
  "amount",
  "email",
];

/** Skeleton grid for loading state (mirrors SpacesPage.tsx SkeletonGrid) */
function SkeletonGrid() {
  return (
    <div className="space-y-8">
      {Array.from({ length: 3 }).map((_, groupIdx) => (
        <div key={groupIdx} className="space-y-3">
          <div className="h-5 w-32 rounded bg-bg-tertiary animate-pulse" />
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
            {Array.from({ length: 6 }).map((_, i) => (
              <div key={i} className="card p-4 animate-pulse">
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
        </div>
      ))}
    </div>
  );
}

export default function EntitiesPage() {
  const [filter, setFilter] = useState<string>("all");

  const {
    data: allEntities,
    isLoading: allLoading,
    isError: allError,
    refetch,
  } = useEntities();

  const {
    data: filteredEntities,
    isLoading: filteredLoading,
  } = useEntitiesByType(filter === "all" ? "" : filter);

  const isLoading = filter === "all" ? allLoading : filteredLoading;

  // Group by entity type when filter is "all"
  const groupedEntities = useMemo<Map<string, EntitySummary[]>>(() => {
    if (filter !== "all" || !allEntities) return new Map();
    const map = new Map<string, EntitySummary[]>();
    for (const entity of allEntities) {
      const group = map.get(entity.entityType) ?? [];
      group.push(entity);
      map.set(entity.entityType, group);
    }
    // Sort each group by document count descending
    for (const [key, list] of map) {
      map.set(
        key,
        list.slice().sort((a, b) => b.documentCount - a.documentCount),
      );
    }
    return map;
  }, [allEntities, filter]);

  // Error state
  if (allError) {
    return (
      <div className="flex flex-col items-center justify-center min-h-[60vh] text-center space-y-4">
        <AlertCircle size={40} className="text-red-400" />
        <div className="space-y-2">
          <p className="text-text-primary font-medium section-header">Could not load entities</p>
          <p className="text-text-secondary text-sm">
            Try again or check that scanning has completed.
          </p>
        </div>
        <button
          type="button"
          onClick={() => refetch()}
          className="px-4 py-2 rounded-lg border border-border-primary text-text-secondary hover:bg-bg-tertiary transition-colors text-sm"
        >
          Retry
        </button>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Header */}
      <div>
        <h1 className="page-title text-text-primary">Entities</h1>
        <p className="text-text-secondary text-sm mt-1">
          Click any entity to see every document mentioning it.
        </p>
      </div>

      {/* Filter bar */}
      <EntityTypeFilterBar active={filter} onSelect={setFilter} />

      {/* Content */}
      {isLoading ? (
        <SkeletonGrid />
      ) : filter === "all" ? (
        // Grouped view
        groupedEntities.size === 0 ? (
          <div className="flex flex-col items-center justify-center min-h-[60vh] text-center space-y-4">
            <div className="p-4 rounded-full bg-bg-secondary">
              <Network size={40} className="text-text-tertiary" />
            </div>
            <div className="space-y-2">
              <p className="text-text-primary font-medium section-header">No entities yet</p>
              <p className="text-text-secondary text-sm max-w-md">
                Once Cortex finishes scanning your documents, people, places, and organizations
                will appear here.
              </p>
            </div>
          </div>
        ) : (
          <div className="space-y-8">
            {ENTITY_TYPE_ORDER.filter((type) => groupedEntities.has(type)).map((type) => {
              const entities = groupedEntities.get(type) ?? [];
              return (
                <div key={type} className="space-y-3">
                  <h2 className="section-header text-text-primary capitalize">
                    {type.charAt(0).toUpperCase() + type.slice(1)}{" "}
                    <span className="text-text-tertiary text-sm font-normal">
                      ({entities.length})
                    </span>
                  </h2>
                  <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
                    {entities.map((entity) => (
                      <EntityCard key={entity.id} entity={entity} />
                    ))}
                  </div>
                </div>
              );
            })}
          </div>
        )
      ) : (
        // Single-type filtered view
        !filteredEntities || filteredEntities.length === 0 ? (
          <div className="flex flex-col items-center justify-center min-h-[40vh] text-center space-y-4">
            <Network size={40} className="text-text-tertiary" />
            <p className="text-text-secondary text-sm">No entities of this type found.</p>
          </div>
        ) : (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
            {filteredEntities
              .slice()
              .sort((a, b) => b.documentCount - a.documentCount)
              .map((entity) => (
                <EntityCard key={entity.id} entity={entity} />
              ))}
          </div>
        )
      )}
    </div>
  );
}
