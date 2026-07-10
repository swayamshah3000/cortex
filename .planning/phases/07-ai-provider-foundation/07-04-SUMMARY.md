---
phase: 07-ai-provider-foundation
plan: "04"
subsystem: frontend
tags: [typescript, react-query, zustand, tauri, ipc, ai, provider, hooks, tdd]
dependency_graph:
  requires:
    - 8 Tauri IPC commands from Plan 03 (list_providers, connect_provider, disconnect_provider,
      set_active_provider, get_active_provider, save_setup_token, test_connection, chat)
    - "api-key" kebab-case wire format fix from Plan 03 (ProviderAuthStatus.method)
    - tauriInvoke helper + queryKeys factory from existing useTauri.ts (Phase 4)
  provides:
    - 6 TypeScript interfaces mirroring Rust IPC structs (types.ts)
    - 6 React Query hooks for AI provider operations (useTauri.ts)
    - queryKeys.providers + queryKeys.activeProvider (useTauri.ts)
    - useAiBannerStore (session-only Zustand store, no persist) (stores.ts)
    - mockProviders array for browser dev mode (mock-data.ts)
    - stores.test.ts with 3 vitest tests enforcing no-persist invariant
  affects:
    - Plans 05 and 06 (consume useProviders, useConnectProvider, useDisconnectProvider,
      useSetActiveProvider, useSaveSetupToken, useTestConnection, useAiBannerStore)
tech-stack:
  added: []
  patterns:
    - React Query useQuery with staleTime for provider status (30s — credential state is stable)
    - React Query useMutation with dual invalidateQueries (providers + activeProvider)
    - tauriInvoke third-arg mock fallback for browser dev mode (mockProviders[0] or literal)
    - Zustand create<T> without persist middleware (session-only D-15 contract)
    - Vitest store API surface test (api.persist === undefined as no-persist guard)
key-files:
  created:
    - client/lib/stores.test.ts (37 lines — 3 vitest tests for useAiBannerStore)
  modified:
    - client/lib/types.ts (added 6 interfaces in Phase 7 section)
    - client/lib/stores.ts (added useAiBannerStore at end, no persist wrapper)
    - client/hooks/useTauri.ts (added 3 imports, 2 queryKeys, 6 hooks)
    - client/lib/mock-data.ts (added mockProviders ProviderAuthStatus[] with 4 entries)
decisions:
  - "useAiBannerStore uses create<T> not create<T>()() — no persist middleware (D-15: banner resets each launch until provider connected)"
  - "useTestConnection does not invalidate queryKeys.providers — test is read-only, does not change auth state"
  - "mock fallback for mutations: useConnectProvider returns mockProviders[0] (realistic success path); void mutations return undefined as void"
  - "method field in ProviderAuthStatus comment explicitly documents kebab-case: 'api-key' not 'apikey' (cross-plan data contract)"
metrics:
  duration: ~8min
  completed: "2026-07-01"
  tasks: 2
  files: 5
---

# Phase 7 Plan 04: Frontend Foundation (Hooks + Types + Store) Summary

**6 TypeScript interfaces + 6 React Query hooks + useAiBannerStore (session-only) + mockProviders fixture — 8 tests pass (3 new), bun typecheck clean.**

## Performance

- **Duration:** ~8 minutes
- **Completed:** 2026-07-01
- **Tasks:** 2 completed / 2 total
- **Files modified:** 5

## Accomplishments

- Added 6 TypeScript interfaces to `client/lib/types.ts` mirroring the Rust IPC structs from Plan 03:
  `ProviderAuthStatus`, `ConnectProviderRequest`, `OAuthStartResult`, `AiServiceMessage`, `AiChatRequest`, `AiChatResponse`
- Added `useAiBannerStore` to `client/lib/stores.ts` — session-only Zustand store (no `persist` middleware), enforcing D-15: banner returns on every app launch until a provider is connected
- Created `client/lib/stores.test.ts` with 3 vitest tests verifying initial state, `dismiss()` behavior, and no-persist invariant (`api.persist === undefined`)
- Extended `queryKeys` in `useTauri.ts` with `providers` and `activeProvider` entries
- Added 6 React Query hooks to `useTauri.ts`: `useProviders` (query), `useConnectProvider`, `useDisconnectProvider`, `useSetActiveProvider`, `useSaveSetupToken`, `useTestConnection` (mutations)
- Added `mockProviders: ProviderAuthStatus[]` to `mock-data.ts` — 4 entries reflecting a realistic mixed state (Anthropic OAuth + Ollama connected; OpenAI + Gemini not connected)

## 6 TypeScript Interfaces Added

| Interface | File | Mirrors Rust Struct |
|-----------|------|---------------------|
| `ProviderAuthStatus` | client/lib/types.ts | `auth/commands.rs::ProviderAuthStatus` (serde camelCase) |
| `ConnectProviderRequest` | client/lib/types.ts | `auth/commands.rs::LoginRequest` (method is kebab-case "api-key") |
| `OAuthStartResult` | client/lib/types.ts | `auth/oauth.rs::OAuthStartResult` |
| `AiServiceMessage` | client/lib/types.ts | `ai/service.rs::ServiceMessage` |
| `AiChatRequest` | client/lib/types.ts | `ai/service.rs::AIServiceRequest` |
| `AiChatResponse` | client/lib/types.ts | `ai/service.rs::AIServiceResponse` |

## 6 React Query Hooks Added

| Hook | Type | TanStack Query Key | IPC Command |
|------|------|--------------------|-------------|
| `useProviders` | useQuery | `queryKeys.providers` = `["providers"]` | `list_providers` |
| `useConnectProvider` | useMutation | invalidates `providers` + `activeProvider` | `connect_provider` |
| `useDisconnectProvider` | useMutation | invalidates `providers` + `activeProvider` | `disconnect_provider` |
| `useSetActiveProvider` | useMutation | invalidates `providers` + `activeProvider` | `set_active_provider` |
| `useSaveSetupToken` | useMutation | invalidates `providers` | `save_setup_token` |
| `useTestConnection` | useMutation | no invalidation (read-only probe) | `test_connection` |

Note: `get_active_provider` and `chat` IPC commands do not get dedicated hooks — the active provider is derived from `useProviders()` filter, and `chat` is invoked directly by Phase 8 features.

## useAiBannerStore No-Persist Enforcement

Test (`client/lib/stores.test.ts`):
- Test 1: Initial `isDismissed` is `false`
- Test 2: `dismiss()` flips `isDismissed` to `true`
- Test 3: `api.persist` is `undefined` (Zustand persist middleware adds `.persist` key to StoreApi; absence confirms no middleware)

All 3 tests pass. Store uses `create<AiBannerState>((set) => ...)` pattern (not `create<T>()(persist(...))`) — session-only per D-15.

## Mock Fallback Coverage

All `tauriInvoke` calls include a third-argument mock fallback — verified:

| Hook | Mock Fallback |
|------|---------------|
| `useProviders` | `() => mockProviders` (full ProviderAuthStatus[]) |
| `useConnectProvider` | `() => mockProviders[0]` (Anthropic connected — success path) |
| `useDisconnectProvider` | `() => undefined as void` |
| `useSetActiveProvider` | `() => undefined as void` |
| `useSaveSetupToken` | `() => ({ started: true, provider: "anthropic" })` |
| `useTestConnection` | `() => undefined as void` |

## Build and Test Results

```
bun typecheck: clean exit (tsc — 0 errors)
bun test --run: 8 pass, 0 fail (3 new stores tests + 5 existing utils tests)
```

## Task Commits

1. **Task 1: Add Phase 7 TS types and useAiBannerStore** — `bca13e6` (feat, TDD RED+GREEN)
2. **Task 2: Add 6 AI provider hooks to useTauri.ts and mockProviders fixture** — `cc95246` (feat)

## Files Created/Modified

- `client/lib/stores.test.ts` (created, 37 lines) — 3 vitest tests for useAiBannerStore including no-persist guard
- `client/lib/types.ts` (modified) — 6 new interfaces in Phase 7 section at bottom of file
- `client/lib/stores.ts` (modified) — useAiBannerStore added at end, no persist middleware
- `client/hooks/useTauri.ts` (modified) — 3 new type imports, 2 new queryKeys, 6 new hooks, mockProviders import
- `client/lib/mock-data.ts` (modified) — mockProviders ProviderAuthStatus[] added with 4 realistic entries

## Deviations from Plan

### Auto-fixed Issues

None.

### Manual Adjustments

**Test isolation improvement:** The plan's test for `starts with isDismissed = false` didn't reset state between tests. Since vitest runs tests in the same module scope, a prior `dismiss()` call in test 2 would cause test 1 to fail if ordering changed. Added `useAiBannerStore.setState({ isDismissed: false })` at the start of each test that depends on initial state. This strengthens the test isolation without changing the semantics — it's the same "simulate app reload" technique the plan already uses in test 3.

## Known Stubs

None. All hooks have real IPC command names matching the Plan 03 backend. Mock fallbacks are for browser dev mode only — production Tauri builds always have `window.__TAURI__` defined and use real IPC.

## Threat Flags

No new unplanned threat surface introduced. All IPC calls go through the existing `tauriInvoke` helper (T-07-14 accepted: mock fallback only fires when `window.__TAURI__` absent, which never occurs in production).

## Self-Check

- [x] `client/lib/types.ts` contains `ProviderAuthStatus`: CONFIRMED
- [x] `client/lib/types.ts` contains `ConnectProviderRequest`: CONFIRMED
- [x] `client/lib/types.ts` contains `OAuthStartResult`: CONFIRMED
- [x] `client/lib/types.ts` contains `AiServiceMessage`: CONFIRMED
- [x] `client/lib/types.ts` contains `AiChatRequest`: CONFIRMED
- [x] `client/lib/types.ts` contains `AiChatResponse`: CONFIRMED
- [x] `client/lib/stores.ts` exports `useAiBannerStore`: CONFIRMED
- [x] `useAiBannerStore` has NO `persist` wrapper: CONFIRMED (uses `create<AiBannerState>((set) =>`, not `create<T>()(persist(...))`)
- [x] `client/lib/stores.test.ts` exists with 3 tests: CONFIRMED
- [x] 3 tests pass: CONFIRMED (`bun test --run stores.test.ts` — 3 pass, 0 fail)
- [x] `useProviders` exported from useTauri.ts: CONFIRMED
- [x] `useConnectProvider` exported from useTauri.ts: CONFIRMED
- [x] `useDisconnectProvider` exported from useTauri.ts: CONFIRMED
- [x] `useSetActiveProvider` exported from useTauri.ts: CONFIRMED
- [x] `useSaveSetupToken` exported from useTauri.ts: CONFIRMED
- [x] `useTestConnection` exported from useTauri.ts: CONFIRMED
- [x] `queryKeys.providers` present: CONFIRMED
- [x] `queryKeys.activeProvider` present: CONFIRMED
- [x] `mockProviders` in mock-data.ts: CONFIRMED (4 entries)
- [x] `bun typecheck` passes: CONFIRMED (clean exit)
- [x] All 8 tests pass: CONFIRMED (8 pass, 0 fail)
- [x] Commits bca13e6 and cc95246 exist: CONFIRMED

## Self-Check: PASSED
---
phase: 07-ai-provider-foundation
plan: "04"
subsystem: frontend
tags: [typescript, react-query, zustand, tauri, ipc, ai, provider, hooks, tdd]
dependency_graph:
  requires:
    - 8 Tauri IPC commands from Plan 03 (list_providers, connect_provider, disconnect_provider,
      set_active_provider, get_active_provider, save_setup_token, test_connection, chat)
    - "api-key" kebab-case wire format fix from Plan 03 (ProviderAuthStatus.method)
    - tauriInvoke helper + queryKeys factory from existing useTauri.ts (Phase 4)
  provides:
    - 6 TypeScript interfaces mirroring Rust IPC structs (types.ts)
    - 6 React Query hooks for AI provider operations (useTauri.ts)
    - queryKeys.providers + queryKeys.activeProvider (useTauri.ts)
    - useAiBannerStore (session-only Zustand store, no persist) (stores.ts)
    - mockProviders array for browser dev mode (mock-data.ts)
    - stores.test.ts with 3 vitest tests enforcing no-persist invariant
  affects:
    - Plans 05 and 06 (consume useProviders, useConnectProvider, useDisconnectProvider,
      useSetActiveProvider, useSaveSetupToken, useTestConnection, useAiBannerStore)
tech-stack:
  added: []
  patterns:
    - React Query useQuery with staleTime for provider status (30s — credential state is stable)
    - React Query useMutation with dual invalidateQueries (providers + activeProvider)
    - tauriInvoke third-arg mock fallback for browser dev mode (mockProviders[0] or literal)
    - Zustand create<T> without persist middleware (session-only D-15 contract)
    - Vitest store API surface test (api.persist === undefined as no-persist guard)
key-files:
  created:
    - client/lib/stores.test.ts (37 lines — 3 vitest tests for useAiBannerStore)
  modified:
    - client/lib/types.ts (added 6 interfaces in Phase 7 section)
    - client/lib/stores.ts (added useAiBannerStore at end, no persist wrapper)
    - client/hooks/useTauri.ts (added 3 imports, 2 queryKeys, 6 hooks)
    - client/lib/mock-data.ts (added mockProviders ProviderAuthStatus[] with 4 entries)
decisions:
  - "useAiBannerStore uses create<T> not create<T>()() — no persist middleware (D-15: banner resets each launch until provider connected)"
  - "useTestConnection does not invalidate queryKeys.providers — test is read-only, does not change auth state"
  - "mock fallback for mutations: useConnectProvider returns mockProviders[0] (realistic success path); void mutations return undefined as void"
  - "method field in ProviderAuthStatus comment explicitly documents kebab-case: 'api-key' not 'apikey' (cross-plan data contract)"
metrics:
  duration: ~8min
  completed: "2026-07-01"
  tasks: 2
  files: 5
---

# Phase 7 Plan 04: Frontend Foundation (Hooks + Types + Store) Summary

**6 TypeScript interfaces + 6 React Query hooks + useAiBannerStore (session-only) + mockProviders fixture — 8 tests pass (3 new), bun typecheck clean.**

## Performance

- **Duration:** ~8 minutes
- **Completed:** 2026-07-01
- **Tasks:** 2 completed / 2 total
- **Files modified:** 5

## Accomplishments

- Added 6 TypeScript interfaces to `client/lib/types.ts` mirroring the Rust IPC structs from Plan 03:
  `ProviderAuthStatus`, `ConnectProviderRequest`, `OAuthStartResult`, `AiServiceMessage`, `AiChatRequest`, `AiChatResponse`
- Added `useAiBannerStore` to `client/lib/stores.ts` — session-only Zustand store (no `persist` middleware), enforcing D-15: banner returns on every app launch until a provider is connected
- Created `client/lib/stores.test.ts` with 3 vitest tests verifying initial state, `dismiss()` behavior, and no-persist invariant (`api.persist === undefined`)
- Extended `queryKeys` in `useTauri.ts` with `providers` and `activeProvider` entries
- Added 6 React Query hooks to `useTauri.ts`: `useProviders` (query), `useConnectProvider`, `useDisconnectProvider`, `useSetActiveProvider`, `useSaveSetupToken`, `useTestConnection` (mutations)
- Added `mockProviders: ProviderAuthStatus[]` to `mock-data.ts` — 4 entries reflecting a realistic mixed state (Anthropic OAuth + Ollama connected; OpenAI + Gemini not connected)

## 6 TypeScript Interfaces Added

| Interface | File | Mirrors Rust Struct |
|-----------|------|---------------------|
| `ProviderAuthStatus` | client/lib/types.ts | `auth/commands.rs::ProviderAuthStatus` (serde camelCase) |
| `ConnectProviderRequest` | client/lib/types.ts | `auth/commands.rs::LoginRequest` (method is kebab-case "api-key") |
| `OAuthStartResult` | client/lib/types.ts | `auth/oauth.rs::OAuthStartResult` |
| `AiServiceMessage` | client/lib/types.ts | `ai/service.rs::ServiceMessage` |
| `AiChatRequest` | client/lib/types.ts | `ai/service.rs::AIServiceRequest` |
| `AiChatResponse` | client/lib/types.ts | `ai/service.rs::AIServiceResponse` |

## 6 React Query Hooks Added

| Hook | Type | TanStack Query Key | IPC Command |
|------|------|--------------------|-------------|
| `useProviders` | useQuery | `queryKeys.providers` = `["providers"]` | `list_providers` |
| `useConnectProvider` | useMutation | invalidates `providers` + `activeProvider` | `connect_provider` |
| `useDisconnectProvider` | useMutation | invalidates `providers` + `activeProvider` | `disconnect_provider` |
| `useSetActiveProvider` | useMutation | invalidates `providers` + `activeProvider` | `set_active_provider` |
| `useSaveSetupToken` | useMutation | invalidates `providers` | `save_setup_token` |
| `useTestConnection` | useMutation | no invalidation (read-only probe) | `test_connection` |

Note: `get_active_provider` and `chat` IPC commands do not get dedicated hooks — the active provider is derived from `useProviders()` filter, and `chat` is invoked directly by Phase 8 features.

## useAiBannerStore No-Persist Enforcement

Test (`client/lib/stores.test.ts`):
- Test 1: Initial `isDismissed` is `false`
- Test 2: `dismiss()` flips `isDismissed` to `true`
- Test 3: `api.persist` is `undefined` (Zustand persist middleware adds `.persist` key to StoreApi; absence confirms no middleware)

All 3 tests pass. Store uses `create<AiBannerState>((set) => ...)` pattern (not `create<T>()(persist(...))`) — session-only per D-15.

## Mock Fallback Coverage

All `tauriInvoke` calls include a third-argument mock fallback — verified:

| Hook | Mock Fallback |
|------|---------------|
| `useProviders` | `() => mockProviders` (full ProviderAuthStatus[]) |
| `useConnectProvider` | `() => mockProviders[0]` (Anthropic connected — success path) |
| `useDisconnectProvider` | `() => undefined as void` |
| `useSetActiveProvider` | `() => undefined as void` |
| `useSaveSetupToken` | `() => ({ started: true, provider: "anthropic" })` |
| `useTestConnection` | `() => undefined as void` |

## Build and Test Results

```
bun typecheck: clean exit (tsc — 0 errors)
bun test --run: 8 pass, 0 fail (3 new stores tests + 5 existing utils tests)
```

## Task Commits

1. **Task 1: Add Phase 7 TS types and useAiBannerStore** — `bca13e6` (feat, TDD RED+GREEN)
2. **Task 2: Add 6 AI provider hooks to useTauri.ts and mockProviders fixture** — `cc95246` (feat)

## Files Created/Modified

- `client/lib/stores.test.ts` (created, 37 lines) — 3 vitest tests for useAiBannerStore including no-persist guard
- `client/lib/types.ts` (modified) — 6 new interfaces in Phase 7 section at bottom of file
- `client/lib/stores.ts` (modified) — useAiBannerStore added at end, no persist middleware
- `client/hooks/useTauri.ts` (modified) — 3 new type imports, 2 new queryKeys, 6 new hooks, mockProviders import
- `client/lib/mock-data.ts` (modified) — mockProviders ProviderAuthStatus[] added with 4 realistic entries

## Deviations from Plan

### Auto-fixed Issues

None.

### Manual Adjustments

**Test isolation improvement:** The plan's test for `starts with isDismissed = false` didn't reset state between tests. Since vitest runs tests in the same module scope, a prior `dismiss()` call in test 2 would cause test 1 to fail if ordering changed. Added `useAiBannerStore.setState({ isDismissed: false })` at the start of each test that depends on initial state. This strengthens the test isolation without changing the semantics — it's the same "simulate app reload" technique the plan already uses in test 3.

## Known Stubs

None. All hooks have real IPC command names matching the Plan 03 backend. Mock fallbacks are for browser dev mode only — production Tauri builds always have `window.__TAURI__` defined and use real IPC.

## Threat Flags

No new unplanned threat surface introduced. All IPC calls go through the existing `tauriInvoke` helper (T-07-14 accepted: mock fallback only fires when `window.__TAURI__` absent, which never occurs in production).

## Self-Check

- [x] `client/lib/types.ts` contains `ProviderAuthStatus`: CONFIRMED
- [x] `client/lib/types.ts` contains `ConnectProviderRequest`: CONFIRMED
- [x] `client/lib/types.ts` contains `OAuthStartResult`: CONFIRMED
- [x] `client/lib/types.ts` contains `AiServiceMessage`: CONFIRMED
- [x] `client/lib/types.ts` contains `AiChatRequest`: CONFIRMED
- [x] `client/lib/types.ts` contains `AiChatResponse`: CONFIRMED
- [x] `client/lib/stores.ts` exports `useAiBannerStore`: CONFIRMED
- [x] `useAiBannerStore` has NO `persist` wrapper: CONFIRMED (uses `create<AiBannerState>((set) =>`, not `create<T>()(persist(...))`)
- [x] `client/lib/stores.test.ts` exists with 3 tests: CONFIRMED
- [x] 3 tests pass: CONFIRMED (`bun test --run stores.test.ts` — 3 pass, 0 fail)
- [x] `useProviders` exported from useTauri.ts: CONFIRMED
- [x] `useConnectProvider` exported from useTauri.ts: CONFIRMED
- [x] `useDisconnectProvider` exported from useTauri.ts: CONFIRMED
- [x] `useSetActiveProvider` exported from useTauri.ts: CONFIRMED
- [x] `useSaveSetupToken` exported from useTauri.ts: CONFIRMED
- [x] `useTestConnection` exported from useTauri.ts: CONFIRMED
- [x] `queryKeys.providers` present: CONFIRMED
- [x] `queryKeys.activeProvider` present: CONFIRMED
- [x] `mockProviders` in mock-data.ts: CONFIRMED (4 entries)
- [x] `bun typecheck` passes: CONFIRMED (clean exit)
- [x] All 8 tests pass: CONFIRMED (8 pass, 0 fail)
- [x] Commits bca13e6 and cc95246 exist: CONFIRMED

## Self-Check: PASSED

