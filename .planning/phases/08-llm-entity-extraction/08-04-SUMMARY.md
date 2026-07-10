---
phase: 08-llm-entity-extraction
plan: "04"
subsystem: frontend-types-hooks
tags: [typescript, react-query, mock-data, entity-extraction, phase-8]
dependency_graph:
  requires: []
  provides:
    - ExtractedEntity (TS interface, class/subclass/confidence optional)
    - ExtractedEntities (TS container interface)
    - ExtractionSettings (TS standalone interface)
    - Settings.extractionModel + Settings.useLlmExtraction
    - useExtractionSettings hook
    - useUpdateExtractionSettings hook
    - useTriggerEntityBackfill hook
    - queryKeys.extractionSettings
    - mockExtractionSettings export
    - mockDocumentEntitiesContainer export
  affects:
    - client/pages/SettingsPage.tsx (Plan 07 — ExtractionSettings component)
    - client/pages/DocumentPage.tsx (Plan 08 — entity sidebar with confidence expander)
tech_stack:
  added: []
  patterns:
    - TDD (RED → GREEN committed separately per task)
    - React Query useQuery/useMutation with browser-mode fallback
    - TypeScript interface extension with optional fields for backward compat
key_files:
  created:
    - client/lib/types.test.ts
    - client/hooks/useTauri.test.ts
  modified:
    - client/lib/types.ts
    - client/lib/mock-data.ts
    - client/hooks/useTauri.ts
decisions:
  - "ExtractedEntity class/subclass/confidence fields are all optional to preserve Phase 6 backward compatibility"
  - "Document.extractedEntities migrated from inline anonymous type to ExtractedEntity[] reference"
  - "useTriggerEntityBackfill uses direct isTauri() check (no-op in browser) rather than tauriInvoke fallback, because fire-and-forget has no meaningful return value to mock"
  - "mockDocuments[0].extractedEntities upgraded in-place (not a separate array) to give browser dev-mode realistic Phase 8 data from a single source of truth"
  - "PAN entity with confidence=0.65 added to mockDocuments[0] to populate the Also-found expander in browser mode without requiring a separate mock array"
metrics:
  duration: "~8 minutes"
  completed: "2026-07-03"
  tasks_completed: 2
  files_modified: 5
---

# Phase 08 Plan 04: Frontend types + hooks + mock-data Summary

**One-liner:** TypeScript mirror of Plan 01 Rust structs — ExtractedEntity/ExtractedEntities/ExtractionSettings interfaces + three React Query hooks (useExtractionSettings, useUpdateExtractionSettings, useTriggerEntityBackfill) with browser-mode mock fallbacks.

## What Was Built

### TypeScript Type Extensions (`client/lib/types.ts`)

New standalone `ExtractedEntity` interface (replaces inline anonymous type in `Document.extractedEntities`):
```typescript
export interface ExtractedEntity {
  label: string;
  value: string;
  entityType: string;
  canonicalId?: string;
  // Phase 8 additions — optional for Phase 6 backward compatibility
  class?: string;    // 8-class taxonomy: Person | Organization | Location | ...
  subclass?: string; // free-form: "aadhaar", "iban", "pan", etc.
  confidence?: number; // 0.0–1.0; < 0.7 shown under "Also found" expander
}
```

New `ExtractedEntities` container:
```typescript
export interface ExtractedEntities {
  entities: ExtractedEntity[];
  topic: string | null;
  tags: string[];
  entitiesVersion: number; // 2=BERT, 2.5=Pass1, 3=Pass1+Pass2
  language: string | null;
}
```

New `ExtractionSettings` standalone type:
```typescript
export interface ExtractionSettings {
  extractionModel: string;
  useLlmExtraction: boolean;
}
```

`Settings` extended with two non-optional fields (backend default_settings() guarantees presence):
```typescript
extractionModel: string;
useLlmExtraction: boolean;
```

### Mock Data Updates (`client/lib/mock-data.ts`)

- `mockExtractionSettings`: `{ extractionModel: "", useLlmExtraction: true }` (matches Rust defaults)
- `mockDocumentEntitiesContainer`: property-tax doc container with `topic="identity"`, `tags=["aadhaar","personal_id","property_tax"]`, `entitiesVersion=3`, `language="en"`
- `mockDocuments[0].extractedEntities` upgraded: 5 entities total including Person (confidence=0.91), Aadhaar Identifier (subclass="aadhaar", confidence=0.92), PAN Identifier (confidence=0.65 — populates "Also found" expander)
- `defaultSettings` extended with `extractionModel=""` and `useLlmExtraction=true`

### New React Query Hooks (`client/hooks/useTauri.ts`)

New query key:
```typescript
extractionSettings: ["extraction-settings"] as const
```

Three new hooks:
```typescript
// Fetches extraction settings; browser fallback = mockExtractionSettings
export function useExtractionSettings(): UseQueryResult<ExtractionSettings>

// Saves settings; invalidates extractionSettings on success
export function useUpdateExtractionSettings(): UseMutationResult<void, Error, ExtractionSettings>

// Fire-and-forget backfill trigger; no-op in browser mode (progress via event stream)
export function useTriggerEntityBackfill(): UseMutationResult<void>
```

## TDD Gate Compliance

**Task 1:**
- RED commit `92f8fa6`: 8 tests failing (mock exports absent, no confidence field)
- GREEN commit `6b067e9`: All 14 tests passing

**Task 2:**
- RED commit `fc252a5`: 14 hook tests failing (hooks not yet exported)
- GREEN commit `5d6fc60`: All 16 hook tests + 18 existing tests passing

Total: 34 tests in `useTauri.test.ts` + `useTauri.test.tsx`, 14 tests in `types.test.ts`, 69 total across all client test files — all green.

## Deviations from Plan

### Auto-fixed Issues

None — plan executed exactly as written.

### Notes

- The plan's "upgrade at least 3 entries in `mockEntities`" was interpreted as upgrading `mockDocuments[0].extractedEntities` (which uses `ExtractedEntity[]` shape with `label/value`) rather than `mockEntities` (which is `EntitySummary[]` with `canonicalName` — a different type from the KG). The "Also found" expander data lives in `Document.extractedEntities`, not in `EntitySummary`.
- The `isTauri` function was not imported in `useTauri.ts` before this plan — added it for `useTriggerEntityBackfill`'s direct browser-mode check.

## Threat Flags

None — no new network endpoints, auth paths, or file access patterns introduced. All new code is frontend TypeScript types and React Query hooks.

## Self-Check

Files created/modified:

- client/lib/types.ts — FOUND (exports verified by tsc + vitest)
- client/lib/mock-data.ts — FOUND (exports verified by vitest)
- client/hooks/useTauri.ts — FOUND (hooks verified by vitest)
- client/lib/types.test.ts — FOUND (14 tests passing)
- client/hooks/useTauri.test.ts — FOUND (16 tests passing)

Commits:
- 92f8fa6 — test(08-04): add failing tests for Phase 8 type shapes and mock data
- 6b067e9 — feat(08-04): extend TypeScript types and mock data for Phase 8 entity extraction
- fc252a5 — test(08-04): add failing tests for Phase 8 extraction settings hooks
- 5d6fc60 — feat(08-04): add useExtractionSettings, useUpdateExtractionSettings, useTriggerEntityBackfill hooks

## Self-Check: PASSED
