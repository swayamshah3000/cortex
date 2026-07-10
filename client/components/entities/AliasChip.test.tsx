/**
 * Tests for AliasChip component (Plan 06-07 Task 1 - Test 4)
 */

import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import React from "react";
import { AliasChip } from "./AliasChip";

describe("AliasChip (06-07 Task 1)", () => {
  it("renders alias text", () => {
    const onSplit = vi.fn();
    render(<AliasChip alias="J. Smith" isCanonical={false} onSplit={onSplit} />);
    expect(screen.getByText("J. Smith")).toBeInTheDocument();
  });

  it("renders scissors button with correct aria-label for non-canonical", () => {
    const onSplit = vi.fn();
    render(<AliasChip alias="J. Smith" isCanonical={false} onSplit={onSplit} />);
    const btn = screen.getByRole("button", { name: /split alias 'J. Smith' off/i });
    expect(btn).toBeInTheDocument();
  });

  it("scissors button has opacity-0 class (hidden by default)", () => {
    const onSplit = vi.fn();
    render(<AliasChip alias="J. Smith" isCanonical={false} onSplit={onSplit} />);
    const btn = screen.getByRole("button", { name: /split alias/i });
    expect(btn.className).toContain("opacity-0");
  });

  it("clicking scissors button calls onSplit", async () => {
    const user = userEvent.setup();
    const onSplit = vi.fn();
    render(<AliasChip alias="J. Smith" isCanonical={false} onSplit={onSplit} />);
    const btn = screen.getByRole("button", { name: /split alias/i });
    await user.click(btn);
    expect(onSplit).toHaveBeenCalledOnce();
  });

  it("hides scissors button when isCanonical=true", () => {
    const onSplit = vi.fn();
    render(<AliasChip alias="John Smith" isCanonical={true} onSplit={onSplit} />);
    expect(screen.queryByRole("button", { name: /split alias/i })).not.toBeInTheDocument();
  });

  it("shows Check icon when isCanonical=true", () => {
    const onSplit = vi.fn();
    const { container } = render(
      <AliasChip alias="John Smith" isCanonical={true} onSplit={onSplit} />,
    );
    // Check icon should be present in the chip — look for its test-id or class
    const canonicalIndicator = container.querySelector(".text-accent-primary");
    expect(canonicalIndicator).toBeInTheDocument();
  });
});
