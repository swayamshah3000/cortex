/**
 * Tests for EntityDetailHeader component (Plan 06-07 Task 1 - Tests 2, 4 for header)
 *
 * Test: rename pencil click → edit mode; Enter saves; Esc cancels
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import React from "react";
import { EntityDetailHeader } from "./EntityDetailHeader";
import type { CanonicalEntity } from "@/lib/types";

const mockEntity: CanonicalEntity = {
  id: "entity-001",
  canonicalName: "John Smith",
  entityType: "person",
  aliases: ["J. Smith", "Smith, John", "John Smith"],
  documentCount: 12,
};

describe("EntityDetailHeader (06-07 Task 1)", () => {
  it("renders canonical name, type badge, and document count", () => {
    const onRename = vi.fn();
    render(<EntityDetailHeader entity={mockEntity} onRename={onRename} />);
    expect(screen.getByText("John Smith")).toBeInTheDocument();
    expect(screen.getByText("12 documents")).toBeInTheDocument();
  });

  it("shows rename pencil button", () => {
    const onRename = vi.fn();
    render(<EntityDetailHeader entity={mockEntity} onRename={onRename} />);
    const pencilBtn = screen.getByRole("button", { name: /rename canonical name/i });
    expect(pencilBtn).toBeInTheDocument();
  });

  it("clicking pencil enters edit mode with input", async () => {
    const user = userEvent.setup();
    const onRename = vi.fn();
    render(<EntityDetailHeader entity={mockEntity} onRename={onRename} />);
    const pencilBtn = screen.getByRole("button", { name: /rename canonical name/i });
    await user.click(pencilBtn);
    const input = screen.getByRole("textbox");
    expect(input).toBeInTheDocument();
    expect((input as HTMLInputElement).value).toBe("John Smith");
  });

  it("pressing Enter dispatches onRename with new name", async () => {
    const user = userEvent.setup();
    const onRename = vi.fn();
    render(<EntityDetailHeader entity={mockEntity} onRename={onRename} />);
    await user.click(screen.getByRole("button", { name: /rename canonical name/i }));
    const input = screen.getByRole("textbox");
    await user.clear(input);
    await user.type(input, "Jane Smith");
    await user.keyboard("{Enter}");
    expect(onRename).toHaveBeenCalledWith("Jane Smith");
  });

  it("pressing Escape cancels edit and restores original name", async () => {
    const user = userEvent.setup();
    const onRename = vi.fn();
    render(<EntityDetailHeader entity={mockEntity} onRename={onRename} />);
    await user.click(screen.getByRole("button", { name: /rename canonical name/i }));
    const input = screen.getByRole("textbox");
    await user.clear(input);
    await user.type(input, "Wrong Name");
    await user.keyboard("{Escape}");
    expect(onRename).not.toHaveBeenCalled();
    // Edit mode should be exited
    expect(screen.queryByRole("textbox")).not.toBeInTheDocument();
  });

  it("input has maxLength=200", async () => {
    const user = userEvent.setup();
    const onRename = vi.fn();
    render(<EntityDetailHeader entity={mockEntity} onRename={onRename} />);
    await user.click(screen.getByRole("button", { name: /rename canonical name/i }));
    const input = screen.getByRole("textbox");
    expect(input).toHaveAttribute("maxLength", "200");
  });
});
