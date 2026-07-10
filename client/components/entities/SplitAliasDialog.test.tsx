/**
 * Tests for SplitAliasDialog component (Plan 06-07 Task 1 - Test 5)
 */

import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import React from "react";
import { SplitAliasDialog } from "./SplitAliasDialog";

describe("SplitAliasDialog (06-07 Task 1)", () => {
  it("renders title with alias text when open", () => {
    const onConfirm = vi.fn();
    const onOpenChange = vi.fn();
    render(
      <SplitAliasDialog
        alias="J. Smith"
        open={true}
        onOpenChange={onOpenChange}
        onConfirm={onConfirm}
      />,
    );
    expect(screen.getByText(/Split "J. Smith" off\?/i)).toBeInTheDocument();
  });

  it("renders description text from UI-SPEC", () => {
    const onConfirm = vi.fn();
    const onOpenChange = vi.fn();
    render(
      <SplitAliasDialog
        alias="J. Smith"
        open={true}
        onOpenChange={onOpenChange}
        onConfirm={onConfirm}
      />,
    );
    expect(
      screen.getByText(/this alias will become its own entity/i),
    ).toBeInTheDocument();
  });

  it("renders 'Split alias' confirm button", () => {
    const onConfirm = vi.fn();
    const onOpenChange = vi.fn();
    render(
      <SplitAliasDialog
        alias="J. Smith"
        open={true}
        onOpenChange={onOpenChange}
        onConfirm={onConfirm}
      />,
    );
    expect(screen.getByRole("button", { name: /split alias/i })).toBeInTheDocument();
  });

  it("renders Cancel button", () => {
    const onConfirm = vi.fn();
    const onOpenChange = vi.fn();
    render(
      <SplitAliasDialog
        alias="J. Smith"
        open={true}
        onOpenChange={onOpenChange}
        onConfirm={onConfirm}
      />,
    );
    expect(screen.getByRole("button", { name: /cancel/i })).toBeInTheDocument();
  });

  it("Confirm button does NOT use destructive/red classes", () => {
    const onConfirm = vi.fn();
    const onOpenChange = vi.fn();
    render(
      <SplitAliasDialog
        alias="J. Smith"
        open={true}
        onOpenChange={onOpenChange}
        onConfirm={onConfirm}
      />,
    );
    const confirmBtn = screen.getByRole("button", { name: /split alias/i });
    expect(confirmBtn.className).not.toContain("destructive");
    expect(confirmBtn.className).not.toContain("bg-red-");
  });

  it("calls onConfirm when Confirm is clicked", async () => {
    const user = userEvent.setup();
    const onConfirm = vi.fn();
    const onOpenChange = vi.fn();
    render(
      <SplitAliasDialog
        alias="J. Smith"
        open={true}
        onOpenChange={onOpenChange}
        onConfirm={onConfirm}
      />,
    );
    await user.click(screen.getByRole("button", { name: /split alias/i }));
    expect(onConfirm).toHaveBeenCalledOnce();
  });

  it("does not render when open=false", () => {
    const onConfirm = vi.fn();
    const onOpenChange = vi.fn();
    render(
      <SplitAliasDialog
        alias="J. Smith"
        open={false}
        onOpenChange={onOpenChange}
        onConfirm={onConfirm}
      />,
    );
    expect(screen.queryByText(/Split "J. Smith" off\?/i)).not.toBeInTheDocument();
  });
});
