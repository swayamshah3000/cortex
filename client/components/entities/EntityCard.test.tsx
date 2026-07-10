/**
 * Tests for EntityCard component (Plan 06-06 Task 1, Test 3)
 *
 * Test 3: EntityCard wraps a Link to /entities/{id} with type-color icon tile,
 * canonical_name, document_count caption, and "X aliases" caption when aliases.length > 1.
 */

import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import React from "react";
import { MemoryRouter } from "react-router-dom";
import { EntityCard } from "./EntityCard";
import type { EntitySummary } from "@/lib/types";

const baseEntity: EntitySummary & { aliases?: string[] } = {
  id: "entity-john-smith",
  canonicalName: "John Smith",
  entityType: "person",
  documentCount: 12,
};

function renderCard(entity: EntitySummary & { aliases?: string[] }) {
  return render(
    <MemoryRouter>
      <EntityCard entity={entity} />
    </MemoryRouter>,
  );
}

describe("EntityCard (06-06 Task 1 - Test 3)", () => {
  it("renders a link to /entities/{id}", () => {
    renderCard(baseEntity);
    const link = screen.getByRole("link");
    expect(link.getAttribute("href")).toBe("/entities/entity-john-smith");
  });

  it("renders canonical name", () => {
    renderCard(baseEntity);
    expect(screen.getByText("John Smith")).toBeDefined();
  });

  it("renders document count caption", () => {
    renderCard(baseEntity);
    expect(screen.getByText(/12 doc/i)).toBeDefined();
  });

  it("renders 'X aliases' caption when aliases.length > 1", () => {
    renderCard({
      ...baseEntity,
      aliases: ["J. Smith", "Smith, John", "John Smith"],
    });
    expect(screen.getByText(/3 aliases/i)).toBeDefined();
  });

  it("does NOT render aliases caption when aliases is undefined", () => {
    renderCard(baseEntity);
    expect(screen.queryByText(/aliases/i)).toBeNull();
  });

  it("does NOT render aliases caption when only 1 alias", () => {
    renderCard({
      ...baseEntity,
      aliases: ["John Smith"],
    });
    expect(screen.queryByText(/1 alias/i)).toBeNull();
  });

  it("applies card and hover classes", () => {
    renderCard(baseEntity);
    const link = screen.getByRole("link");
    expect(link.className).toContain("card");
    expect(link.className).toContain("hover:shadow-md");
  });

  it("applies type-color icon tile for person type (purple)", () => {
    const { container } = renderCard(baseEntity);
    const iconTile = container.querySelector(".bg-purple-400\\/10");
    expect(iconTile).not.toBeNull();
  });
});
