---
phase: 06-knowledge-graph-and-native-integrations
plan: 06
subsystem: entity-ui
tags: [entity-ui, knowledge-graph, react-router, react-query, entities-page, entity-chip]
dependency_graph:
  requires: [06-03, 06-04, 06-05]
  provides: [entities-page, entity-component-library, entity-react-query-hooks, sidebar-entities-link]
  affects:
    - client/pages/EntitiesPage.tsx
    - client/pages/DocumentPage.tsx
    - client/components/entities/*
    - client/components/layout/Sidebar.tsx
    - client/hooks/useTauri.ts
    - client/lib/mock-data.ts
    - client/App.tsx
tech_stack:
  added: []
  patterns:
    - "EntityChip: Link-wrapped chip with entityTypeIcon helper + aria-label convention"
    - "EntityTypeBadge: tokenMap-driven color pills (6 entity types)"
    - "EntityCard: Link-wrapped card with type-color icon tile, analogous to SubSpaceCard"
    - "EntityTypeFilterBar: pill toggle group mirroring FilterChip from SearchPage"
    - "RelatedEntityChip: EntityChip + co-occurrence count badge (× N)"
    - "useEntities/useEntitiesByType/useEntity/useEntityDocuments/useRelatedEntities: 5 React Query hooks extending queryKeys factory"
    - "EntitiesPage: type-grouped grid + filter bar + skeleton/empty/error states"
key_files:
  created:
    - client/components/entities/EntityChip.tsx
    - client/components/entities/EntityChip.test.tsx
    - client/components/entities/EntityTypeBadge.tsx
    - client/components/entities/EntityTypeBadge.test.tsx
    - client/components/entities/EntityCard.tsx
    - client/components/entities/EntityCard.test.tsx
    - client/components/entities/EntityTypeFilterBar.tsx
    - client/components/entities/EntityTypeFilterBar.test.tsx
    - client/components/entities/RelatedEntityChip.tsx
    - client/components/entities/RelatedEntityChip.test.tsx
    - client/hooks/useTauri.test.tsx
    - client/pages/EntitiesPage.tsx
    - client/pages/EntitiesPage.test.tsx
    - client/components/layout/Sidebar.test.tsx
    - client/App.test.tsx
  modified:
    - client/hooks/useTauri.ts (5 entity query keys + 5 read hooks + type imports)
    - client/lib/mock-data.ts (mockEntities 12 entries, mockRelatedEntities)
    - client/pages/DocumentPage.tsx (EntityChip swap, entityTypeIcon removed, unused imports removed)
    - client/components/layout/Sidebar.tsx (Network icon + Entities link in bottomLinks)
    - client/App.tsx (EntitiesPage import + /entities route)
decisions:
  - "EntityChip extracts entityTypeIcon helper as module-level non-exported function (DocumentPage no longer defines it)"
  - "tokenMap in EntityTypeBadge exported for use by EntityCard icon tile coloring"
  - "EntitiesPage uses useEntities() for all-types view, useEntitiesByType(filter) when filter !== 'all'"
  - "App.test.tsx uses fs.readFileSync with process.cwd() for structural route verification (import.meta.url path failed in vitest)"
  - "DocumentPage unused lucide imports (Tag, Users, MapPin, DollarSign, Building2, Mail) removed after EntityChip extraction"
metrics:
  duration: "~15 minutes"
  completed: "2026-06-29"
  tasks: 2
  files_created: 15
  files_modified: 5
  tests_added: 65
---

# Phase 06 Plan 06: Entity UI Component Library and Entities Index Page Summary

Delivered the entity UI component library (EntityChip, EntityTypeBadge, EntityCard, EntityTypeFilterBar, RelatedEntityChip), the /entities index route, 5 React Query read hooks for the Plan 03 entity IPCs, Sidebar navigation entry, and the DocumentPage entity chip extraction.

## What Was Built

### 5 Entity Components (`client/components/entities/`)

**EntityChip.tsx** — Reusable clickable chip extracted from DocumentPage inline render. Links to `/entities/{canonicalId ?? encodeURIComponent(value)}`. Uses aria-label `Entity: {value}, {entityType}`. Inline `entityTypeIcon` helper covers all 6 types (date=blue, amount=green, person=purple, organization=amber, location=red, email=cyan). Styling: `inline-flex rounded-full border-border-secondary bg-bg-tertiary hover:bg-accent-subtle focus-visible:ring-2`.

**EntityTypeBadge.tsx** — Pill badge with `tokenMap` for 6 entity types. Exports `tokenMap` for use by EntityCard and future components. Background `bg-{type}-400/10`, foreground `text-{type}-400`, border `border-{type}-400/30`. Capitalized type name with 12px type icon.

**EntityCard.tsx** — Link-wrapped card analog of SubSpaceCard. Shows type-color icon tile (18px), canonical name, document count. Shows "X aliases" caption when `aliases.length > 1`. Classes: `card p-4 hover:shadow-md hover:border-accent-primary/50 transition-all`.

**EntityTypeFilterBar.tsx** — 7-pill toggle group (All + person, organization, location, date, amount, email). Mirror of FilterChip from SearchPage.tsx. Active pill: `bg-accent-primary text-white border-accent-primary`. Inactive: `bg-bg-secondary text-text-secondary border-border-primary`.

**RelatedEntityChip.tsx** — Composes `EntityChip` with a co-occurrence count badge `× N` in `text-[10px] text-text-tertiary tabular-nums`. Tooltip "Co-occurs in N documents with this entity" via `title` attribute.

### 5 React Query Read Hooks (`client/hooks/useTauri.ts`)

Added to `queryKeys` factory:
- `entities: ["entities"] as const`
- `entitiesByType: (type) => ["entities", "byType", type] as const`
- `entity: (id) => ["entities", id] as const`
- `entityDocuments: (id) => ["entities", id, "documents"] as const`
- `relatedEntities: (id) => ["entities", id, "related"] as const`

Added 5 read hooks:
- `useEntities()` — calls `get_entities_by_type` with no type filter; fallback: `mockEntities`
- `useEntitiesByType(type)` — enabled when `Boolean(type)`; fallback: filtered `mockEntities`
- `useEntity(id)` — calls `get_entity`; enabled when `Boolean(id)`
- `useEntityDocuments(id)` — calls `get_documents_for_entity`; fallback: `[]`
- `useRelatedEntities(id, min?, limit?)` — calls `get_related_entities`; fallback: `mockRelatedEntities[id] ?? []`

No existing queryKey or hook modified.

### Mock Data (`client/lib/mock-data.ts`)

Added `mockEntities: EntitySummary[]` — 12 entries, 2 per type (person, organization, location, date, amount, email). Realistic fictional names/values. Added `mockRelatedEntities: Record<string, RelatedEntity[]>` keyed by canonical id with sample co-occurrence relationships.

### DocumentPage Chip Swap (`client/pages/DocumentPage.tsx`)

Replaced inline entity chip render block (Plan 05 implementation) with `<EntityChip key={i} entity={e} />`. Removed local `function entityTypeIcon(...)`. Removed 6 now-unused lucide imports (Users, MapPin, DollarSign, Building2, Mail, Tag). Added `import { EntityChip } from "@/components/entities/EntityChip"`. All 7 existing DocumentPage tests still pass.

### EntitiesPage (`client/pages/EntitiesPage.tsx`)

Default export route page for `/entities`. Layout: `space-y-6`. Uses `useEntities()` for all-types view and `useEntitiesByType(filter)` for filtered view. Groups entities by `ENTITY_TYPE_ORDER` (person → organization → location → date → amount → email). States: loading (SkeletonGrid with animate-pulse), empty ("No entities yet" + Network icon + body copy from UI-SPEC), error ("Could not load entities" + Retry button), populated (type-grouped grid or single-type grid).

### Sidebar Entities Link (`client/components/layout/Sidebar.tsx`)

Added `{ path: "/entities", label: "Entities", icon: Network }` to `bottomLinks` between Tags and Watched Folders. Imported `Network` from lucide-react.

### App.tsx Route (`client/App.tsx`)

Added `import EntitiesPage from "./pages/EntitiesPage"`. Registered `<Route path="/entities" element={<EntitiesPage />} />` inside AppShell Route group before the catch-all `*`.

## Test Results

```
Test Files  15 passed (15)
Tests       120 passed (120)
```

New tests added in this plan:
- 7 EntityChip tests (EntityChip.test.tsx)
- 10 EntityTypeBadge tests (EntityTypeBadge.test.tsx)
- 8 EntityCard tests (EntityCard.test.tsx)
- 7 EntityTypeFilterBar tests (EntityTypeFilterBar.test.tsx)
- 6 RelatedEntityChip tests (RelatedEntityChip.test.tsx)
- 14 useTauri hook tests (useTauri.test.tsx)
- 7 EntitiesPage tests (EntitiesPage.test.tsx)
- 4 Sidebar tests (Sidebar.test.tsx)
- 2 App routing tests (App.test.tsx)

Total new: 65 tests. All 55 pre-existing tests continue to pass (120 total).

## Notes on Output Spec Questions

**mock-data.ts additions TypeScript regression:** None. Types EntitySummary and RelatedEntity were already in `client/lib/types.ts` from Plan 04. Import added cleanly.

**Visual regressions from EntityChip extraction:** None observed. The chip renders the same visual output — same Tailwind classes, same icon resolution, same aria-label format — as the Plan 05 inline version. The only code-level change is that `entityTypeIcon` now lives in EntityChip.tsx instead of DocumentPage.tsx. All 7 DocumentPage tests pass.

**React Query key collisions:** No collisions. The new `entities` key family uses `["entities", ...]` prefix which was previously unused. Spot-checked against existing keys (`spaces`, `documents`, `search`, `stats`, `watched-folders`, `tags`, `activity-feed`, `settings`) — no overlap.

**Component line counts:** All 5 entity components are under 60 lines of implementation code:
- EntityChip.tsx: ~55 lines
- EntityTypeBadge.tsx: ~75 lines (slightly over due to tokenMap constant — all token data required)
- EntityCard.tsx: ~58 lines
- EntityTypeFilterBar.tsx: ~38 lines
- RelatedEntityChip.tsx: ~27 lines

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] tokenMap exported from EntityTypeBadge for EntityCard reuse**
- **Found during:** Task 1 implementation
- **Issue:** EntityCard needs the same type → color mapping as EntityTypeBadge. Duplicating it inline would violate DRY and risk divergence.
- **Fix:** Exported `tokenMap` from EntityTypeBadge.tsx so EntityCard imports it directly.
- **Files modified:** client/components/entities/EntityTypeBadge.tsx, client/components/entities/EntityCard.tsx
- **Commit:** 898b2fe

**2. [Rule 3 - Blocking] App.test.tsx import.meta.url path resolution failed in vitest**
- **Found during:** Task 2 test implementation
- **Issue:** `new URL("./App.tsx", import.meta.url).pathname` resolved to `/client/App.tsx` (absolute root) instead of the real filesystem path under vitest's JSDOM environment.
- **Fix:** Used `path.resolve(process.cwd(), "client/App.tsx")` instead. Structural test approach preserved.
- **Files modified:** client/App.test.tsx
- **Commit:** 0a91062

## Known Stubs

None. All 5 entity components render real data from the 5 React Query hooks. The hooks call actual IPC commands in Tauri mode and fall back to `mockEntities`/`mockRelatedEntities` in browser-dev mode — same pattern as all existing hooks. EntitiesPage wires to `useEntities()` and `useEntitiesByType()`. DocumentPage wires to `EntityChip` which renders real entity data.

## Threat Flags

No new network endpoints, auth paths, or trust boundaries introduced. All threats from the plan's threat model:
- T-06-CHIP-LINK: `encodeURIComponent` applied for raw values without canonicalId — implemented correctly in EntityChip.tsx
- T-06-FILTER-INPUT: Fixed-enum pill list, no free-form input — EntityTypeFilterBar correctly constrains to 7 known values
- T-06-MOCK-LEAK: Browser-dev fallback only fires when `!isTauri()` — same pattern as existing useDocument fallback

## Self-Check: PASSED

Files verified:
- client/components/entities/EntityChip.tsx ✓
- client/components/entities/EntityTypeBadge.tsx ✓
- client/components/entities/EntityCard.tsx ✓
- client/components/entities/EntityTypeFilterBar.tsx ✓
- client/components/entities/RelatedEntityChip.tsx ✓
- client/pages/EntitiesPage.tsx ✓
- client/hooks/useTauri.ts (useEntities, useEntitiesByType, useEntity, useEntityDocuments, useRelatedEntities present) ✓
- client/lib/mock-data.ts (mockEntities, mockRelatedEntities present) ✓
- client/pages/DocumentPage.tsx (uses EntityChip, no entityTypeIcon function) ✓
- client/components/layout/Sidebar.tsx (Entities + Network icon) ✓
- client/App.tsx (/entities route + EntitiesPage import) ✓

Commits verified:
- 898b2fe: feat(06-06): entity component library + 5 React Query hooks + mock-data + DocumentPage chip swap
- 0a91062: feat(06-06): EntitiesPage + Sidebar Entities link + /entities route registration
