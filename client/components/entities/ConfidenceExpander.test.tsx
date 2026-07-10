/**
 * Tests for ConfidenceExpander component (Plan 08-08 Task 1 RED)
 *
 * Behavior:
 * - Filters entities to those with confidence < 0.7
 * - Returns null when no low-confidence entities
 * - Shows "Also found ({count})" trigger with ChevronDown
 * - Correct aria-label on trigger
 * - Renders low-confidence entities via renderEntity prop after opening
 */

import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import React from "react";
import type { ExtractedEntity } from "@/lib/types";
import { ConfidenceExpander } from "./ConfidenceExpander";

// 3 test entities: confidence [0.9, 0.65, 0.5] → 2 are low-confidence
const highConfEntity: ExtractedEntity = {
  label: "Date",
  value: "2024-01-01",
  entityType: "date",
  confidence: 0.9,
};
const lowConfEntity1: ExtractedEntity = {
  label: "Amount",
  value: "100.00",
  entityType: "amount",
  confidence: 0.65,
};
const lowConfEntity2: ExtractedEntity = {
  label: "Email",
  value: "a@b.com",
  entityType: "email",
  confidence: 0.5,
};

describe("ConfidenceExpander (08-08 Task 1)", () => {
  it("shows 'Also found (2)' when 2 entities have confidence < 0.7", () => {
    render(
      <ConfidenceExpander
        entities={[highConfEntity, lowConfEntity1, lowConfEntity2]}
        renderEntity={(e) => <span key={e.value}>{e.value}</span>}
      />,
    );
    expect(screen.getByText(/Also found \(2\)/)).toBeDefined();
  });

  it("returns null when no entities have confidence < 0.7", () => {
    const { container } = render(
      <ConfidenceExpander
        entities={[highConfEntity]}
        renderEntity={() => <span>entity</span>}
      />,
    );
    expect(container.firstChild).toBeNull();
  });

  it("returns null for empty entities array", () => {
    const { container } = render(
      <ConfidenceExpander
        entities={[]}
        renderEntity={() => <span>entity</span>}
      />,
    );
    expect(container.firstChild).toBeNull();
  });

  it("trigger button has correct aria-label", () => {
    render(
      <ConfidenceExpander
        entities={[highConfEntity, lowConfEntity1, lowConfEntity2]}
        renderEntity={(e) => <span key={e.value}>{e.value}</span>}
      />,
    );
    const trigger = screen.getByRole("button");
    expect(trigger.getAttribute("aria-label")).toBe(
      "Low-confidence entities — may contain OCR errors",
    );
  });

  it("shows low-confidence entity values in content after trigger click", () => {
    render(
      <ConfidenceExpander
        entities={[highConfEntity, lowConfEntity1, lowConfEntity2]}
        renderEntity={(e) => (
          <span key={e.value} data-testid={`chip-${e.value}`}>
            {e.value}
          </span>
        )}
      />,
    );
    const trigger = screen.getByRole("button");
    fireEvent.click(trigger);
    // After opening, the low-confidence entities should be visible
    expect(screen.getByTestId(`chip-${lowConfEntity1.value}`)).toBeDefined();
    expect(screen.getByTestId(`chip-${lowConfEntity2.value}`)).toBeDefined();
  });

  it("does NOT include high-confidence entities", () => {
    render(
      <ConfidenceExpander
        entities={[highConfEntity, lowConfEntity1, lowConfEntity2]}
        renderEntity={(e) => (
          <span key={e.value} data-testid={`chip-${e.value}`}>
            {e.value}
          </span>
        )}
      />,
    );
    fireEvent.click(screen.getByRole("button"));
    // High-confidence entity should NOT be rendered by renderEntity
    expect(screen.queryByTestId(`chip-${highConfEntity.value}`)).toBeNull();
  });

  it("returns null when only entity has no confidence value", () => {
    const noConfEntity: ExtractedEntity = {
      label: "Name",
      value: "John",
      entityType: "person",
      // confidence is undefined
    };
    const { container } = render(
      <ConfidenceExpander
        entities={[noConfEntity]}
        renderEntity={() => <span>entity</span>}
      />,
    );
    // No confidence means NOT a low-confidence entity (treated as high-confidence)
    expect(container.firstChild).toBeNull();
  });
});
