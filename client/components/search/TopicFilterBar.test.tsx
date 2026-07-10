/**
 * Tests for TopicFilterBar component (Phase 8 Plan 09).
 *
 * RED: Fails until TopicFilterBar.tsx exports TopicFilterBar and TopicFilterChip.
 *
 * Covers all must_have truths from the plan:
 * - Returns null when data is undefined
 * - Returns null when data is empty array
 * - Renders 20 chips when 20 topics; no Show more
 * - Renders 20 chips + Show more when 21+ topics; click Show more shows next 20
 * - Clicking inactive chip fires onSelect(topic)
 * - Clicking the currently-selected chip fires onSelect(null)
 * - Active chip has bg-accent-primary class; inactive does not
 */
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import React from "react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { TopicFilterBar } from "./TopicFilterBar";

// Mock useTopics hook so tests run without QueryClient internals / Tauri
const mockUseTopics = vi.fn();

vi.mock("@/hooks/useTauri", () => ({
  useTopics: () => mockUseTopics(),
}));

function makeTopics(n: number) {
  return Array.from({ length: n }, (_, i) => ({
    topic: `topic_${String(i + 1).padStart(2, "0")}`,
    count: n - i,
  }));
}

function renderBar(props: { selected: string | null; onSelect: (t: string | null) => void }) {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <TopicFilterBar {...props} />
    </QueryClientProvider>,
  );
}

describe("TopicFilterBar — null-render cases", () => {
  const onSelect = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("returns null when data is undefined (loading state)", () => {
    mockUseTopics.mockReturnValue({ data: undefined, isLoading: true });
    const { container } = renderBar({ selected: null, onSelect });
    expect(container.firstChild).toBeNull();
  });

  it("returns null when data is empty array (0 topics indexed)", () => {
    mockUseTopics.mockReturnValue({ data: [], isLoading: false });
    const { container } = renderBar({ selected: null, onSelect });
    expect(container.firstChild).toBeNull();
  });
});

describe("TopicFilterBar — chip rendering and pagination", () => {
  const onSelect = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renders exactly 20 chips when data has exactly 20 topics", () => {
    mockUseTopics.mockReturnValue({ data: makeTopics(20), isLoading: false });
    renderBar({ selected: null, onSelect });
    expect(screen.getAllByTestId("topic-filter-chip")).toHaveLength(20);
  });

  it("does NOT render Show more when data has 20 topics (1-20 range)", () => {
    mockUseTopics.mockReturnValue({ data: makeTopics(20), isLoading: false });
    renderBar({ selected: null, onSelect });
    expect(screen.queryByText("Show more")).toBeNull();
  });

  it("renders 20 chips + Show more when data has 25 topics", () => {
    mockUseTopics.mockReturnValue({ data: makeTopics(25), isLoading: false });
    renderBar({ selected: null, onSelect });
    expect(screen.getAllByTestId("topic-filter-chip")).toHaveLength(20);
    expect(screen.getByText("Show more")).toBeTruthy();
  });

  it("clicking Show more reveals all 25 topics (5 more shown)", () => {
    mockUseTopics.mockReturnValue({ data: makeTopics(25), isLoading: false });
    renderBar({ selected: null, onSelect });
    fireEvent.click(screen.getByText("Show more"));
    expect(screen.getAllByTestId("topic-filter-chip")).toHaveLength(25);
  });

  it("renders label 'Topics:' before the chip row", () => {
    mockUseTopics.mockReturnValue({ data: makeTopics(3), isLoading: false });
    renderBar({ selected: null, onSelect });
    expect(screen.getByText("Topics:")).toBeTruthy();
  });
});

describe("TopicFilterBar — chip selection behavior", () => {
  const onSelect = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("clicking an inactive chip calls onSelect(topic)", () => {
    mockUseTopics.mockReturnValue({
      data: [{ topic: "finance", count: 12 }],
      isLoading: false,
    });
    renderBar({ selected: null, onSelect });
    fireEvent.click(screen.getByTestId("topic-filter-chip"));
    expect(onSelect).toHaveBeenCalledWith("finance");
  });

  it("clicking the currently-selected chip calls onSelect(null)", () => {
    mockUseTopics.mockReturnValue({
      data: [{ topic: "finance", count: 12 }],
      isLoading: false,
    });
    renderBar({ selected: "finance", onSelect });
    fireEvent.click(screen.getByTestId("topic-filter-chip"));
    expect(onSelect).toHaveBeenCalledWith(null);
  });
});

describe("TopicFilterBar — active/inactive chip styling", () => {
  const onSelect = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("active chip has bg-accent-primary class", () => {
    mockUseTopics.mockReturnValue({
      data: [
        { topic: "finance", count: 12 },
        { topic: "kids", count: 4 },
      ],
      isLoading: false,
    });
    renderBar({ selected: "finance", onSelect });
    const chips = screen.getAllByTestId("topic-filter-chip");
    expect(chips[0].className).toContain("bg-accent-primary");
  });

  it("inactive chip does NOT have bg-accent-primary class", () => {
    mockUseTopics.mockReturnValue({
      data: [
        { topic: "finance", count: 12 },
        { topic: "kids", count: 4 },
      ],
      isLoading: false,
    });
    renderBar({ selected: "finance", onSelect });
    const chips = screen.getAllByTestId("topic-filter-chip");
    expect(chips[1].className).not.toContain("bg-accent-primary");
  });

  it("inactive chip has bg-bg-secondary class", () => {
    mockUseTopics.mockReturnValue({
      data: [{ topic: "finance", count: 12 }],
      isLoading: false,
    });
    renderBar({ selected: "kids", onSelect });
    const chip = screen.getByTestId("topic-filter-chip");
    expect(chip.className).toContain("bg-bg-secondary");
  });
});

describe("TopicFilterBar — topic display transform", () => {
  const onSelect = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renders 'term_insurance' as 'Term insurance' (snake_case → sentence case)", () => {
    mockUseTopics.mockReturnValue({
      data: [{ topic: "term_insurance", count: 5 }],
      isLoading: false,
    });
    renderBar({ selected: null, onSelect });
    expect(screen.getByText(/Term insurance/)).toBeTruthy();
  });
});
