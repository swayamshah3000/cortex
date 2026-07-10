import { Link, useLocation } from "react-router-dom";
import {
  Home,
  Brain,
  Search,
  Clock,
  Star,
  Tag,
  Folder,
  BarChart3,
  Settings,
  ChevronLeft,
  ChevronRight,
  Network,
  Bookmark,
  Wallet,
  MessageSquare,
} from "lucide-react";
import { useMemo } from "react";
import { cn } from "@/lib/utils";
import { formatBytes } from "@/lib/utils";
import {
  useSpaces,
  useStats,
  useSavedSearches,
  useSavedSearchCounts,
  useTopPersonId,
} from "@/hooks/useTauri";
import { useSidebarStore, useCommandPaletteStore } from "@/lib/stores";
import type { SavedSearch } from "@/lib/types";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible";

// Assumed storage quota (configurable in settings later)
const STORAGE_QUOTA_BYTES = 5 * 1024 * 1024 * 1024; // 5 GB

/**
 * Reconstructs the /search URL from a SavedSearch's query + entity filters.
 * Phase 11 v1: entity + query round-trip only.
 * TODO Plan 11 v2: hydrate SearchPage query state from ?q= URL param
 * (SearchPage currently reads query from local useState, not from URL on mount).
 */
function buildSavedSearchUrl(ss: SavedSearch): string {
  const params = new URLSearchParams();
  if (ss.query) params.set("q", ss.query);
  (ss.filters.entities ?? []).forEach((e) => params.append("entity", e));
  const qs = params.toString();
  return qs ? `/search?${qs}` : "/search";
}

export function Sidebar() {
  const {
    isCollapsed,
    toggle: toggleCollapsed,
    expandedSpaceIds,
    toggleSpaceExpanded,
  } = useSidebarStore();
  const { open: openPalette } = useCommandPaletteStore();
  const location = useLocation();

  // Live data hooks
  const { data: spaces, isLoading: spacesLoading } = useSpaces();
  const { data: stats } = useStats();

  // Saved Searches — D-07, D-08 (ENEX-02, ENEX-04)
  const { data: savedSearches } = useSavedSearches();
  const savedSearchIds = useMemo(
    () => (savedSearches ?? []).map((s) => s.id),
    [savedSearches],
  );
  const { data: savedSearchCounts } = useSavedSearchCounts(savedSearchIds);

  // Owned by me quick link — Phase 11.5 ONTO-04.
  const topPersonId = useTopPersonId();

  const isActive = (path: string) => {
    return location.pathname === path || location.pathname.startsWith(path + "/");
  };

  // D-17: Top 5 top-level Spaces by documentCount.
  // RESEARCH.md pitfall #2: filter to top-level only (!parentId) before sorting.
  // Sub-spaces render beneath their parent, never as direct sidebar entries.
  const topLevelSpaces = spaces?.filter((s) => !s.parentId) ?? [];
  const sidebarSpaces = [...topLevelSpaces]
    .sort((a, b) => b.documentCount - a.documentCount)
    .slice(0, 5);

  // Storage display
  const indexSize = stats?.indexSize ?? 0;
  const storageLabel = `${formatBytes(indexSize)} / ${formatBytes(STORAGE_QUOTA_BYTES)}`;
  const storagePercent = STORAGE_QUOTA_BYTES > 0 ? Math.min((indexSize / STORAGE_QUOTA_BYTES) * 100, 100) : 0;

  const mainLinks = [
    { path: "/", label: "Dashboard", icon: Home },
    { path: "/spaces", label: "Smart Spaces", icon: Brain },
    { path: "/search", label: "Search", icon: Search },
    { path: "/chat", label: "Chat", icon: MessageSquare },
    { path: "/recent", label: "Recent", icon: Clock },
    { path: "/favorites", label: "Favorites", icon: Star },
  ];

  const bottomLinks = [
    { path: "/tags", label: "Tags", icon: Tag },
    { path: "/entities", label: "Entities", icon: Network },
    { path: "/watched", label: "Watched Folders", icon: Folder },
    { path: "/insights", label: "Insights", icon: BarChart3 },
    { path: "/settings", label: "Settings", icon: Settings },
  ];

  const NavLink = ({
    path,
    label,
    icon: Icon,
  }: {
    path: string;
    label: string;
    icon: React.ComponentType<{ size: number; className?: string }>;
  }) => (
    <Link
      to={path}
      className={cn(
        "group relative flex items-center gap-3 rounded-md px-3 py-2.5 text-sm font-medium transition-all duration-150",
        isActive(path)
          ? "bg-accent-primary text-white"
          : "text-text-secondary hover:bg-bg-tertiary hover:text-text-primary"
      )}
    >
      <Icon size={20} />
      {!isCollapsed && <span>{label}</span>}
      {isCollapsed && (
        <div className="absolute left-full ml-2 hidden rounded-md bg-bg-secondary px-2 py-1 text-xs text-text-primary shadow-lg group-hover:block whitespace-nowrap">
          {label}
        </div>
      )}
    </Link>
  );

  return (
    <aside
      className={cn(
        "fixed left-0 top-0 h-screen border-r border-border-primary bg-bg-primary transition-all duration-250 flex flex-col z-50",
        isCollapsed ? "w-20" : "w-60"
      )}
    >
      {/* Logo Section */}
      <div className="flex items-center justify-between gap-2 border-b border-border-primary px-4 py-4">
        <div className="flex items-center gap-2">
          <div className="flex h-8 w-8 items-center justify-center rounded-md bg-accent-primary">
            <Brain size={18} className="text-white" />
          </div>
          {!isCollapsed && (
            <span className="app-title text-text-primary">Cortex</span>
          )}
        </div>
      </div>

      {/* Quick Search — will open command palette in Plan 05 */}
      <div className="border-b border-border-primary px-3 py-4">
        <button
          onClick={openPalette}
          className={cn(
            "flex w-full items-center gap-2 rounded-md border border-border-primary bg-bg-secondary px-3 py-2 text-sm text-text-tertiary transition-colors hover:border-border-secondary hover:bg-bg-tertiary",
            isCollapsed && "justify-center"
          )}
        >
          <Search size={16} />
          {!isCollapsed && (
            <span className="flex-1 text-left">Cmd+K</span>
          )}
        </button>
      </div>

      {/* Main Navigation */}
      <nav className="flex-1 space-y-1 overflow-y-auto px-3 py-4">
        <div className="space-y-1">
          {mainLinks.map((link) => (
            <NavLink
              key={link.path}
              path={link.path}
              label={link.label}
              icon={link.icon}
            />
          ))}
        </div>

        {/* Spaces Section — driven by useSpaces()
         *
         * Interaction contract: chevron click toggles expandedSpaceIds; name click navigates.
         * Verified by end-to-end UX checkpoint in Plan 09.
         *
         * D-17: top 5 top-level spaces by documentCount (sub-spaces excluded from slot).
         * D-13: chevron toggles useSidebarStore.expandedSpaceIds — no navigation.
         * D-14: sub-count "(N)" inline after name, text-xs text-text-tertiary, ml-1.
         */}
        <div className="pt-4">
          {!isCollapsed && (
            <div className="px-3 py-2">
              <span className="text-xs font-semibold uppercase tracking-wider text-text-tertiary">
                Spaces
              </span>
            </div>
          )}
          <div className="space-y-1">
            {spacesLoading ? (
              // Loading skeleton: 4 placeholder lines
              Array.from({ length: 4 }).map((_, i) => (
                <div
                  key={i}
                  className="flex items-center gap-3 rounded-md px-3 py-2"
                >
                  <div className="h-2 w-2 rounded-full bg-bg-tertiary animate-pulse flex-shrink-0" />
                  {!isCollapsed && (
                    <div className="h-3 flex-1 rounded bg-bg-tertiary animate-pulse" />
                  )}
                </div>
              ))
            ) : sidebarSpaces.length > 0 ? (
              <>
                {sidebarSpaces.map((space) => {
                  const hasSubSpaces =
                    space.subSpaceIds != null && space.subSpaceIds.length > 0;
                  const isExpanded = expandedSpaceIds != null && expandedSpaceIds.has(space.id);

                  // Collapsed sidebar: only color dot — no name, no chevron, no sub-list
                  if (isCollapsed) {
                    return (
                      <Link
                        key={space.id}
                        to={`/spaces/${space.id}`}
                        className={cn(
                          "group flex items-center justify-center rounded-md px-3 py-2 transition-colors hover:bg-bg-tertiary",
                          isActive(`/spaces/${space.id}`)
                            ? "bg-bg-tertiary"
                            : ""
                        )}
                      >
                        <div
                          className="h-2 w-2 rounded-full flex-shrink-0"
                          style={{ backgroundColor: space.color }}
                        />
                      </Link>
                    );
                  }

                  // Expanded sidebar with Collapsible sub-space support
                  return (
                    <Collapsible
                      key={space.id}
                      open={isExpanded}
                      onOpenChange={() =>
                        toggleSpaceExpanded && toggleSpaceExpanded(space.id)
                      }
                    >
                      <div className="group flex items-center gap-1 rounded-md hover:bg-bg-tertiary">
                        {/* Space name + color dot + sub-count: clicking navigates */}
                        <Link
                          to={`/spaces/${space.id}`}
                          className={cn(
                            "flex flex-1 items-center gap-3 rounded-md px-3 py-2 text-sm font-medium transition-colors",
                            isActive(`/spaces/${space.id}`)
                              ? "bg-bg-tertiary text-text-primary"
                              : "text-text-secondary"
                          )}
                          onClick={(e) => e.stopPropagation()}
                        >
                          <div
                            className="h-2 w-2 rounded-full flex-shrink-0"
                            style={{ backgroundColor: space.color }}
                          />
                          <span className="flex-1 truncate">{space.name}</span>
                          {/* D-14: sub-count "(N)" inline, text-xs text-text-tertiary, ml-1 */}
                          {hasSubSpaces && (
                            <span className="ml-1 text-xs text-text-tertiary flex-shrink-0">
                              ({space.subSpaceIds!.length})
                            </span>
                          )}
                          <span className="text-xs text-text-tertiary">
                            {space.documentCount}
                          </span>
                        </Link>

                        {/* Chevron toggle button — only when sub-spaces exist (D-13) */}
                        {hasSubSpaces && (
                          <CollapsibleTrigger asChild>
                            <button
                              aria-label={
                                isExpanded
                                  ? "Collapse sub-spaces"
                                  : "Expand sub-spaces"
                              }
                              className="p-1 rounded hover:bg-bg-tertiary text-text-tertiary hover:text-text-secondary transition-colors opacity-0 group-hover:opacity-100"
                              style={{ minWidth: 44, minHeight: 44 }}
                              onClick={(e) => e.stopPropagation()}
                            >
                              {/* CSS transition-transform + rotate-90 (no Framer Motion per RESEARCH.md) */}
                              <ChevronRight
                                size={14}
                                className={cn(
                                  "transition-transform duration-150",
                                  isExpanded && "rotate-90"
                                )}
                              />
                            </button>
                          </CollapsibleTrigger>
                        )}
                      </div>

                      {/* Sub-space list: indented pl-8, 6px dot, text-xs (UI-SPEC §1) */}
                      <CollapsibleContent>
                        <div className="pl-8 space-y-0.5 pb-1">
                          {spaces
                            ?.filter((s) => s.parentId === space.id)
                            .map((sub) => (
                              <Link
                                key={sub.id}
                                to={`/spaces/${sub.id}`}
                                className={cn(
                                  "flex items-center gap-2 px-3 py-2 rounded-md text-xs transition-colors",
                                  isActive(`/spaces/${sub.id}`)
                                    ? "bg-bg-tertiary text-text-primary"
                                    : "text-text-tertiary hover:bg-bg-tertiary hover:text-text-secondary"
                                )}
                              >
                                {/* 6px dot (h-1.5 w-1.5) vs parent 8px (h-2 w-2) — smaller reinforces hierarchy */}
                                <div
                                  className="h-1.5 w-1.5 rounded-full flex-shrink-0"
                                  style={{ backgroundColor: sub.color }}
                                />
                                <span className="truncate">{sub.name}</span>
                              </Link>
                            ))}
                        </div>
                      </CollapsibleContent>
                    </Collapsible>
                  );
                })}
                {/* View All link: reflects top-level count, not total (D-17) */}
                {topLevelSpaces.length > 5 && (
                  <Link
                    to="/spaces"
                    className="block px-3 py-1.5 text-xs text-accent-primary hover:text-accent-hover transition-colors"
                  >
                    View All ({topLevelSpaces.length})
                  </Link>
                )}
              </>
            ) : (
              !isCollapsed && (
                <p className="px-3 py-2 text-xs text-text-tertiary">
                  No spaces yet
                </p>
              )
            )}
          </div>
        </div>

        {/* Saved Searches Section — D-07, D-08 (ENEX-02, ENEX-04)
         *
         * Renders a "Saved Searches" header + rows below the Spaces section.
         * Each row: Bookmark icon (text-accent-primary) + name + live count (N).
         * Count comes from useSavedSearchCounts(ids) with 30s TTL (ENEX-04);
         * falls back to docCountCache while the count query is loading.
         * Collapsed sidebar: only the Bookmark icon is shown for each row.
         * T-11-29 mitigation: 30s staleTime + React Query dedupe by sorted-id key.
         * T-11-31 mitigation: React JSX escapes; no dangerouslySetInnerHTML.
         */}
        <div className="pt-4">
          {!isCollapsed && (
            <div className="px-3 py-2">
              <span className="text-xs font-semibold uppercase tracking-wider text-text-tertiary">
                Saved Searches
              </span>
            </div>
          )}
          {savedSearches && savedSearches.length > 0 ? (
            <nav className="space-y-0.5 px-2 pb-2">
              {savedSearches.map((ss) => {
                const url = buildSavedSearchUrl(ss);
                const count = savedSearchCounts?.[ss.id] ?? ss.docCountCache;
                const active = isActive(url);
                return (
                  <Link
                    key={ss.id}
                    to={url}
                    className={cn(
                      "flex items-center gap-3 rounded-md px-3 py-2 text-sm font-medium transition-colors",
                      active
                        ? "bg-bg-tertiary text-text-primary"
                        : "text-text-secondary hover:bg-bg-tertiary hover:text-text-primary"
                    )}
                  >
                    <Bookmark size={14} className="text-accent-primary flex-shrink-0" />
                    {!isCollapsed && (
                      <>
                        <span className="flex-1 truncate">{ss.name}</span>
                        <span className="text-xs text-text-tertiary flex-shrink-0">({count})</span>
                      </>
                    )}
                  </Link>
                );
              })}
            </nav>
          ) : (
            !isCollapsed && (
              <p className="px-3 py-2 text-xs text-text-tertiary">No saved searches yet</p>
            )
          )}
        </div>

        {/* Owned by me quick link — Phase 11.5 ONTO-04.
         *
         * Hidden until at least one Person entity is indexed. Navigates to
         * /ownership/{topPersonId} (highest doc_count person = usually the user).
         * Discovery entry point for the /ownership/:personId route (Plan 11.5-07).
         */}
        {topPersonId && (
          <div className="pt-4">
            <Link
              to={`/ownership/${topPersonId}`}
              className={cn(
                "group relative flex items-center gap-3 rounded-md px-3 py-2.5 text-sm font-medium transition-all duration-150",
                isActive(`/ownership/${topPersonId}`)
                  ? "bg-accent-primary text-white"
                  : "text-text-secondary hover:bg-bg-tertiary hover:text-text-primary"
              )}
            >
              <Wallet size={20} />
              {!isCollapsed && <span>Owned by me</span>}
              {isCollapsed && (
                <div className="absolute left-full ml-2 hidden rounded-md bg-bg-secondary px-2 py-1 text-xs text-text-primary shadow-lg group-hover:block whitespace-nowrap">
                  Owned by me
                </div>
              )}
            </Link>
          </div>
        )}
      </nav>

      {/* Bottom Section */}
      <div className="border-t border-border-primary px-3 py-4 space-y-1">
        {bottomLinks.map((link) => (
          <NavLink
            key={link.path}
            path={link.path}
            label={link.label}
            icon={link.icon}
          />
        ))}
      </div>

      {/* Storage Bar — real index size from useStats() */}
      <div className="border-t border-border-primary px-3 py-3">
        <div className="space-y-2">
          {!isCollapsed && (
            <div className="text-xs text-text-tertiary">{storageLabel}</div>
          )}
          <div className="h-1.5 w-full rounded-full bg-bg-secondary overflow-hidden">
            <div
              className="h-full rounded-full bg-accent-primary transition-all"
              style={{ width: `${storagePercent}%` }}
            />
          </div>
        </div>
      </div>

      {/* Collapse Toggle */}
      <button
        onClick={toggleCollapsed}
        className="hidden sm:flex items-center justify-center border-t border-border-primary py-3 text-text-tertiary hover:text-text-secondary transition-colors w-full"
      >
        {isCollapsed ? (
          <ChevronRight size={18} />
        ) : (
          <ChevronLeft size={18} />
        )}
      </button>
    </aside>
  );
}
