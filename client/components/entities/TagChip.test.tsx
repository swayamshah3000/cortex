/**
 * Tests for TagChip component (Plan 08-08 Task 1 RED)
 *
 * Behavior:
 * - Renders with neutral bg-bg-tertiary background + Hash icon + rounded-md rectangular pill
 * - Transforms snake_case to space-separated (no capitalization)
 * - max-w-[120px] truncate for overflow protection
 * - Returns null for empty tag
 */

import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import React from "react";
import { TagChip } from "./TagChip";

describe("TagChip (08-08 Task 1)", () => {
  it("renders with rounded-md rectangular pill shape", () => {
    const { container } = render(<TagChip tag="bank_statement" />);
    const chip = container.firstChild as HTMLElement;
    expect(chip?.className).toContain("rounded-md");
  });

  it("renders with neutral bg-bg-tertiary background", () => {
    const { container } = render(<TagChip tag="bank_statement" />);
    const chip = container.firstChild as HTMLElement;
    expect(chip?.className).toContain("bg-bg-tertiary");
  });

  it("renders a Hash icon (SVG element present)", () => {
    const { container } = render(<TagChip tag="bank_statement" />);
    const svg = container.querySelector("svg");
    expect(svg).not.toBeNull();
  });

  it("transforms snake_case to space-separated with no capitalization: khush_school → 'khush school'", () => {
    render(<TagChip tag="khush_school" />);
    expect(screen.getByText("khush school")).toBeDefined();
  });

  it("does NOT capitalize the first letter (unlike TopicChip)", () => {
    render(<TagChip tag="property_tax" />);
    expect(screen.getByText("property tax")).toBeDefined();
  });

  it("preserves single-word tags unchanged", () => {
    render(<TagChip tag="finance" />);
    expect(screen.getByText("finance")).toBeDefined();
  });

  it("has truncate class for overflow protection", () => {
    const { container } = render(<TagChip tag="some_very_long_tag" />);
    expect(container.querySelector(".truncate")).not.toBeNull();
  });

  it("returns null for empty string tag", () => {
    const { container } = render(<TagChip tag="" />);
    expect(container.firstChild).toBeNull();
  });

  it("has text-text-secondary color class", () => {
    const { container } = render(<TagChip tag="finance" />);
    const chip = container.firstChild as HTMLElement;
    expect(chip?.className).toContain("text-text-secondary");
  });
});
