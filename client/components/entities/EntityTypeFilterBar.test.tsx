/**
 * Tests for EntityTypeFilterBar component (Plan 06-06 Task 1, Test 4)
 *
 * Test 4: Renders 7 pills (All + 6 types), clicking calls onSelect with correct value.
 * Active pill renders with accent-primary classes; inactive uses bg-bg-secondary + border-border-primary.
 */

import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import React from "react";
import { EntityTypeFilterBar } from "./EntityTypeFilterBar";

describe("EntityTypeFilterBar (06-06 Task 1 - Test 4)", () => {
  it("renders 7 pills: All + 6 entity types", () => {
    render(<EntityTypeFilterBar active="all" onSelect={vi.fn()} />);
    const buttons = screen.getAllByRole("button");
    // Should have All, person, organization, location, date, amount, email
    expect(buttons.length).toBe(7);
  });

  it("renders expected pill labels", () => {
    render(<EntityTypeFilterBar active="all" onSelect={vi.fn()} />);
    expect(screen.getByRole("button", { name: /^all$/i })).toBeDefined();
    expect(screen.getByRole("button", { name: /^person$/i })).toBeDefined();
    expect(screen.getByRole("button", { name: /^organization$/i })).toBeDefined();
    expect(screen.getByRole("button", { name: /^location$/i })).toBeDefined();
    expect(screen.getByRole("button", { name: /^date$/i })).toBeDefined();
    expect(screen.getByRole("button", { name: /^amount$/i })).toBeDefined();
    expect(screen.getByRole("button", { name: /^email$/i })).toBeDefined();
  });

  it("clicking 'person' pill calls onSelect('person')", () => {
    const onSelect = vi.fn();
    render(<EntityTypeFilterBar active="all" onSelect={onSelect} />);
    fireEvent.click(screen.getByRole("button", { name: /^person$/i }));
    expect(onSelect).toHaveBeenCalledWith("person");
  });

  it("clicking 'All' pill calls onSelect('all')", () => {
    const onSelect = vi.fn();
    render(<EntityTypeFilterBar active="person" onSelect={onSelect} />);
    fireEvent.click(screen.getByRole("button", { name: /^all$/i }));
    expect(onSelect).toHaveBeenCalledWith("all");
  });

  it("active pill has accent-primary classes", () => {
    render(<EntityTypeFilterBar active="person" onSelect={vi.fn()} />);
    const personBtn = screen.getByRole("button", { name: /^person$/i });
    expect(personBtn.className).toContain("bg-accent-primary");
  });

  it("inactive pills have bg-bg-secondary and border-border-primary classes", () => {
    render(<EntityTypeFilterBar active="all" onSelect={vi.fn()} />);
    const personBtn = screen.getByRole("button", { name: /^person$/i });
    expect(personBtn.className).toContain("bg-bg-secondary");
    expect(personBtn.className).toContain("border-border-primary");
  });

  it("clicking 'organization' pill calls onSelect('organization')", () => {
    const onSelect = vi.fn();
    render(<EntityTypeFilterBar active="all" onSelect={onSelect} />);
    fireEvent.click(screen.getByRole("button", { name: /^organization$/i }));
    expect(onSelect).toHaveBeenCalledWith("organization");
  });
});
