---
wave: 4
depends_on: [PLAN-01, PLAN-03]
requirements: [TAURI-05]
files_modified:
  - client/hooks/useTauri.ts
  - client/lib/tauri.ts
  - package.json
  - client/global.css
  - tailwind.config.ts
  - postcss.config.js
  - vite.config.ts
autonomous: true
---

# Plan 05: Dual-Mode Frontend Hooks and React/Tailwind Upgrades

## Goal

Frontend hooks detect Tauri runtime and switch between `invoke()` (Tauri) and mock data (browser dev). React 18 is upgraded to React 19 and TailwindCSS 3 to TailwindCSS 4. The app renders correctly in both modes: `pnpm dev` (browser with mock data) and `pnpm tauri dev` (desktop with IPC stubs).

## Context

- TAURI-05: Dual-mode frontend hooks (mock data in dev, Tauri invoke in production).
- Locked decision: Frontend runs standalone without Tauri (pnpm dev) — hooks fall back to mock data.
- Locked decision: React 18 -> 19 and TailwindCSS 3 -> 4 as part of Phase 1.
- Research Pitfall 4: Upgrade TailwindCSS first (CSS-only, isolated risk), then React 19.
- TailwindCSS 4 uses `@import "tailwindcss"` in CSS, not `@tailwind base/components/utilities`.
- Tauri 2 uses `@tauri-apps/api/core` for `invoke()` (not `@tauri-apps/api/tauri`).
- Existing mock data lives in `client/lib/mock-data.ts`.
- Existing hooks are in `client/hooks/`.
- No `useTauri.ts` exists yet — create it.

## Tasks

<task id="05.1" effort="M">
<title>Upgrade TailwindCSS 3 to 4</title>
<detail>
**Step 1: Run the official upgrade codemod**
```bash
pnpm dlx @tailwindcss/upgrade
```

If the codemod fails or produces unexpected output, do the manual migration:

**Step 2: Manual migration (if codemod insufficient)**

Update `package.json` devDependencies:
- Change `tailwindcss` to `^4`
- Remove `autoprefixer` (bundled in TW4)
- Remove `postcss` (TW4 handles it internally)
- Remove `tailwindcss-animate` (check compatibility or replace)
- Remove `@tailwindcss/typography` (check if v4 compatible or replace)

Delete `postcss.config.js` — TailwindCSS 4 does not use PostCSS plugin.

Update `vite.config.ts`:
- Add `@tailwindcss/vite` plugin:
```typescript
import tailwindcss from '@tailwindcss/vite'

export default defineConfig({
  plugins: [react(), tailwindcss()],
})
```
- Install: `pnpm add -D @tailwindcss/vite`

Update `client/global.css`:
- Replace `@tailwind base;`, `@tailwind components;`, `@tailwind utilities;` with `@import "tailwindcss";`
- Move theme customizations from `tailwind.config.ts` into `@theme { }` block in CSS
- Migrate custom CSS variables, color definitions, font families

The existing `tailwind.config.ts` may still work for TW4 in compatibility mode, but the preferred approach is CSS-first. Migrate what you can; leave complex plugins for a follow-up if needed.

**Step 3: Verify**
Run `pnpm dev` — app must render with correct styling. Check:
- Dark mode background colors render
- Sidebar layout is intact
- Typography (Inter, Plus Jakarta Sans) loads
- All Lucide icons display
- No console errors about missing Tailwind classes

If styling breaks, check for TW4 class name changes (some utility names changed in v4). Fix any regressions before proceeding.

**Done criteria (all must hold):**
- `client/global.css` contains `@import "tailwindcss"` and does NOT contain `@tailwind base`, `@tailwind components`, or `@tailwind utilities`
- `package.json` shows `tailwindcss` at `^4.x` (confirm with `pnpm list tailwindcss`)
- `pnpm dev` renders the app with correct styling (dark mode, sidebar, typography)
</detail>
</task>

<task id="05.2" effort="M">
<title>Upgrade React 18 to 19 and add Tauri JS API</title>
<detail>
**Step 1: Upgrade React**
```bash
pnpm add react@^19 react-dom@^19
pnpm add -D @types/react@^19 @types/react-dom@^19
```

If peer dependency conflicts arise with `@radix-ui/*` or `framer-motion`, use:
```bash
pnpm add react@^19 react-dom@^19 --force
```

Check `pnpm dev` — the app must render without runtime errors. React 19 is backward-compatible for client-only apps. Watch for:
- `ReactDOM.render` usage (should be `createRoot` — check `index.html` or entry point)
- Any deprecated lifecycle methods in class components (unlikely given modern codebase)

**Step 2: Add Tauri JS API**
```bash
pnpm add @tauri-apps/api@^2
```

**Step 3: Verify**
Run `pnpm dev` — app renders in browser.
Run `pnpm typecheck` (if `tsc` is configured) — no TypeScript errors.
Check React version in browser dev tools (React DevTools or `React.version`).
</detail>
</task>

<task id="05.3" effort="M">
<title>Create dual-mode Tauri hooks</title>
<detail>
Create `client/lib/tauri.ts` — runtime detection utility:

```typescript
import { invoke } from '@tauri-apps/api/core';

/**
 * Check if running inside Tauri desktop shell.
 * Returns false in plain browser (pnpm dev without Tauri).
 */
export const isTauri = (): boolean =>
  typeof window !== 'undefined' && '__TAURI__' in window;

/**
 * Type-safe invoke wrapper that only calls Tauri when available.
 * Falls back to the provided fallback function in browser mode.
 */
export async function tauriInvoke<T>(
  command: string,
  args?: Record<string, unknown>,
  fallback?: () => T | Promise<T>,
): Promise<T> {
  if (isTauri()) {
    return invoke<T>(command, args);
  }
  if (fallback) {
    return fallback();
  }
  throw new Error(`Tauri not available and no fallback for command: ${command}`);
}
```

Create `client/hooks/useTauri.ts` — React Query hooks for each IPC command:

```typescript
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { tauriInvoke } from '../lib/tauri';
// Import mock data from existing mock-data.ts
// Import types — define or import from a shared types file

// Example pattern for each hook:
export function useSpaces() {
  return useQuery({
    queryKey: ['spaces'],
    queryFn: () => tauriInvoke<Space[]>('get_spaces', {}, () => mockSpaces),
  });
}

export function useDocumentSearch(query: string, filters: SearchFilters) {
  return useQuery({
    queryKey: ['search', query, filters],
    queryFn: () => tauriInvoke<SearchResult[]>('search_documents', { query, filters }, () => []),
    enabled: query.length > 0,
  });
}

export function useStats() {
  return useQuery({
    queryKey: ['stats'],
    queryFn: () => tauriInvoke<Stats>('get_stats', {}, () => mockStats),
  });
}

export function useSettings() {
  return useQuery({
    queryKey: ['settings'],
    queryFn: () => tauriInvoke<Settings>('get_settings', {}, () => defaultSettings),
  });
}

export function useUpdateSettings() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (settings: Settings) => tauriInvoke<void>('update_settings', { settings }, () => undefined),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['settings'] }),
  });
}

// Add hooks for all major data types:
// useDocument(id), useRelatedDocuments(id), useWatchedFolders(),
// useSpaceDocuments(spaceId), useSpaceGraph(), useSearchAnalytics()
// useTriggerScan(), useAddWatchedFolder(), useRemoveWatchedFolder(),
// useMoveDocumentToSpace(), useToggleFavorite()
```

Create hooks for ALL IPC commands. Each query hook uses `useQuery` with a fallback to mock data. Each mutation hook uses `useMutation` with appropriate query invalidation.

Import mock data from existing `client/lib/mock-data.ts`. If mock data doesn't exist for a type, create a minimal default (empty array, zeroed stats, etc.).

**Verify:**
- `pnpm dev` — app loads in browser, hooks return mock data
- `pnpm typecheck` — no TS errors in new hook files
- Open browser console — no errors about `@tauri-apps/api` (it should not be called in browser mode)
</detail>
</task>

## Verification

```bash
# 1. Frontend builds
pnpm build:client || pnpm build

# 2. Dev server starts
timeout 10 pnpm dev &
sleep 5
curl -s http://localhost:5173 | grep -q "<" && echo "PASS" || echo "FAIL"
kill %1

# 3. TypeScript compiles
pnpm typecheck

# 4. Hook files exist
test -f client/hooks/useTauri.ts && test -f client/lib/tauri.ts && echo "PASS" || echo "FAIL"

# 5. Tauri API installed
grep "@tauri-apps/api" package.json && echo "PASS" || echo "FAIL"

# 6. React 19 installed
grep '"react":' package.json | grep -q "19" && echo "PASS" || echo "FAIL"

# 7. TailwindCSS 4 installed
grep '"tailwindcss":' package.json | grep -q "4" && echo "PASS" || echo "FAIL"
```

## must_haves

- [ ] `client/lib/tauri.ts` exports `isTauri()` and `tauriInvoke()` functions
- [ ] `client/hooks/useTauri.ts` exports React Query hooks for all major IPC commands
- [ ] Hooks return mock data when `isTauri()` is false (browser mode)
- [ ] Hooks call `invoke()` from `@tauri-apps/api/core` when `isTauri()` is true
- [ ] `@tauri-apps/api@^2` is in `package.json` dependencies
- [ ] React 19 is installed (`react@^19`, `react-dom@^19`)
- [ ] TailwindCSS 4 is installed and configured (`pnpm list tailwindcss` shows `4.x`)
- [ ] `postcss.config.js` is removed or replaced by TW4 approach
- [ ] `client/global.css` uses `@import "tailwindcss"` and contains zero `@tailwind` directives
- [ ] `pnpm dev` starts and the app renders in browser with correct styling
- [ ] `pnpm typecheck` passes (no TypeScript errors)
