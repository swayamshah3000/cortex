# Technology Stack

**Analysis Date:** 2026-02-27

## Languages

**Primary:**
- TypeScript 5.9.2 - All frontend and server code (React components, Express handlers, shared types)
- JavaScript (ES2020+) - Build output, configuration files

**Configuration:**
- TypeScript compiled to ES2020 target
- JSX enabled with React 18 preset
- Path aliases: `@/*` → `client/`, `@shared/*` → `shared/`

## Runtime

**Environment:**
- Node.js 22 (production target in `vite.config.server.ts`)
- Browser: Modern ES2020+ support (dark mode class preference in Tailwind)

**Package Manager:**
- pnpm 10.14.0 (required, enforced via `packageManager` field)
- Lockfile: `pnpm-lock.yaml` (present, 202KB)
- Legacy peer deps allowed (set in `.npmrc`)

## Frameworks

**Core Frontend:**
- React 18.3.1 - Component library and rendering
- React Router 6.30.1 - Client-side SPA routing (6 routes defined in `client/App.tsx`)
- React Router v7 (listed in CLAUDE.md as planned version)

**UI Framework:**
- Radix UI (15 components) - Accessible, headless component library
  - Includes: Dialog, Accordion, Tabs, Select, Popover, Toast, Switch, Slider, Progress, etc.
  - Located: `@radix-ui/react-*` packages

**Styling:**
- TailwindCSS 3.4.17 - Utility-first CSS framework with custom design tokens
  - Config: `tailwind.config.ts`
  - Design system: Dark mode default, custom color palette (bg-primary, accent-primary, text-tertiary, etc.)
  - Animations: TailwindCSS Animate plugin for transitions
  - Extends: Custom fonts (Inter, Plus Jakarta Sans, JetBrains Mono)

**Server/Backend:**
- Express 5.1.0 - HTTP server framework
  - Dev integration: Express app mounted as middleware on Vite dev server
  - Production: Built to `dist/server/` and started via `node dist/server/node-build.mjs`
  - Routes: `/api/ping`, `/api/demo` (example endpoints in `server/routes/`)

**State Management:**
- React Query 5.84.2 - Server state + data fetching (TanStack)
- next-themes 0.4.6 - Dark mode theme persistence and toggling
- Zustand (not found in current package.json, listed in CLAUDE.md as planned)

**Build/Dev:**
- Vite 7.1.2 - Fast build tool and dev server
  - Config: `vite.config.ts` (client SPA), `vite.config.server.ts` (Express build)
  - Plugin: `@vitejs/plugin-react-swc` 4.0.0 - SWC-based React compilation
  - Dev server: Port 8080, unified frontend/backend
  - Build output: `dist/spa/` (client), `dist/server/` (server)

**Testing:**
- Vitest 3.2.4 - Unit/component testing framework (Vite-native)
  - Config: Built into Vite, no separate vitest.config file found
  - Run with: `npm test` → `vitest --run`

**CSS Processing:**
- PostCSS 8.5.6 - CSS transformation pipeline
  - Config: `postcss.config.js`
  - Plugins: Tailwind, autoprefixer
- Autoprefixer 10.4.21 - Cross-browser CSS prefixing

**Form & Validation:**
- React Hook Form 7.62.0 - Lightweight form state management
- Zod 3.25.76 - Schema validation (runtime type checking)
- Resolvers 5.2.1 - Zod/validation integration with Hook Form

## Key Dependencies

**Critical:**
- framer-motion 12.23.12 - Animation and motion library
- lucide-react 0.539.0 - Icon set (1.5px stroke weight)
- recharts 2.12.7 - React charting library for analytics visualizations
- three 0.176.0 - 3D graphics library
- @react-three/fiber 8.18.0 - React renderer for Three.js
- @react-three/drei 9.122.0 - Three.js utilities (3D models, effects)

**UI & UX:**
- cmdk 1.1.1 - Command palette/search component
- embla-carousel-react 8.6.0 - Carousel/slider functionality
- react-resizable-panels 3.0.4 - Resizable panel layouts
- react-day-picker 9.8.1 - Date picker UI component
- sonner 1.7.4 - Toast notifications
- vaul 1.1.2 - Drawer component primitive
- class-variance-authority 0.7.1 - Component variant system
- clsx 2.1.1 - Conditional className utility
- tailwind-merge 2.6.0 - Merges Tailwind classes intelligently
- input-otp 1.4.2 - OTP input component

**Date/Time:**
- date-fns 4.1.0 - Date manipulation and formatting

**Build/Dev Tools:**
- @swc/core 1.13.3 - SWC compiler (powers React plugin)
- tsx 4.20.3 - TypeScript Node runner
- prettier 3.6.2 - Code formatter
- typescript 5.9.2 - TypeScript compiler

**Utilities:**
- cors 2.8.5 - CORS middleware for Express
- dotenv 17.2.1 - Environment variable loading
- serverless-http 3.2.0 - Netlify Functions adapter
- globals 16.3.0 - Global object polyfills

**Type Definitions:**
- @types/express 5.0.3
- @types/react 18.3.23
- @types/react-dom 18.3.7
- @types/node 24.2.1
- @types/cors 2.8.19
- @types/three 0.176.0

## Configuration

**Environment:**
- Environment variables loaded via `dotenv` in server (`server/index.ts`)
- `.env` file present (312 bytes) - contains configuration
- Required env vars: `PING_MESSAGE`, `PORT` (defaults shown in code)
- See `.env` file for current values (not readable via tool)

**Build Configuration:**
- TypeScript config: `tsconfig.json`
  - Target: ES2020
  - Strict mode: OFF (easier development, but less type safety)
  - No unused variable checks enabled
- Prettier config: `.prettierrc`
  - 2-space indentation
  - Trailing commas: all
  - No semicolons
- VSCode integration: `components.json` for shadcn/ui scaffolding

## Platform Requirements

**Development:**
- Node.js 20+ (pnpm requires Node 18+)
- macOS/Linux/Windows (cross-platform via Vite + Node)
- Editor: TypeScript-aware (VS Code recommended)

**Production:**
- Node.js 22 (specified in `vite.config.server.ts`)
- Build artifact: Single JavaScript bundle + source maps
- Deployment options: Netlify (via `netlify.toml`), Vercel, self-hosted Node

## Deployment & Hosting

**Currently Configured:**
- Netlify deployment via `netlify.toml`
  - Build command: `npm run build:client` (only frontend)
  - Functions: Express server compiled to Netlify Functions
  - Node bundler: esbuild
  - Publish directory: `dist/spa/`
  - API rewrites: `/api/*` → `/.netlify/functions/api/:splat`
- External modules for functions: `express` (not bundled)

**Scalability:**
- Single-port Vite dev server (port 8080) during development
- Production: Separate client SPA and Node server builds
- Can be containerized or deployed to serverless platforms

---

*Stack analysis: 2026-02-27*
