# Codebase Structure

**Analysis Date:** 2026-02-27

## Directory Layout

```
cortex/
├── client/                    # React frontend application
│   ├── components/           # React components
│   │   ├── layout/          # Layout-level components (AppShell, Sidebar, TopBar)
│   │   └── ui/              # shadcn/ui + Radix UI components (40+ primitives)
│   ├── pages/               # Page-level components (route endpoints)
│   ├── hooks/               # Custom React hooks (mobile detection, toast)
│   ├── lib/                 # Utilities (cn, classname merging)
│   ├── App.tsx              # Root component with routing
│   ├── global.css           # Design system tokens and base styles
│   └── vite-env.d.ts        # Vite environment types
├── server/                  # Express backend application
│   ├── routes/              # API route handlers
│   ├── index.ts             # Express app factory (createServer)
│   └── node-build.ts        # Production entry point with static serving
├── shared/                  # Shared code between frontend and backend
│   └── api.ts               # API type definitions (DemoResponse)
├── .builder/                # Builder/build-related artifacts
├── .planning/               # Planning and analysis documents
├── public/                  # Static assets served by Express
├── node_modules/            # Dependencies (pnpm)
├── index.html               # HTML entry point for SPA
├── package.json             # Project metadata and dependencies
├── pnpm-lock.yaml           # Lockfile for reproducible installs
├── tsconfig.json            # TypeScript configuration
├── vite.config.ts           # Vite build config (development frontend)
├── vite.config.server.ts    # Vite build config (production backend)
├── tailwind.config.ts       # TailwindCSS theme and content
├── postcss.config.js        # PostCSS (for TailwindCSS processing)
├── components.json          # shadcn/ui configuration
├── .prettierrc               # Code formatter config
├── .npmrc                    # npm registry config
└── CLAUDE.md                # Project-specific instructions
```

## Directory Purposes

**client/**
- Purpose: All frontend React code, styles, and component logic
- Contains: Page components, layout components, UI primitives, hooks, utilities
- Key files: `App.tsx` (root), `global.css` (design tokens)

**client/components/**
- Purpose: Reusable React components organized by type
- Contains: Layout wrappers, UI primitives, page-level layouts
- Key subdirs: `layout/` (AppShell, Sidebar, TopBar), `ui/` (40+ shadcn/ui components)

**client/components/layout/**
- Purpose: Core layout components used by all pages
- Key files:
  - `AppShell.tsx` - Root layout with sidebar + topbar + outlet
  - `Sidebar.tsx` - Fixed navigation panel with collapsible state
  - `TopBar.tsx` - Sticky header with search and theme toggle

**client/components/ui/**
- Purpose: Pre-built UI primitives from shadcn/ui based on Radix UI
- Contains: 40+ components (button, dialog, input, select, accordion, etc.)
- Pattern: Exported as named exports for tree-shaking

**client/pages/**
- Purpose: Route-level page components (each file corresponds to a route)
- Key files:
  - `Index.tsx` - Dashboard page (12 route)
  - `Placeholder.tsx` - Stub for unimplemented routes (11 other routes)
  - `NotFound.tsx` - 404 fallback page
- Pattern: Default exports for React Router lazy loading

**client/hooks/**
- Purpose: Custom React hooks for shared logic
- Key files:
  - `use-mobile.tsx` - Media query hook for responsive design
  - `use-toast.ts` - Toast notification management (Sonner integration)

**client/lib/**
- Purpose: Utility functions and helpers
- Key files:
  - `utils.ts` - `cn()` function (clsx + tailwind-merge) for conditional classnames
  - `utils.spec.ts` - Vitest tests for `cn()` function
  - `use-toast.ts` - Legacy toast hook (superseded by Sonner)

**server/**
- Purpose: Express backend application
- Key files:
  - `index.ts` - Server factory function (createServer)
  - `node-build.ts` - Production entrypoint with static serving
  - `routes/demo.ts` - Example API route handler

**server/routes/**
- Purpose: API endpoint implementations
- Pattern: One file per logical grouping (routes/demo.ts, routes/auth.ts, etc.)
- Current routes:
  - `GET /api/ping` - Health check
  - `GET /api/demo` - Example with typed response

**shared/**
- Purpose: Code shared between frontend and backend
- Contains: TypeScript interfaces, shared utilities, constants
- Key files:
  - `api.ts` - API response types (DemoResponse)

**vite configs:**
- `vite.config.ts` - Frontend build and dev server config
- `vite.config.server.ts` - Backend production build config

## Key File Locations

**Entry Points:**
- `index.html` - Browser entry point (script src="/client/App.tsx")
- `client/App.tsx` - React app root with routing and providers
- `server/index.ts` - Express app factory
- `server/node-build.ts` - Production Express server startup

**Configuration:**
- `tsconfig.json` - TypeScript compiler options and path aliases
- `vite.config.ts` - Dev server and frontend build config
- `vite.config.server.ts` - Backend production build config
- `tailwind.config.ts` - Design system theme extension
- `postcss.config.js` - CSS processing pipeline
- `components.json` - shadcn/ui CLI configuration

**Core Logic:**
- `client/App.tsx` - Route definitions and provider setup
- `client/components/layout/AppShell.tsx` - Root layout composition
- `client/components/layout/Sidebar.tsx` - Navigation state and rendering
- `client/pages/Index.tsx` - Dashboard page with mock data

**Styling:**
- `client/global.css` - Design tokens (colors, spacing, shadows), component utilities
- `tailwind.config.ts` - Custom color tokens mapped to CSS variables
- Individual component files - Scoped TailwindCSS classes

**Testing:**
- `client/lib/utils.spec.ts` - Vitest tests for utility functions

## Naming Conventions

**Files:**
- Components: PascalCase (e.g., `AppShell.tsx`, `Sidebar.tsx`)
- Hooks: `use-` prefix, kebab-case (e.g., `use-mobile.tsx`, `use-toast.ts`)
- Pages: PascalCase (e.g., `Index.tsx`, `NotFound.tsx`)
- Utilities: lowercase with dots for domain (e.g., `utils.ts`, `api.ts`)
- Tests: `*.spec.ts` or `*.test.ts` suffix

**Directories:**
- Feature directories: lowercase (e.g., `components/`, `pages/`, `hooks/`)
- Groupings within components: lowercase (e.g., `layout/`, `ui/`)
- No feature-based subdirs yet (all code at top level)

**TypeScript Classes/Interfaces:**
- Components: PascalCase functional components (no classes)
- Types: PascalCase (e.g., `DemoResponse`)
- Enums: PascalCase (none currently defined)
- Constants: CONSTANT_CASE for module-level (none defined yet)

## Where to Add New Code

**New Feature (Page):**
- Implementation: `client/pages/FeatureName.tsx` - Default export component
- Tests: `client/pages/FeatureName.spec.ts` - Vitest tests
- Styling: Inline TailwindCSS classes, leverage design tokens from `global.css`
- Routing: Add route in `client/App.tsx` with path and element

**New Component (Reusable):**
- Implementation: `client/components/ComponentName.tsx` - Named export
- Tests: `client/components/ComponentName.spec.ts` if complex
- Pattern: Functional components with hooks, no PropTypes (use TypeScript interfaces)
- Styling: Inherit TailwindCSS + design tokens, use `cn()` for conditional classes

**New Layout Component:**
- Location: `client/components/layout/LayoutName.tsx` - Named export
- Pattern: Composition with Outlet for nested content
- Styling: Match AppShell, Sidebar, TopBar pattern (fixed positioning, responsive)

**New API Route:**
- Implementation: `server/routes/domain.ts` - Export RequestHandler function
- Type definition: Add interface to `shared/api.ts`
- Integration: Import and use in `server/index.ts` (app.get/post/etc.)

**New Utility:**
- Implementation: `client/lib/domain.ts` (if client-only) or `shared/domain.ts` (if shared)
- Tests: `client/lib/domain.spec.ts` - Vitest tests
- Pattern: Pure functions, no side effects

**New Hook:**
- Implementation: `client/hooks/use-domain.tsx` or `use-domain.ts` - Named export
- Pattern: React hooks returning state/callbacks or data
- Location: `hooks/` for custom hooks, `components/ui/use-*.ts` for component-specific

**New UI Component (shadcn/ui):**
- Add via: `npx shadcn-ui@latest add component-name` (if available)
- Customization: Edit in `client/components/ui/component-name.tsx`
- Import path: `@/components/ui/component-name`

## Special Directories

**.builder/**
- Purpose: Build-related artifacts and tooling (unclear current contents)
- Generated: Likely yes
- Committed: Unknown

**.planning/**
- Purpose: Project planning, documentation, and analysis (this file included)
- Generated: No (manually created)
- Committed: Yes (git tracked)

**dist/**
- Purpose: Build output directory
- Contents: `dist/spa/` (compiled frontend), `dist/server/` (compiled backend)
- Generated: Yes (created by build process)
- Committed: No (.gitignore)

**node_modules/**
- Purpose: Installed dependencies
- Generated: Yes (by pnpm)
- Committed: No (.gitignore)

## Module Resolution

**Path Aliases (tsconfig.json):**
- `@/*` → `./client/*` (client-side imports)
- `@shared/*` → `./shared/*` (shared code imports)

**Example imports:**
```typescript
// Component imports
import { AppShell } from "@/components/layout/AppShell";
import { Button } from "@/components/ui/button";

// Utility imports
import { cn } from "@/lib/utils";

// Hook imports
import { useToast } from "@/hooks/use-toast";

// Shared type imports
import type { DemoResponse } from "@shared/api";
```

**Why this matters:** Absolute imports prevent `../../../` chains, improve refactoring, and clarify code origin.

---

*Structure analysis: 2026-02-27*
