/**
 * Tests for RelatedEntityChip component (Plan 06-06 Task 1, Test 5)
 *
 * Test 5: RelatedEntityChip composes EntityChip with a co-occurrence count badge
 * showing "× {n}" with tooltip text.
 */

import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import React from "react";
import { MemoryRouter } from "react-router-dom";
import { RelatedEntityChip } from "./RelatedEntityChip";
import type { RelatedEntity } from "@/lib/types";

const mockRelated: RelatedEntity = {
  entity: {
    id: "entity-acme-1",
    canonicalName: "Acme Corp",
    entityType: "organization",
    documentCount: 8,
  },
  coOccurrenceCount: 5,
};

function renderChip(related: RelatedEntity) {
  return render(
    <MemoryRouter>
      <RelatedEntityChip related={related} />
    </MemoryRouter>,
  );
}

describe("RelatedEntityChip (06-06 Task 1 - Test 5)", () => {
  it("renders the entity canonical name", () => {
    renderChip(mockRelated);
    expect(screen.getByText("Acme Corp")).toBeDefined();
  });

  it("renders the co-occurrence count badge with × symbol", () => {
    renderChip(mockRelated);
    expect(screen.getByText(/×\s*5/)).toBeDefined();
  });

  it("renders as an accessible button (Phase 11 dual-nav refactor)", () => {
    renderChip(mockRelated);
    // Phase 11 refactored EntityChip from <Link> to <button> w/ useNavigate.
    // RelatedEntityChip composes EntityChip so it now renders a button.
    const button = screen.getByRole("button");
    expect(button.getAttribute("aria-label")).toContain("Acme Corp");
  });

  it("count badge has tabular-nums and text-text-tertiary classes", () => {
    const { container } = renderChip(mockRelated);
    const badge = container.querySelector(".tabular-nums");
    expect(badge).not.toBeNull();
    expect(badge?.className).toContain("text-text-tertiary");
  });

  it("renders co-occurrence count with text-[10px] class", () => {
    const { container } = renderChip(mockRelated);
    // The count badge should have tiny font size
    const badge = container.querySelector(".text-\\[10px\\]");
    expect(badge).not.toBeNull();
  });

  it("renders with different co-occurrence counts correctly", () => {
    renderChip({
      entity: {
        id: "entity-xyz",
        canonicalName: "Brooklyn",
        entityType: "location",
        documentCount: 3,
      },
      coOccurrenceCount: 12,
    });
    expect(screen.getByText(/×\s*12/)).toBeDefined();
  });
});
