import { useState, useMemo } from "react";
import { useNavigate } from "react-router-dom";
import { Tag as TagIcon, LayoutGrid, List, Filter } from "lucide-react";
import { useTags } from "../hooks/useTauri";
import type { Tag } from "../lib/types";
import { TopicFilterBar } from "../components/search/TopicFilterBar";

type ViewMode = "cloud" | "list";
type TagFilter = "all" | "auto" | "user";

function getTagFontSize(count: number, min: number, max: number): number {
  if (min === max) return 20;
  const scale = (count - min) / (max - min);
  return 14 + scale * 18; // 14px to 32px
}

export default function TagsPage() {
  const { data: tags, isLoading } = useTags();
  const navigate = useNavigate();
  const [viewMode, setViewMode] = useState<ViewMode>("cloud");
  const [filter, setFilter] = useState<TagFilter>("all");
  // Phase 8 Plan 09: topic filter state — parent owns it, passed to TopicFilterBar.
  // Full backend integration (get_tags_by_topic) is deferred to Phase 11.
  // For now: when selectedTopic is set, a badge "Filtered by topic: {topic}" is shown
  // above the tag cloud as a visual indicator. Tag cloud counts remain unfiltered
  // (counts come from the get_tags IPC which aggregates across all docs).
  // TODO(Phase 11): replace visual marker with get_tags_by_topic IPC for true filtering.
  const [selectedTopic, setSelectedTopic] = useState<string | null>(null);

  const filtered = useMemo(() => {
    if (!tags) return [];
    if (filter === "all") return tags;
    return tags.filter((t) => t.tagType === filter);
  }, [tags, filter]);

  const { minCount, maxCount } = useMemo(() => {
    if (!filtered.length) return { minCount: 0, maxCount: 0 };
    const counts = filtered.map((t) => t.documentCount);
    return { minCount: Math.min(...counts), maxCount: Math.max(...counts) };
  }, [filtered]);

  function handleTagClick(tag: Tag) {
    navigate(`/search?tag=${encodeURIComponent(tag.name)}`);
  }

  if (isLoading) {
    return (
      <div className="space-y-6">
        <div className="space-y-2">
          <h1 className="page-title text-text-primary">Tags</h1>
          <p className="text-text-secondary">Loading tags...</p>
        </div>
        <div className="card p-8 animate-pulse">
          <div className="flex flex-wrap gap-3">
            {Array.from({ length: 8 }).map((_, i) => (
              <div key={i} className="h-8 rounded-full bg-bg-tertiary" style={{ width: `${60 + i * 15}px` }} />
            ))}
          </div>
        </div>
      </div>
    );
  }

  if (!tags || tags.length === 0) {
    return (
      <div className="flex items-center justify-center min-h-[60vh]">
        <div className="text-center space-y-4">
          <div className="mx-auto w-16 h-16 rounded-full bg-bg-tertiary flex items-center justify-center">
            <TagIcon size={32} className="text-text-tertiary" />
          </div>
          <h2 className="text-xl font-semibold text-text-primary">No tags yet</h2>
          <p className="text-text-secondary max-w-sm">
            Tags will appear automatically as documents are indexed and analyzed.
          </p>
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div className="space-y-2">
          <h1 className="page-title text-text-primary">Tags</h1>
          <p className="text-text-secondary">
            {filtered.length} tag{filtered.length !== 1 ? "s" : ""} across your documents
          </p>
        </div>
        <div className="flex items-center gap-2">
          {/* Filter */}
          <div className="flex items-center gap-1.5">
            <Filter size={14} className="text-text-tertiary" />
            <select
              value={filter}
              onChange={(e) => setFilter(e.target.value as TagFilter)}
              className="text-sm bg-bg-secondary border border-border-primary rounded-lg px-3 py-1.5 text-text-primary focus:outline-none focus:ring-1 focus:ring-accent-primary"
            >
              <option value="all">All Tags</option>
              <option value="auto">Auto-generated</option>
              <option value="user">User-created</option>
            </select>
          </div>
          {/* View toggle */}
          <div className="flex border border-border-primary rounded-lg overflow-hidden">
            <button
              type="button"
              onClick={() => setViewMode("cloud")}
              className={`p-1.5 ${viewMode === "cloud" ? "bg-accent-subtle text-accent-primary" : "text-text-tertiary hover:text-text-primary"}`}
              title="Cloud view"
            >
              <LayoutGrid size={16} />
            </button>
            <button
              type="button"
              onClick={() => setViewMode("list")}
              className={`p-1.5 ${viewMode === "list" ? "bg-accent-subtle text-accent-primary" : "text-text-tertiary hover:text-text-primary"}`}
              title="List view"
            >
              <List size={16} />
            </button>
          </div>
        </div>
      </div>

      {/* Phase 8 Plan 09: Topic filter bar — between tag-type filter row and tag cloud.
          Full backend integration (get_tags_by_topic) deferred to Phase 11. */}
      <TopicFilterBar selected={selectedTopic} onSelect={setSelectedTopic} />

      {/* Visual marker shown when a topic filter is active (Phase 11 will make this functional) */}
      {selectedTopic && (
        <div className="flex items-center gap-2 px-3 py-2 rounded-lg bg-accent-primary/10 border border-accent-primary/20">
          <span className="text-xs text-accent-primary">
            Filtered by topic: <span className="font-semibold">{selectedTopic.replace(/_/g, " ")}</span>
          </span>
          <span className="text-xs text-text-tertiary ml-1">
            — tag counts across all docs shown (Phase 11 will narrow by topic)
          </span>
          <button
            className="ml-auto text-xs text-accent-primary hover:text-accent-hover"
            onClick={() => setSelectedTopic(null)}
            aria-label="Clear topic filter"
          >
            Clear
          </button>
        </div>
      )}

      {viewMode === "cloud" ? (
        <div className="card p-8">
          <div className="flex flex-wrap items-center justify-center gap-3">
            {filtered.map((tag) => (
              <button
                key={tag.id}
                type="button"
                onClick={() => handleTagClick(tag)}
                className="px-3 py-1.5 rounded-full hover:opacity-80 transition-opacity cursor-pointer"
                style={{
                  fontSize: `${getTagFontSize(tag.documentCount, minCount, maxCount)}px`,
                  backgroundColor: `${tag.color}20`,
                  color: tag.color,
                }}
                title={`${tag.name} (${tag.documentCount} documents)`}
              >
                {tag.name}
              </button>
            ))}
          </div>
        </div>
      ) : (
        <div className="card overflow-hidden">
          <table className="w-full">
            <thead>
              <tr className="border-b border-border-primary text-left">
                <th className="px-4 py-3 text-xs font-medium text-text-tertiary uppercase tracking-wider">Tag</th>
                <th className="px-4 py-3 text-xs font-medium text-text-tertiary uppercase tracking-wider">Type</th>
                <th className="px-4 py-3 text-xs font-medium text-text-tertiary uppercase tracking-wider text-right">Documents</th>
              </tr>
            </thead>
            <tbody>
              {filtered.map((tag) => (
                <tr
                  key={tag.id}
                  onClick={() => handleTagClick(tag)}
                  className="border-b border-border-primary last:border-0 hover:bg-bg-secondary cursor-pointer transition-colors"
                >
                  <td className="px-4 py-3">
                    <div className="flex items-center gap-2">
                      <div
                        className="w-2.5 h-2.5 rounded-full flex-shrink-0"
                        style={{ backgroundColor: tag.color }}
                      />
                      <span className="text-sm font-medium text-text-primary">{tag.name}</span>
                    </div>
                  </td>
                  <td className="px-4 py-3">
                    <span
                      className={`text-xs px-2 py-0.5 rounded-full ${
                        tag.tagType === "auto"
                          ? "bg-blue-500/10 text-blue-400"
                          : "bg-green-500/10 text-green-400"
                      }`}
                    >
                      {tag.tagType}
                    </span>
                  </td>
                  <td className="px-4 py-3 text-sm text-text-secondary text-right">
                    {tag.documentCount}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}
