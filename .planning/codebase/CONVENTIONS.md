# Coding Conventions

**Analysis Date:** 2026-02-27

## Naming Patterns

**Files:**
- Components: `PascalCase.tsx` — `Button.tsx`, `Sidebar.tsx`, `AppShell.tsx`
- Pages: `PascalCase.tsx` with default export — `Index.tsx`, `NotFound.tsx`, `Placeholder.tsx`
- Utilities: `camelCase.ts` — `utils.ts`, `api.ts`
- Hooks: `useFeatureName.tsx` — `use-toast.ts`, `use-mobile.tsx`
- Directories: `kebab-case` for multi-word — `components/`, `client/`, `server/`

**Functions:**
- Components: `PascalCase` — `function Dashboard()`, `export function Sidebar()`
- Utilities: `camelCase` — `cn()`, `handleDemo()`
- Event handlers: `handleEventName` — `handleDemo: RequestHandler`, `onClick={() => setIsCollapsed(!isCollapsed)}`
- Constants in objects: `camelCase` — `mainLinks`, `routeNames`, `topSpaces`

**Variables:**
- State: `camelCase` — `isCollapsed`, `isMobile`, `isActive`, `theme`
- Props: `camelCase` — `path`, `label`, `icon`, `color`
- Data arrays: `camelCase` plural — `mainLinks`, `bottomLinks`, `stats`, `recentDocuments`

**Types:**
- Interfaces: `PascalCase` — `ButtonProps`, `DemoResponse`, `RequestHandler`
- Union/generic types: `PascalCase` — `ClassValue`, `VariantProps`
- Type imports: `import type { X }` — `import type { ButtonProps }`

## Code Style

**Formatting:**
- Tool: Prettier v3.6.2
- Tab width: 2 spaces
- No tabs, spaces only
- Trailing commas: all (enabled in `.prettierrc`)
- Config: `.prettierrc` with minimal customization

**Run formatting:**
```bash
npm run format.fix  # Applies Prettier to entire codebase
```

**Linting:**
- No ESLint config detected — style compliance relies on Prettier
- TypeScript strict mode: disabled (`"strict": false`)
- Unused variable/parameter detection: disabled (`noUnusedLocals: false`, `noUnusedParameters: false`)

## Import Organization

**Order:**
1. External packages (React, routing libraries, UI libraries)
2. Relative imports from internal modules (@/ aliases)
3. Type imports (when applicable)

**Pattern:**
```typescript
// External packages
import { useState } from "react";
import { Link, useLocation } from "react-router-dom";
import { Home, Brain, Search } from "lucide-react";

// Internal components/utilities
import { cn } from "@/lib/utils";
import { Sidebar } from "./Sidebar";

// Type imports
import type { ButtonProps } from "@/components/ui/button";
```

**Path Aliases:**
- `@/*` → `./client/*` — client-side imports
- `@shared/*` → `./shared/*` — shared types and utilities

Used throughout: `import { cn } from "@/lib/utils"`, `import type { DemoResponse } from "@shared/api"`

## Error Handling

**Patterns:**
- No centralized error handling detected in frontend code
- Console errors/logging: not observed in production code
- Try-catch: not prevalent
- Error boundaries: not implemented
- Backend (Express): type-safe responses with `RequestHandler<T>` and direct `.json()` responses

**Backend pattern** (`/Users/gshah/work/apps/cortex/server/routes/demo.ts`):
```typescript
export const handleDemo: RequestHandler = (req, res) => {
  const response: DemoResponse = {
    message: "Hello from Express server",
  };
  res.status(200).json(response);
};
```

## Logging

**Framework:** Console (default)

**Patterns:**
- No structured logging detected in source code
- Console logging not observable in application code
- Suitable for development environment

## Comments

**When to Comment:**
- JSDoc for utility functions: `cn()` function is self-documenting
- Inline comments for non-obvious logic: seen in layout comments like `{/* Sidebar - fixed positioned */}`
- Section comments for major sections within components

**JSDoc/TSDoc:**
- Minimal usage observed
- API interfaces documented via TypeScript types: `export interface DemoResponse { message: string; }`
- shadcn/ui components use internal TypeDoc

## Function Design

**Size:**
- Utility functions: single-purpose, 5-10 lines typical
  - `cn()`: 5 lines
  - `useIsMobile()`: 12 lines with React setup
- Components: 30-100+ lines typical
  - `Sidebar()`: 132 lines (complex layout)
  - `Dashboard()`: 236 lines (multi-section page)
  - `TopBar()`: 36 lines (simple layout)

**Parameters:**
- Destructured props: `({ className, variant, size, asChild = false, ...props }, ref) => `
- Optional parameters with defaults: `asChild = false`
- Type-safe via TypeScript interfaces

**Return Values:**
- Components: JSX.Element
- Utilities: consistent return types (string, boolean, etc.)
- React hooks: explicit type annotations when needed

## Module Design

**Exports:**
- **Named exports** for components: `export function Sidebar() { }`
- **Default exports** for route pages: `export default function Dashboard() { }`
- **Named + default exports** for utility components:
  ```typescript
  const Button = React.forwardRef<HTMLButtonElement, ButtonProps>((props, ref) => ...)
  export { Button, buttonVariants };
  ```

**Barrel Files:**
- Not used — imports are direct to specific files
- Example: `import { Sidebar } from "./Sidebar"` not `import { Sidebar } from "./index"`

## Component Patterns

**Styling:**
- Tailwind CSS utility classes directly in JSX
- `cn()` utility for conditional class merging (`import { cn } from "@/lib/utils"`)
- Component-level CSS classes via `@apply` in `global.css` (`.btn-primary`, `.card`, `.input-base`)

**Example** (`/Users/gshah/work/apps/cortex/client/components/layout/Sidebar.tsx`):
```typescript
className={cn(
  "group relative flex items-center gap-3 rounded-md px-3 py-2.5 text-sm font-medium transition-all duration-150",
  isActive(path)
    ? "bg-accent-primary text-white"
    : "text-text-secondary hover:bg-bg-tertiary hover:text-text-primary"
)}
```

**State Management:**
- React hooks for local component state: `useState()`
- React Router for page routing: `useLocation()`, `useTheme()`
- React Query (`@tanstack/react-query`) for server state (configured but not actively used in demo)
- Zustand would be used for global UI state (imported in App.tsx but not yet implemented)

**Props Pattern:**
- Type-safe props with TypeScript interfaces
- Destructuring in function signature
- Default values for optional props

**Event Handlers:**
- Arrow functions in JSX: `onClick={() => setIsCollapsed(!isCollapsed)}`
- Type-safe handlers: `const isActive = (path: string) => { return ... }`
- RequestHandler type for backend routes: `export const handleDemo: RequestHandler = (req, res) => { }`

## Data Structures

**API Types** (`/Users/gshah/work/apps/cortex/shared/api.ts`):
```typescript
export interface DemoResponse {
  message: string;
}
```

**Component-level Data:**
- Mock data inline in components as objects/arrays
- Example (`Index.tsx`):
```typescript
const stats = [
  {
    label: "Total Documents",
    value: "3.9K",
    icon: FileText,
    color: "bg-blue-500/10 text-blue-500",
  },
  // ...
];
```

**Validation:**
- Zod is included in dependencies but not actively used
- Type-safe via TypeScript interfaces for API contracts

## Design System Integration

**Color Tokens:**
- CSS custom properties defined in `global.css`:
  - `--bg-primary`, `--bg-secondary`, `--bg-tertiary`
  - `--text-primary`, `--text-secondary`, `--text-tertiary`
  - `--accent-primary`, `--accent-hover`, `--accent-subtle`
  - `--success`, `--warning`, `--error`, `--info`
  - `--border-primary`, `--border-secondary`

**Typography:**
- Fonts: Plus Jakarta Sans (display), Inter (body), JetBrains Mono (code)
- Classes: `.app-title`, `.page-title`, `.section-header`, `.card-title`, `.caption`, `.badge-text`
- All defined via `@apply` in `global.css` with consistent semantic naming

**Spacing:**
- Tailwind's default 4px base grid: `1`, `2`, `3`, `4`, `6`, `8`, `12` etc.
- Consistent gap/padding usage: `gap-2`, `gap-3`, `px-3`, `py-4`, `p-6`

**Button Variants:**
- Uses `class-variance-authority` (CVA) for variant management
- Defined in `button.tsx`:
```typescript
const buttonVariants = cva(
  // base styles
  {
    variants: {
      variant: { default, destructive, outline, secondary, ghost, link },
      size: { default, sm, lg, icon },
    },
  }
);
```

---

*Convention analysis: 2026-02-27*
