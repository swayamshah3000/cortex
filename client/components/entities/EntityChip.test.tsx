/**
 * Tests for EntityChip component
 *
 * Phase 06-06 (Task 1, Test 1):
 *   EntityChip renders a Link to /entities/{canonicalId ?? encodeURIComponent(value)}
 *   with correct aria-label and icon resolved by entityTypeIcon helper.
 *
 * Phase 08-08 (Task 2 RED):
 *   All 8 class icons, subclass Badge for Identifier, legacy entityType fallback.
 *
 * Phase 11-05 (Dual-navigation refactor):
 *   Left-click → /search?entity={class}:{value}
 *   Right-click → /entity/{class}/{value}
 *   isActive prop drives accent styling
 *   aria-label communicates dual navigation
 */

import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import React from "react";
import { MemoryRouter, Routes, Route, useLocation, useParams } from "react-router-dom";
import { EntityChip } from "./EntityChip";
import type { ExtractedEntity } from "@/lib/types";

// ---------------------------------------------------------------------------
// Helper: render with full router (for navigation tests)
// ---------------------------------------------------------------------------

function renderWithRouter(chip: React.ReactElement) {
  function SearchRoute() {
    const location = useLocation();
    return <span data-testid="search-route">SEARCH:{location.search}</span>;
  }

  function EntityRoute() {
    const params = useParams<{ class: string; value: string }>();
    return (
      <span data-testid="entity-route">
        ENTITY:{params.class}/{params.value}
      </span>
    );
  }

  return render(
    <MemoryRouter initialEntries={["/"]}>
      <Routes>
        <Route path="/" element={chip} />
        <Route path="/search" element={<SearchRoute />} />
        <Route path="/entity/:class/:value" element={<EntityRoute />} />
      </Routes>
    </MemoryRouter>,
  );
}

function renderChip(entity: { value: string; entityType: string; canonicalId?: string }) {
  return render(
    <MemoryRouter>
      <EntityChip entity={entity} />
    </MemoryRouter>,
  );
}

function renderChipPhase8(entity: ExtractedEntity) {
  return render(
    <MemoryRouter>
      <EntityChip entity={entity} />
    </MemoryRouter>,
  );
}

// ---------------------------------------------------------------------------
// Legacy tests updated for Phase 11-05 (button replaces link)
// ---------------------------------------------------------------------------

describe("EntityChip (06-06 Task 1 - Test 1)", () => {
  it("renders the entity value text", () => {
    renderChip({ value: "support@example.com", entityType: "email", canonicalId: "entity-email-1" });
    expect(screen.getByText("support@example.com")).toBeDefined();
  });

  it("renders building2 icon for organization type (amber color)", () => {
    const { container } = renderChip({
      value: "Acme Corp",
      entityType: "organization",
      canonicalId: "entity-123",
    });
    // Icon with amber-400 class should be present for organization
    const icon = container.querySelector(".text-amber-400");
    expect(icon).not.toBeNull();
  });

  it("renders mail icon for email type (cyan color)", () => {
    const { container } = renderChip({
      value: "test@example.com",
      entityType: "email",
      canonicalId: "entity-email-1",
    });
    const icon = container.querySelector(".text-cyan-400");
    expect(icon).not.toBeNull();
  });

  it("renders correct base styling classes on the button element", () => {
    renderChip({ value: "Brooklyn", entityType: "location", canonicalId: "entity-loc-1" });
    const btn = screen.getByRole("button");
    expect(btn.className).toContain("inline-flex");
    expect(btn.className).toContain("rounded-full");
    expect(btn.className).toContain("border-border-secondary");
    expect(btn.className).toContain("bg-bg-tertiary");
  });
});

// =============================================================================
// Phase 8 (08-08 Task 2 RED): 8-class icon map + subclass Badge
// =============================================================================

describe("EntityChip Phase 8 — 8-class icon map (08-08 Task 2)", () => {
  it("renders Phone icon (teal-400) for class='Phone'", () => {
    const { container } = renderChipPhase8({
      label: "Phone",
      value: "+91 98765 43210",
      entityType: "phone",
      class: "Phone",
    });
    expect(container.querySelector(".text-teal-400")).not.toBeNull();
  });

  it("renders Fingerprint icon (orange-400) for class='Identifier'", () => {
    const { container } = renderChipPhase8({
      label: "Aadhaar",
      value: "1234 5678 9012",
      entityType: "identifier",
      class: "Identifier",
      subclass: "aadhaar",
    });
    expect(container.querySelector(".text-orange-400")).not.toBeNull();
  });

  it("renders purple-400 icon for class='Person'", () => {
    const { container } = renderChipPhase8({
      label: "Owner",
      value: "Alex Doe",
      entityType: "person",
      class: "Person",
    });
    expect(container.querySelector(".text-purple-400")).not.toBeNull();
  });

  it("renders amber-400 icon for class='Organization'", () => {
    const { container } = renderChipPhase8({
      label: "Company",
      value: "Acme Corp",
      entityType: "organization",
      class: "Organization",
    });
    expect(container.querySelector(".text-amber-400")).not.toBeNull();
  });

  it("renders red-400 icon for class='Location'", () => {
    const { container } = renderChipPhase8({
      label: "City",
      value: "Mumbai",
      entityType: "location",
      class: "Location",
    });
    expect(container.querySelector(".text-red-400")).not.toBeNull();
  });

  it("renders blue-400 icon for class='Date'", () => {
    const { container } = renderChipPhase8({
      label: "Date",
      value: "2024-01-01",
      entityType: "date",
      class: "Date",
    });
    expect(container.querySelector(".text-blue-400")).not.toBeNull();
  });

  it("renders green-400 icon for class='Amount'", () => {
    const { container } = renderChipPhase8({
      label: "Amount",
      value: "₹50,000",
      entityType: "amount",
      class: "Amount",
    });
    expect(container.querySelector(".text-green-400")).not.toBeNull();
  });

  it("renders cyan-400 icon for class='Email'", () => {
    const { container } = renderChipPhase8({
      label: "Email",
      value: "user@example.com",
      entityType: "email",
      class: "Email",
    });
    expect(container.querySelector(".text-cyan-400")).not.toBeNull();
  });

  it("falls back to legacy entityType when class is absent", () => {
    const { container } = renderChipPhase8({
      label: "Person",
      value: "Jane Doe",
      entityType: "person",
      // class intentionally absent — should use legacy entityType mapping
    });
    expect(container.querySelector(".text-purple-400")).not.toBeNull();
  });

  it("renders subclass Badge for Identifier with known subclass='aadhaar'", () => {
    renderChipPhase8({
      label: "Aadhaar",
      value: "1234 5678 9012",
      entityType: "identifier",
      class: "Identifier",
      subclass: "aadhaar",
    });
    // Badge with subclass text should be present
    expect(screen.getByText("aadhaar")).toBeDefined();
  });

  it("does NOT render subclass Badge for subclass='unknown'", () => {
    renderChipPhase8({
      label: "ID",
      value: "XYZ-12345",
      entityType: "identifier",
      class: "Identifier",
      subclass: "unknown",
    });
    // "unknown" subclass should NOT appear (noisy, from Pass 1 weak-format IDs)
    expect(screen.queryByText("unknown")).toBeNull();
  });

  it("does NOT render subclass Badge for non-Identifier classes", () => {
    renderChipPhase8({
      label: "Date",
      value: "2024-01-01",
      entityType: "date",
      class: "Date",
      subclass: "some_subclass",
    });
    // Non-identifier classes should never show subclass Badge
    expect(screen.queryByText("some_subclass")).toBeNull();
  });
});

// =============================================================================
// Phase 11-05: Dual-navigation EntityChip refactor
// =============================================================================

describe("EntityChip Phase 11-05 — dual navigation (11-05 Task 1)", () => {
  it("Test 1 (left click): navigates to /search?entity={class}:{value} URL-encoded", () => {
    renderWithRouter(
      <EntityChip entity={{ value: "Alex Doe", entityType: "person" }} />,
    );

    const chip = screen.getByRole("button");
    fireEvent.click(chip);

    // encodeURIComponent("Person:Alex Doe") = "Person%3AAlex%20Shah"
    const searchRoute = screen.getByTestId("search-route");
    expect(searchRoute.textContent).toContain("?entity=Person%3AAlex%20Shah");
  });

  it("Test 2 (right click): navigates to /entity/{class}/{value} and suppresses native menu", () => {
    renderWithRouter(
      <EntityChip entity={{ value: "Alex Doe", entityType: "person" }} />,
    );

    const chip = screen.getByRole("button");

    // Use fireEvent.contextMenu to trigger React's synthetic onContextMenu handler
    // fireEvent returns false when the event's default was prevented
    const eventNotCancelled = fireEvent.contextMenu(chip);

    // React Router decodes URL params — route text shows unencoded value
    const entityRoute = screen.getByTestId("entity-route");
    expect(entityRoute.textContent).toContain("ENTITY:Person/Alex Doe");

    // fireEvent returns false when e.preventDefault() was called (event cancelled)
    expect(eventNotCancelled).toBe(false);
  });

  it("Test 3 (isActive styling): renders accent classes when isActive=true", () => {
    const { container } = renderWithRouter(
      <EntityChip entity={{ value: "Alex Doe", entityType: "person" }} isActive={true} />,
    );

    const button = container.querySelector("button");
    expect(button).not.toBeNull();
    const className = button!.className;
    expect(className).toContain("bg-accent-subtle");
    expect(className).toContain("text-accent-primary");
    expect(className).toContain("border-accent-primary/20");
  });

  it("Test 4 (default styling): renders bg-bg-tertiary and no accent text when isActive=false", () => {
    const { container } = renderWithRouter(
      <EntityChip entity={{ value: "Alex Doe", entityType: "person" }} isActive={false} />,
    );

    const button = container.querySelector("button");
    expect(button).not.toBeNull();
    const className = button!.className;
    expect(className).toContain("bg-bg-tertiary");
    expect(className).not.toContain("text-accent-primary");
  });

  it("Test 5 (Phase 8 explicit class): entity.class overrides entityType in URL param", () => {
    renderWithRouter(
      <EntityChip
        entity={{ value: "AlphaComplex", entityType: "organization", class: "Location" }}
      />,
    );

    const chip = screen.getByRole("button");
    fireEvent.click(chip);

    const searchRoute = screen.getByTestId("search-route");
    // encodeURIComponent("Location:AlphaComplex") = "Location%3AAlphaComplex"
    expect(searchRoute.textContent).toContain("?entity=Location%3AAlphaComplex");
    expect(searchRoute.textContent).not.toContain("Organization");
  });

  it("Test 6 (aria-label): communicates dual navigation intent", () => {
    renderWithRouter(
      <EntityChip entity={{ value: "Alex Doe", entityType: "person" }} />,
    );

    const chip = screen.getByRole("button");
    const ariaLabel = chip.getAttribute("aria-label");
    expect(ariaLabel).toContain("Filter by Person: Alex Doe");
    expect(ariaLabel).toContain("Right-click for entity detail page");
  });
});
