# Architecture

**Analysis Date:** 2026-02-27

## Pattern Overview

**Overall:** Full-stack monorepo with separated frontend (React), backend (Express/Node), and shared types. Client-server architecture with Vite-based development and production builds.

**Key Characteristics:**
- Shared type definitions via `shared/` directory for type safety across client-server boundary
- Express server embedded in Vite dev server, separate production build
- React Router-based SPA with fallback to index.html for non-API routes
- TailwindCSS design system with custom color tokens via CSS variables
- shadcn/ui components adapted to Cortex design language

## Layers

**Frontend (React/Client):**
- Purpose: Interactive UI for document organization, search, and management
- Location: `client/`
- Contains: React components (layout, pages, UI), hooks, utilities, styles
- Depends on: React Router, React Query, TailwindCSS, Radix UI, Express server (dev only)
- Used by: Browser/end users

**API Server (Express):**
- Purpose: HTTP API endpoints and static file serving
- Location: `server/`
- Contains: Express app configuration, API route handlers
- Depends on: Express, CORS, shared types
- Used by: Frontend client (fetch requests)

**Shared Types:**
- Purpose: Single source of truth for types shared between frontend and backend
- Location: `shared/`
- Contains: TypeScript interfaces (e.g., `DemoResponse`)
- Depends on: Zod (runtime validation)
- Used by: Both client and server

## Data Flow

**Development Mode (Vite Dev Server):**

1. Browser requests http://localhost:8080/
2. Vite dev server serves `index.html` with React app
3. React app mounts to `#root` element
4. Express server is mounted as Vite middleware on `/api/` paths
5. Client fetch requests to `/api/*` hit Express middleware
6. Non-API requests to non-existent files serve `index.html` (React Router fallback)

**Production Mode (Built):**

1. Build output: `dist/spa/` (frontend) + `dist/server/` (backend)
2. Express server starts independently (port 3000 by default)
3. Express serves static SPA files from `dist/spa/`
4. All non-API, non-existent routes fallback to `index.html`
5. API routes (`/api/`) return 404 if not handled

**State Management:**

- UI state: Zustand (dark mode, sidebar collapsed state, command palette)
- Server state: React Query (cached API responses, background sync)
- Component state: Local React hooks where appropriate
- Persistence: localStorage for theme preference via next-themes

## Key Abstractions

**AppShell Component:**
- Purpose: Root layout wrapper providing sidebar + topbar + content area
- Location: `client/components/layout/AppShell.tsx`
- Pattern: Layout composition with Outlet for nested routes
- Usage: Wraps all authenticated routes in React Router

**Sidebar Component:**
- Purpose: Fixed navigation panel with collapsible state
- Location: `client/components/layout/Sidebar.tsx`
- Pattern: React hooks (useState) for collapse state, useLocation for active route detection
- Responsive: Fixed 240px expanded, 64px collapsed; hidden on mobile

**TopBar Component:**
- Purpose: Sticky header with search and theme toggle
- Location: `client/components/layout/TopBar.tsx`
- Pattern: Minimal interactive header using next-themes for dark mode
- Features: Search placeholder, theme switcher

**Design System Tokens:**
- Purpose: Centralized color, spacing, typography, shadow definitions
- Location: `client/global.css` (CSS variables) + `tailwind.config.ts` (Tailwind theme)
- Pattern: HSL-based color variables with light/dark mode overrides in `.dark` class
- Custom tokens: `bg-primary`, `text-primary`, `accent-primary`, `border-primary` (and secondary/tertiary variants)

**UI Component Library:**
- Purpose: Radix UI primitives + shadcn/ui implementations
- Location: `client/components/ui/`
- Pattern: Headless components wrapping Radix primitives with Cortex styling
- Count: 40+ pre-built components (button, dialog, form, select, etc.)

**Route Configuration:**
- Purpose: Define application navigation structure
- Location: `client/App.tsx`
- Pattern: React Router v7 with nested routes via AppShell
- Routes: 12 routes from dashboard to settings, all use Placeholder pages (awaiting implementation)

## Entry Points

**Frontend Entry Point:**
- Location: `index.html` + `client/App.tsx`
- Triggers: User navigates to application URL
- Responsibilities: Bootstrap React, configure providers (ThemeProvider, QueryClient, BrowserRouter), render root component

**Backend Entry Point (Development):**
- Location: `vite.config.ts` (expressPlugin)
- Triggers: `bun dev` command
- Responsibilities: Create Express server, mount to Vite middleware, enable CORS and JSON parsing

**Backend Entry Point (Production):**
- Location: `server/node-build.ts`
- Triggers: `npm start` command
- Responsibilities: Create Express app, serve static SPA, listen on PORT (default 3000)

**API Routes:**
- Location: `server/routes/`
- Currently defined:
  - `GET /api/ping` - Health check (returns configurable PING_MESSAGE)
  - `GET /api/demo` - Example route returning DemoResponse type

## Error Handling

**Strategy:** Graceful degradation with silent failures and fallbacks

**Patterns:**
- 404 API routes return JSON error response
- Non-API requests fallback to `index.html` (React Router handles missing pages)
- Database/external service failures: Not yet implemented (mock data used)
- Theme toggle: Fallback to system preference if localStorage unavailable
- Invalid routes: Dedicated NotFound page component

## Cross-Cutting Concerns

**Logging:**
- Server: console.log for startup info and graceful shutdown messages
- Client: No structured logging (can implement via React Query devtools or custom middleware)

**Validation:**
- Input validation: Zod schemas in `shared/` (defined for DemoResponse)
- Form validation: React Hook Form + Zod (components available, not yet integrated into pages)
- Type safety: TypeScript strict mode disabled (baseUrl + paths aliases configured)

**Authentication:**
- Status: Not implemented
- Planned: OAuth/JWT via Tauri when moving to desktop app variant
- Current: All routes public (mock data only)

**Environment Configuration:**
- Dev: `.env` file for PING_MESSAGE (Vite handles via import.meta.env)
- Production: Environment variables (PORT, PING_MESSAGE) read via `process.env`
- No secrets currently stored

---

*Architecture analysis: 2026-02-27*
