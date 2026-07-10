/**
 * Tests for SpaceCard component — 09-UI-SPEC.md §Interaction States matrix (4 rows).
 *
 * Covers:
 *   Test 1: labelStatus=generating → shimmer visible, lock absent, entity hint absent
 *   Test 2: userLocked=true → Lock icon with correct aria-label
 *   Test 3: canonicalEntityHint present → EntityHintChip renders hint text
 *   Test 4: description present → TooltipContent renders description (truncated at 100 chars)
 *
 * Note on tooltip mocking: Radix Tooltip does not open in JSDOM via pointer events because
 * JSDOM lacks a fully functional Pointer Events API. Instead, @/components/ui/tooltip is
 * mocked to always render TooltipContent (role="tooltip") unconditionally. This tests our
 * component's logic (correct description forwarding + truncation) in isolation from
 * Radix's hover behaviour — which is a third-party concern, not ours.
 *
 * Plan 09-06 Task 1.
 */

import { describe, it, expect, vi, afterEach } from "vitest";
import { render, screen } from "@testing-library/react";
import React from "react";
import { MemoryRouter } from "react-router-dom";
import { SpaceCard } from "./SpaceCard";
import type { Space } from "@/lib/types";

// --- Mocks -------------------------------------------------------------------

vi.mock("@/lib/icons", () => ({
  resolveIcon: () => {
    const MockIcon = (props: React.SVGProps<SVGSVGElement>) => (
      <svg {...props} data-testid="space-icon" />
    );
    MockIcon.displayName = "MockIcon";
    return MockIcon;
  },
}));

vi.mock("@/lib/format", () => ({
  formatRelativeTime: () => "2d ago",
}));

/**
 * Mock Radix Tooltip primitives to always show TooltipContent.
 * In JSDOM, pointer events don't trigger Radix's timer-based open logic.
 * Mocking here tests SpaceCard's contract: "pass description to TooltipContent when set".
 */
vi.mock("@/components/ui/tooltip", () => ({
  TooltipProvider: ({ children }: { children: React.ReactNode }) => <>{children}</>,
  Tooltip: ({ children }: { children: React.ReactNode }) => <>{children}</>,
  TooltipTrigger: ({
    children,
    asChild,
  }: {
    children: React.ReactNode;
    asChild?: boolean;
  }) => <>{children}</>,
  TooltipContent: ({
    children,
    className,
  }: {
    children: React.ReactNode;
    className?: string;
    side?: string;
    sideOffset?: number;
  }) => (
    <div role="tooltip" className={className}>
      {children}
    </div>
  ),
}));

// --- Helpers -----------------------------------------------------------------

const baseSpace: Space = {
  id: "space-1",
  name: "Property Tax Records",
  icon: "Home",
  color: "#6D28D9",
  documentCount: 42,
  lastUpdated: new Date().toISOString(),
  subSpaces: [],
  sampleFiles: ["tax_2023.pdf", "assessment_notice.pdf"],
};

function renderCard(space: Space) {
  return render(
    <MemoryRouter>
      <SpaceCard space={space} />
    </MemoryRouter>,
  );
}

afterEach(() => {
  vi.restoreAllMocks();
});

// --- Tests -------------------------------------------------------------------

describe("SpaceCard (09-06 Task 1)", () => {
  it("Test 1: labelStatus=generating → shimmer visible, 'Generating label…' shown, lock icon absent, entity hint chip absent", () => {
    const space: Space = {
      ...baseSpace,
      labelStatus: "generating",
      canonicalEntityHint: "Person: Alex Doe",
      userLocked: false,
    };
    renderCard(space);

    // Shimmer sub-line visible
    expect(screen.getByText("Generating label…")).toBeDefined();

    // Space name NOT rendered (replaced by skeleton)
    expect(screen.queryByText("Property Tax Records")).toBeNull();

    // Lock icon absent
    expect(screen.queryByLabelText("Label locked by user")).toBeNull();

    // Entity hint chip absent (canonicalEntityHint set but still generating)
    expect(screen.queryByText("Person: Alex Doe")).toBeNull();

    // TooltipContent not rendered (no description)
    expect(screen.queryByRole("tooltip")).toBeNull();
  });

  it("Test 2: userLocked=true → Lock icon visible with aria-label 'Label locked by user'", () => {
    const space: Space = {
      ...baseSpace,
      userLocked: true,
    };
    renderCard(space);

    const lockIcon = screen.getByLabelText("Label locked by user");
    expect(lockIcon).toBeDefined();
  });

  it("Test 3: canonicalEntityHint='Person: Alex Doe' + ready state → EntityHintChip renders hint text", () => {
    const space: Space = {
      ...baseSpace,
      canonicalEntityHint: "Person: Alex Doe",
    };
    renderCard(space);

    // EntityHintChip should render the full hint string
    expect(screen.getByText("Person: Alex Doe")).toBeDefined();
  });

  it("Test 4: description present → TooltipContent renders with description (≤ 100 chars)", () => {
    const description =
      "Documents related to municipal property tax assessments, receipts, and demand notices.";
    const space: Space = { ...baseSpace, description };
    renderCard(space);

    // With mocked tooltip, TooltipContent renders unconditionally when description is set
    const tooltip = screen.getByRole("tooltip");
    expect(tooltip).toBeDefined();
    expect(tooltip.textContent).toBe(description);
  });

  it("Test 4b: description > 100 chars → tooltip shows first 100 chars + ellipsis", () => {
    const longDesc = "A".repeat(101);
    const space: Space = { ...baseSpace, description: longDesc };
    renderCard(space);

    const tooltip = screen.getByRole("tooltip");
    expect(tooltip.textContent).toBe("A".repeat(100) + "…");
  });

  it("Test 4c: description absent → no TooltipContent rendered", () => {
    const space: Space = { ...baseSpace, description: undefined };
    renderCard(space);

    expect(screen.queryByRole("tooltip")).toBeNull();
  });
});
