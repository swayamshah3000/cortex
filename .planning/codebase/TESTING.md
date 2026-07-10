# Testing Patterns

**Analysis Date:** 2026-02-27

## Test Framework

**Runner:**
- Vitest v3.2.4
- Config: Implicit (uses Vite's test configuration)
- No explicit `vitest.config.ts` or `vitest.config.js` file — inferred from `vite.config.ts`

**Assertion Library:**
- Vitest built-in assertions (from `vitest` package)
- Pattern: `import { describe, it, expect } from "vitest"`

**Run Commands:**
```bash
npm run test              # Run all tests once
npm run typecheck         # TypeScript type checking
npm run format.fix        # Code formatting with Prettier
```

Current test coverage: Minimal (only utility function tested)

## Test File Organization

**Location:**
- Co-located with source files (same directory)
- Pattern: `filename.spec.ts` adjacent to `filename.ts`

**Naming:**
- Test files: `*.spec.ts` extension
- Example: `utils.spec.ts` tests `utils.ts`

**Structure:**
```
client/
├── lib/
│   ├── utils.ts           # Implementation
│   └── utils.spec.ts      # Tests (co-located)
```

## Test Structure

**Suite Organization:**
```typescript
import { describe, it, expect } from "vitest";
import { cn } from "./utils";

describe("cn function", () => {
  it("should merge classes correctly", () => {
    expect(cn("text-red-500", "bg-blue-500")).toBe("text-red-500 bg-blue-500");
  });

  it("should handle conditional classes", () => {
    const isActive = true;
    expect(cn("base-class", isActive && "active-class")).toBe(
      "base-class active-class",
    );
  });

  // Additional test cases...
});
```

**Patterns:**
- Suite: `describe("feature name", () => { ... })`
- Test case: `it("should describe behavior", () => { ... })`
- Assertion: `expect(actual).toBe(expected)`
- No setup/teardown hooks observed in current tests

## Assertions

**Style:**
- Equality: `.toBe()` for exact matches
- String comparison: `.toBe()` for string equality
- Multiple assertions per test when testing a single behavior

**Example** (`/Users/gshah/work/apps/cortex/client/lib/utils.spec.ts`):
```typescript
it("should merge tailwind classes properly", () => {
  expect(cn("px-2 py-1", "px-4")).toBe("py-1 px-4");
});

it("should work with object notation", () => {
  expect(cn("base", { conditional: true, "not-included": false })).toBe(
    "base conditional",
  );
});
```

## Mocking

**Framework:** Vitest provides built-in mocking (vi namespace)

**Patterns:**
- Not actively used in current test suite
- No mocks observed in `utils.spec.ts`

**What to Mock:**
- External API calls (when needed)
- Browser APIs (when testing hooks like `useIsMobile`)
- React Router hooks like `useLocation()`

**What NOT to Mock:**
- Pure utility functions like `cn()`
- Data transformation functions
- Internal component logic (prefer integration testing)

## Fixtures and Factories

**Test Data:**
- Inline test cases with literal values
- No shared fixtures or factories detected

**Pattern:**
```typescript
it("should handle conditional classes", () => {
  const isActive = true;  // Inline test data
  expect(cn("base-class", isActive && "active-class")).toBe(
    "base-class active-class",
  );
});
```

**Location:**
- Test data defined within test blocks (`it()` functions)
- Could be refactored to shared fixtures in future tests

## Coverage

**Requirements:** No coverage enforcement detected

**View Coverage:**
```bash
# Coverage command not configured
# To add: npm run test -- --coverage
```

No coverage thresholds or enforcement rules configured. This should be added as test suite grows.

## Test Types

**Unit Tests:**
- **Scope:** Pure functions (utilities, helpers)
- **Approach:** Test single function behavior with various inputs
- **Example:** `utils.spec.ts` tests the `cn()` class merging utility
- **Pattern:** Direct function call with literal inputs, `expect()` assertion on output
- **Current coverage:** Utility functions only

**Integration Tests:**
- Not yet implemented
- Will be needed for component rendering (React + TailwindCSS)
- Candidates: layout components like `Sidebar`, `TopBar`, `AppShell`

**E2E Tests:**
- Not implemented
- Playwright or Cypress would be appropriate for full-page flows
- Needed for: routing, user interactions (sidebar collapse, theme toggle)

## Common Patterns

**Async Testing:**
Not currently needed (no async utilities tested)

When needed, pattern will be:
```typescript
it("should handle async operations", async () => {
  const result = await someAsyncFunction();
  expect(result).toBe(expected);
});
```

**Error Testing:**
Not currently implemented

Pattern when adding error handling tests:
```typescript
it("should throw on invalid input", () => {
  expect(() => {
    functionThatThrows(invalidInput);
  }).toThrow();
});
```

## Testing Gaps

**Not Tested:**
- All React components (Sidebar, TopBar, Dashboard, etc.)
- React hooks (useIsMobile, useToast)
- React Router integration (page routing, navigation)
- API routes (server-side handlers)
- Theme switching (next-themes integration)

**Why:**
- Framework and dependencies are in place but tests not yet written
- Frontend development still in early stages (placeholder pages)
- Backend routes minimal (demo endpoint only)

**Priority for Testing:**
1. Utility functions (in progress)
2. Core layout components (Sidebar, TopBar, AppShell)
3. React hooks for mobile responsiveness
4. API integration with React Query
5. End-to-end user flows (routing, theme toggle)

## Test Execution

**During Development:**
```bash
npm run test              # Run once
```

**Recommended for CI/CD:**
```bash
npm run test -- --watch  # Watch mode for development
npm run test -- --coverage  # Generate coverage report
npm run typecheck         # Verify types before testing
```

**Git Hook Integration:**
- No pre-commit hooks configured for testing
- Should be added: test must pass before commit

## TypeScript in Tests

**Type Safety:**
- Tests are fully typed via TypeScript
- No `any` types used
- Function signatures provide implicit test contracts

**Example:**
```typescript
// utils.ts
export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

// utils.spec.ts imports with full type inference
import { cn } from "./utils";
```

---

*Testing analysis: 2026-02-27*
