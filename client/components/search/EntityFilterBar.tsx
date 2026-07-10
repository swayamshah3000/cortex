import { EntityFilterPill } from "./EntityFilterPill";

/**
 * Renders a row of removable EntityFilterPill chips above search results when
 * one or more `?entity=` URL params are active.
 *
 * Returns null when activeEntityFilters is empty — the bar disappears completely
 * so it does not consume vertical space during an unfiltered search.
 *
 * "Clear all" button appears only when multiple filters are active (single-filter
 * removal is already handled by the pill's own X button).
 *
 * URL params are the source of truth (D-01, 11-CONTEXT.md) — this component
 * is stateless; parent owns searchParams via useSearchParams().
 */
export function EntityFilterBar({
  activeEntityFilters,
  onRemove,
  onClearAll,
}: {
  activeEntityFilters: string[];
  onRemove: (encodedParam: string) => void;
  onClearAll: () => void;
}) {
  if (activeEntityFilters.length === 0) return null;

  return (
    <div className="flex flex-wrap items-center gap-2">
      <span className="text-xs text-text-tertiary">Filtering by:</span>
      {activeEntityFilters.map((encoded) => (
        <EntityFilterPill key={encoded} encodedParam={encoded} onRemove={onRemove} />
      ))}
      {activeEntityFilters.length > 1 && (
        <button
          type="button"
          className="text-xs text-text-tertiary hover:text-text-secondary transition-colors"
          onClick={onClearAll}
        >
          Clear all
        </button>
      )}
    </div>
  );
}
