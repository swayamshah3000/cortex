/**
 * Tests for AliasChipList component (Plan 06-07 Task 1 - Test 3)
 *
 * Test: hides itself when aliases.length === 1 AND aliases[0] === canonicalName
 */

import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import React from "react";
import { AliasChipList } from "./AliasChipList";

describe("AliasChipList (06-07 Task 1)", () => {
  it("renders section when multiple aliases", () => {
    const onSplit = vi.fn();
    render(
      <AliasChipList
        aliases={["J. Smith", "Smith, John", "John Smith"]}
        canonicalName="John Smith"
        onSplit={onSplit}
      />,
    );
    expect(screen.getByText(/aliases/i)).toBeInTheDocument();
  });

  it("renders AliasChip for each alias", () => {
    const onSplit = vi.fn();
    render(
      <AliasChipList
        aliases={["J. Smith", "Smith, John", "John Smith"]}
        canonicalName="John Smith"
        onSplit={onSplit}
      />,
    );
    expect(screen.getByText("J. Smith")).toBeInTheDocument();
    expect(screen.getByText("Smith, John")).toBeInTheDocument();
    expect(screen.getByText("John Smith")).toBeInTheDocument();
  });

  it("hides itself when only one alias equals canonicalName", () => {
    const onSplit = vi.fn();
    const { container } = render(
      <AliasChipList
        aliases={["John Smith"]}
        canonicalName="John Smith"
        onSplit={onSplit}
      />,
    );
    // Section should render nothing
    expect(container.firstChild).toBeNull();
  });

  it("shows section description text", () => {
    const onSplit = vi.fn();
    render(
      <AliasChipList
        aliases={["J. Smith", "John Smith"]}
        canonicalName="John Smith"
        onSplit={onSplit}
      />,
    );
    expect(
      screen.getByText(/these surface forms were merged/i),
    ).toBeInTheDocument();
  });
});
