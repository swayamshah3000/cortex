/**
 * Tests for TopicChip component (Plan 08-08 Task 1 RED)
 *
 * Behavior:
 * - Renders with accent-tinted background + Bookmark icon + rounded-full pill shape
 * - Transforms snake_case topic to "Sentence case" (capitalize first letter of joined string)
 * - Returns null for empty topic or topic === "other"
 */

import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import React from "react";
import { TopicChip } from "./TopicChip";

describe("TopicChip (08-08 Task 1)", () => {
  it("renders with rounded-full pill shape", () => {
    const { container } = render(<TopicChip topic="finance" />);
    const chip = container.firstChild as HTMLElement;
    expect(chip?.className).toContain("rounded-full");
  });

  it("renders with accent-primary tinted background", () => {
    const { container } = render(<TopicChip topic="finance" />);
    const chip = container.firstChild as HTMLElement;
    expect(chip?.className).toContain("bg-accent-primary");
  });

  it("renders a Bookmark icon (SVG element present)", () => {
    const { container } = render(<TopicChip topic="finance" />);
    const svg = container.querySelector("svg");
    expect(svg).not.toBeNull();
  });

  it("transforms snake_case to Sentence case: term_insurance → Term insurance", () => {
    render(<TopicChip topic="term_insurance" />);
    expect(screen.getByText("Term insurance")).toBeDefined();
  });

  it("transforms multi-word: identity_document → Identity document", () => {
    render(<TopicChip topic="identity_document" />);
    expect(screen.getByText("Identity document")).toBeDefined();
  });

  it("transforms single word with no underscore", () => {
    render(<TopicChip topic="finance" />);
    expect(screen.getByText("Finance")).toBeDefined();
  });

  it("returns null for empty string topic", () => {
    const { container } = render(<TopicChip topic="" />);
    expect(container.firstChild).toBeNull();
  });

  it("returns null for topic === 'other'", () => {
    const { container } = render(<TopicChip topic="other" />);
    expect(container.firstChild).toBeNull();
  });

  it("has correct text color class", () => {
    const { container } = render(<TopicChip topic="finance" />);
    const chip = container.firstChild as HTMLElement;
    expect(chip?.className).toContain("text-accent-primary");
  });
});
