# Phase 11 Deferred Items

## Pre-existing Test Failures (out of scope for Plan 11-09)

These test failures existed BEFORE Plan 11-09 was executed. They are caused by the Phase 11 EntityChip refactor (Plan 11-04/05) changing `EntityChip` from a `<Link>` element to a `<button>` element. The tests in `DocumentPage.test.tsx` were written pre-Phase-11 and assert the old `<Link>` behavior.

| Test | File | Line | Failure reason |
|------|------|------|----------------|
| Test 4: Entity chips render as Links | `client/pages/DocumentPage.test.tsx` | ~205 | EntityChip is now `<button>`, not `<Link>` |
| Test 5: Entity with no canonicalId falls back | `client/pages/DocumentPage.test.tsx` | ~213 | Same — EntityChip `<button>` has no href |
| Phase 8 Test A: entity chip links render | `client/pages/DocumentPage.test.tsx` | ~259 | Same — `getByRole("link")` fails for `<button>` |

**Recommended fix:** Update these tests to assert `<button>` role and use `onClick` behavior, matching the current EntityChip implementation. Assign to Plan 11 v2 or a dedicated test-fixup plan.

## TODO: Plan 11 v2 — SearchPage hydration from ?q= URL param

`buildSavedSearchUrl` in `Sidebar.tsx` sets `?q=` param in URL, but `SearchPage.tsx` reads `query` from local `useState` (initialized to `""`), not from URL params on mount. Entity filters are restored correctly via `useSearchParams`, but the query string is not.

**Workaround in Phase 11 v1:** Saved search restores entity filters (the primary saved state) correctly. The `q=` param is present in the URL but not consumed.

**Fix:** Add a `useEffect` in `SearchPage.tsx` to call `setQuery(searchParams.get("q") ?? "")` on initial mount.
