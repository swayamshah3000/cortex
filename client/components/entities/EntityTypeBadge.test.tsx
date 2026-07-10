/**
 * Tests for EntityTypeBadge component (Plan 06-06 Task 1, Test 2)
 *
 * Test 2: EntityTypeBadge renders a pill with capitalized type name and
 * correct bg/text classes resolved via tokenMap for all 6 types.
 */

import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import React from "react";
import { EntityTypeBadge } from "./EntityTypeBadge";

describe("EntityTypeBadge (06-06 Task 1 - Test 2)", () => {
  it("renders capitalized type name for 'person'", () => {
    render(<EntityTypeBadge entityType="person" />);
    expect(screen.getByText("Person")).toBeDefined();
  });

  it("renders capitalized type name for 'organization'", () => {
    render(<EntityTypeBadge entityType="organization" />);
    expect(screen.getByText("Organization")).toBeDefined();
  });

  it("renders capitalized type name for 'location'", () => {
    render(<EntityTypeBadge entityType="location" />);
    expect(screen.getByText("Location")).toBeDefined();
  });

  it("renders capitalized type name for 'date'", () => {
    render(<EntityTypeBadge entityType="date" />);
    expect(screen.getByText("Date")).toBeDefined();
  });

  it("renders capitalized type name for 'amount'", () => {
    render(<EntityTypeBadge entityType="amount" />);
    expect(screen.getByText("Amount")).toBeDefined();
  });

  it("renders capitalized type name for 'email'", () => {
    render(<EntityTypeBadge entityType="email" />);
    expect(screen.getByText("Email")).toBeDefined();
  });

  it("applies purple color classes for person type", () => {
    const { container } = render(<EntityTypeBadge entityType="person" />);
    const badge = container.firstElementChild as HTMLElement;
    expect(badge.className).toContain("text-purple-400");
    expect(badge.className).toContain("bg-purple-400/10");
  });

  it("applies amber color classes for organization type", () => {
    const { container } = render(<EntityTypeBadge entityType="organization" />);
    const badge = container.firstElementChild as HTMLElement;
    expect(badge.className).toContain("text-amber-400");
    expect(badge.className).toContain("bg-amber-400/10");
  });

  it("applies cyan color classes for email type", () => {
    const { container } = render(<EntityTypeBadge entityType="email" />);
    const badge = container.firstElementChild as HTMLElement;
    expect(badge.className).toContain("text-cyan-400");
    expect(badge.className).toContain("bg-cyan-400/10");
  });

  it("applies inline-flex and rounded-md layout classes", () => {
    const { container } = render(<EntityTypeBadge entityType="date" />);
    const badge = container.firstElementChild as HTMLElement;
    expect(badge.className).toContain("inline-flex");
    expect(badge.className).toContain("rounded-md");
  });
});
