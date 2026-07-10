/**
 * Tests for OnboardingPage 5-step wizard extension (AIPV-06 / Plan 07-06).
 *
 * Tests:
 * 1. Clicking Welcome's Continue reaches "Connect your AI" heading (Step 2 at index 1)
 * 2. Connect AI step Continue button is disabled until a provider is connected
 * 3. Skip button advances past Connect AI step WITHOUT dismissing the banner store
 *
 * TDD history: RED tests written against 4-step OnboardingPage (fails at step 1:
 * no "Connect your AI" heading reachable from Welcome's Continue). GREEN tests pass
 * after OnboardingPage is extended to 5 steps with ConnectAiStep mounted at index 1.
 *
 * D-14/D-15 invariant: Skip MUST NOT call useAiBannerStore.dismiss().
 * Verified by Test 3: isDismissed remains false after Skip.
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import React from "react";
import { MemoryRouter } from "react-router-dom";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";

// ---------------------------------------------------------------------------
// Hoist mocks — must be hoisted so vi.mock factories can reference them
// ---------------------------------------------------------------------------

const {
  mockMutateAsync,
  mockNavigate,
  mockSetCompleted,
} = vi.hoisted(() => ({
  mockMutateAsync: vi.fn().mockResolvedValue({ started: true, provider: "anthropic" }),
  mockNavigate: vi.fn(),
  mockSetCompleted: vi.fn(),
}));

// ---------------------------------------------------------------------------
// Module mocks
// ---------------------------------------------------------------------------

// Mock @/hooks/useTauri — controls what providers are "connected"
// Default: empty providers array (no provider connected)
let mockProvidersData: { provider: string; authenticated: boolean; isActive: boolean; model: string | null }[] = [];

vi.mock("@/hooks/useTauri", () => ({
  useProviders: () => ({ data: mockProvidersData }),
  useSaveSetupToken: () => ({ mutateAsync: mockMutateAsync }),
  useConnectProvider: () => ({ mutateAsync: mockMutateAsync }),
  useAddWatchedFolder: () => ({ mutateAsync: vi.fn().mockResolvedValue({ id: "f1", path: "/tmp" }) }),
  useTriggerScan: () => ({ mutateAsync: vi.fn().mockResolvedValue(undefined) }),
  useSpaces: () => ({ data: [] }),
}));

vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: vi.fn().mockResolvedValue(null),
}));

vi.mock("@/lib/tauri", () => ({
  isTauri: () => false,
  tauriInvoke: vi.fn(),
}));

vi.mock("sonner", () => ({
  toast: { success: vi.fn(), error: vi.fn() },
  Toaster: () => null,
}));

vi.mock("react-router-dom", async (importOriginal) => {
  const actual = await importOriginal<typeof import("react-router-dom")>();
  return {
    ...actual,
    useNavigate: () => mockNavigate,
  };
});

vi.mock("@/lib/stores", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@/lib/stores")>();
  return {
    ...actual,
    useOnboardingStore: () => ({
      isCompleted: false,
      setCompleted: mockSetCompleted,
      reset: vi.fn(),
    }),
  };
});

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

import OnboardingPage from "./OnboardingPage";
import { useAiBannerStore } from "@/lib/stores";

function makeQueryClient() {
  return new QueryClient({ defaultOptions: { queries: { retry: false } } });
}

function renderOnboarding() {
  const qc = makeQueryClient();
  return render(
    <QueryClientProvider client={qc}>
      <MemoryRouter>
        <OnboardingPage />
      </MemoryRouter>
    </QueryClientProvider>,
  );
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("OnboardingPage 5-step wizard", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockProvidersData = [];
    // Reset banner store to undismissed state before each test
    useAiBannerStore.getState().isDismissed && useAiBannerStore.setState({ isDismissed: false });
  });

  it("has 5 steps — clicking Welcome Continue reveals 'Connect your AI' heading", () => {
    renderOnboarding();

    // Step 0: Welcome is rendered
    expect(screen.getByText("Welcome to Cortex")).toBeInTheDocument();

    // Click "Get Started" button (Welcome's Continue)
    const getStartedBtn = screen.getByRole("button", { name: /get started/i });
    fireEvent.click(getStartedBtn);

    // Step 1: NEW Connect AI step is rendered
    expect(screen.getByText("Connect your AI")).toBeInTheDocument();
  });

  it("Connect AI step Continue button is disabled until a provider is connected", () => {
    // Empty providers — nothing connected
    mockProvidersData = [];
    renderOnboarding();

    // Advance to Connect AI step
    const getStartedBtn = screen.getByRole("button", { name: /get started/i });
    fireEvent.click(getStartedBtn);

    // "Connect your AI" heading is visible
    expect(screen.getByText("Connect your AI")).toBeInTheDocument();

    // Continue button should be disabled
    const continueBtn = screen.getByRole("button", { name: /continue/i });
    expect(continueBtn).toBeDisabled();
  });

  it("Skip button advances past Connect AI step without dismissing the banner store", () => {
    mockProvidersData = [];
    renderOnboarding();

    // Advance to Connect AI step
    const getStartedBtn = screen.getByRole("button", { name: /get started/i });
    fireEvent.click(getStartedBtn);
    expect(screen.getByText("Connect your AI")).toBeInTheDocument();

    // Confirm banner store is NOT dismissed yet
    expect(useAiBannerStore.getState().isDismissed).toBe(false);

    // Click "Skip for now"
    const skipBtn = screen.getByRole("button", { name: /skip for now/i });
    fireEvent.click(skipBtn);

    // "Connect your AI" heading should no longer be visible (advanced past step 1)
    expect(screen.queryByText("Connect your AI")).not.toBeInTheDocument();

    // D-14/D-15 invariant: banner store must remain NOT dismissed after Skip
    expect(useAiBannerStore.getState().isDismissed).toBe(false);
  });
});
