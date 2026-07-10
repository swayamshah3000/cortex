/**
 * ExtractionSettings.test.tsx
 *
 * Tests for the ExtractionSettings component (Plan 08-07):
 * - Section heading + description renders
 * - Model dropdown shows correct options per active provider
 * - Dropdown disabled when no provider
 * - Re-extract button disabled states (no provider, toggle off, in-flight)
 * - Re-extract click invokes useTriggerEntityBackfill.mutate()
 * - estimateCost pure-function unit tests (tooltip content)
 * - Model-switch toast fires when model changes with docs indexed
 */

import { describe, it, expect, vi, beforeEach, beforeAll } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { TooltipProvider } from "@/components/ui/tooltip";
import { ExtractionSettings, estimateCost } from "./ExtractionSettings";

// --- Mock sonner toast -------------------------------------------------------

vi.mock("sonner", () => ({
  toast: {
    info: vi.fn(),
    success: vi.fn(),
    error: vi.fn(),
    warning: vi.fn(),
  },
}));

// --- Mock hooks --------------------------------------------------------------

vi.mock("@/hooks/useTauri", () => ({
  useExtractionSettings: vi.fn(),
  useUpdateExtractionSettings: vi.fn(),
  useTriggerEntityBackfill: vi.fn(),
  useProviders: vi.fn(),
  useStats: vi.fn(),
}));

// --- Mock stores -------------------------------------------------------------

vi.mock("@/lib/stores", async (importOriginal) => {
  const original = await importOriginal<typeof import("@/lib/stores")>();
  return {
    ...original,
    useBackfillStore: vi.fn(),
  };
});

import * as hooks from "@/hooks/useTauri";
import * as stores from "@/lib/stores";
import { toast } from "sonner";

// --- Helpers -----------------------------------------------------------------

function makeWrapper() {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  return ({ children }: { children: React.ReactNode }) => (
    <QueryClientProvider client={queryClient}>
      <TooltipProvider>{children}</TooltipProvider>
    </QueryClientProvider>
  );
}

const anthropicActiveProviders = [
  {
    provider: "anthropic",
    authenticated: true,
    method: "oauth",
    displayName: "Claude (Subscription)",
    model: "claude-haiku-4-5-20251001",
    isActive: true,
  },
];

const ollamaActiveProviders = [
  {
    provider: "ollama",
    authenticated: true,
    method: "none",
    displayName: null,
    model: "llama3:latest",
    isActive: true,
  },
];

function setupDefaultMocks(overrides: {
  providers?: typeof anthropicActiveProviders;
  extractionModel?: string;
  useLlmExtraction?: boolean;
  backfillStatus?: "idle" | "running" | "complete" | "error";
  docCount?: number;
  mutateFn?: () => void;
  updateMutateFn?: (s: unknown) => void;
} = {}) {
  vi.mocked(hooks.useExtractionSettings).mockReturnValue({
    data: {
      extractionModel: overrides.extractionModel ?? "claude-haiku-4-5-20251001",
      useLlmExtraction: overrides.useLlmExtraction ?? true,
    },
    isLoading: false,
  } as ReturnType<typeof hooks.useExtractionSettings>);

  vi.mocked(hooks.useUpdateExtractionSettings).mockReturnValue({
    mutate: overrides.updateMutateFn ?? vi.fn(),
    isPending: false,
  } as unknown as ReturnType<typeof hooks.useUpdateExtractionSettings>);

  vi.mocked(hooks.useTriggerEntityBackfill).mockReturnValue({
    mutate: overrides.mutateFn ?? vi.fn(),
    isPending: false,
  } as unknown as ReturnType<typeof hooks.useTriggerEntityBackfill>);

  vi.mocked(hooks.useProviders).mockReturnValue({
    data: overrides.providers ?? anthropicActiveProviders,
    isLoading: false,
  } as ReturnType<typeof hooks.useProviders>);

  vi.mocked(hooks.useStats).mockReturnValue({
    data: {
      totalDocuments: overrides.docCount ?? 100,
      smartSpaces: 5,
      lastScan: new Date().toISOString(),
      indexSize: 0,
    },
  } as ReturnType<typeof hooks.useStats>);

  vi.mocked(stores.useBackfillStore).mockReturnValue({
    status: overrides.backfillStatus ?? "idle",
    processed: 0,
    total: 0,
    error: null,
    etaSeconds: null,
    fallbacks: null,
    setProgress: vi.fn(),
    reset: vi.fn(),
  } as unknown as ReturnType<typeof stores.useBackfillStore>);
}

// --- Polyfills for Radix UI Select in jsdom ----------------------------------
// jsdom does not implement pointer capture APIs; Radix UI Select requires them.

beforeAll(() => {
  Element.prototype.setPointerCapture = vi.fn();
  Element.prototype.releasePointerCapture = vi.fn();
  Element.prototype.hasPointerCapture = () => false;
  Element.prototype.scrollIntoView = vi.fn();
  if (!window.ResizeObserver) {
    window.ResizeObserver = class ResizeObserver {
      observe() {}
      unobserve() {}
      disconnect() {}
    };
  }
});

// --- Tests -------------------------------------------------------------------

describe("ExtractionSettings", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    setupDefaultMocks();
  });

  // --- 1. Rendering ---

  it("renders section heading 'Entity Extraction'", () => {
    render(<ExtractionSettings />, { wrapper: makeWrapper() });
    expect(screen.getByText("Entity Extraction")).toBeInTheDocument();
  });

  it("renders description about model selection", () => {
    render(<ExtractionSettings />, { wrapper: makeWrapper() });
    expect(
      screen.getByText(/Choose which model Cortex uses to extract/i),
    ).toBeInTheDocument();
  });

  // --- 2. Model Dropdown ---

  it("shows selected Anthropic model when active provider is anthropic", () => {
    render(<ExtractionSettings />, { wrapper: makeWrapper() });
    // The SelectValue should display "Claude Haiku 4.5"
    expect(screen.getByText("Claude Haiku 4.5")).toBeInTheDocument();
  });

  it("renders model dropdown as disabled when no active provider is connected", () => {
    setupDefaultMocks({ providers: [] });
    render(<ExtractionSettings />, { wrapper: makeWrapper() });
    const trigger = screen.getByRole("combobox");
    expect(trigger).toBeDisabled();
  });

  // --- 3. Re-extract Button Disabled States ---

  it("disables Re-extract button when useLlmExtraction is false", () => {
    setupDefaultMocks({ useLlmExtraction: false });
    render(<ExtractionSettings />, { wrapper: makeWrapper() });
    const button = screen.getByRole("button", { name: /Re-extract entities/i });
    expect(button).toBeDisabled();
  });

  it("disables Re-extract button when no active provider", () => {
    setupDefaultMocks({ providers: [] });
    render(<ExtractionSettings />, { wrapper: makeWrapper() });
    const button = screen.getByRole("button", { name: /Re-extract entities/i });
    expect(button).toBeDisabled();
  });

  it("disables Re-extract button when backfill is already running", () => {
    setupDefaultMocks({ backfillStatus: "running" });
    render(<ExtractionSettings />, { wrapper: makeWrapper() });
    const button = screen.getByRole("button", { name: /Re-extract/i });
    expect(button).toBeDisabled();
  });

  // --- 4. Re-extract Click ---

  it("calls useTriggerEntityBackfill.mutate when Re-extract button is clicked", () => {
    const mockMutate = vi.fn();
    setupDefaultMocks({ mutateFn: mockMutate });
    render(<ExtractionSettings />, { wrapper: makeWrapper() });
    const button = screen.getByRole("button", { name: /Re-extract entities/i });
    fireEvent.click(button);
    expect(mockMutate).toHaveBeenCalledOnce();
  });

  // --- 5. estimateCost pure function ---

  it("estimateCost: correct string for Claude Haiku with 100 docs ($0.16)", () => {
    // 100 docs × 2000 tokens/doc = 200,000 tokens = 0.2M × $0.80/M = $0.16
    const result = estimateCost("claude-haiku-4-5-20251001", 100, "anthropic");
    expect(result).toBe("Est: $0.16 across 100 docs on Claude Haiku 4.5");
  });

  it("estimateCost: correct string for GPT-5 mini with 50 docs ($0.04)", () => {
    // 50 × 2000 = 100,000 tokens = 0.1M × $0.40/M = $0.04
    const result = estimateCost("gpt-5-mini", 50, "openai");
    expect(result).toBe("Est: $0.04 across 50 docs on GPT-5 mini");
  });

  it("estimateCost: free variant for Ollama provider", () => {
    const result = estimateCost("llama3:latest", 100, "ollama");
    expect(result).toBe("Est: free (local model) across 100 docs");
  });

  it("estimateCost: 'No documents to re-extract' when docCount is 0", () => {
    const result = estimateCost("claude-haiku-4-5-20251001", 0, "anthropic");
    expect(result).toBe("No documents to re-extract");
  });

  // --- 6. Model-switch toast ---

  it("fires model-switch info toast when model changes with docs indexed", async () => {
    const user = userEvent.setup();
    const mockUpdateMutate = vi.fn();
    setupDefaultMocks({ updateMutateFn: mockUpdateMutate, docCount: 100 });
    render(<ExtractionSettings />, { wrapper: makeWrapper() });

    // Open the model dropdown
    const trigger = screen.getByRole("combobox");
    await user.click(trigger);

    // Select "Claude Sonnet 4.5"
    const option = await screen.findByRole("option", { name: "Claude Sonnet 4.5" });
    await user.click(option);

    await waitFor(() => {
      expect(toast.info).toHaveBeenCalledWith(
        expect.stringContaining("Claude Sonnet 4.5"),
        expect.objectContaining({ duration: 5000 }),
      );
    });
  });
});
