/**
 * ProviderCard.test.tsx
 *
 * Tests AIPV-05: radio item must be disabled for unconnected providers
 * and enabled for authenticated providers.
 *
 * Wave 0 gap from RESEARCH.md — ensures the RadioGroupItem disabled invariant
 * is always enforced regardless of future refactors.
 *
 * Tests AIPV-02, AIPV-03 (Plan 07-10): two-mode OpenAI card with
 * "Sign in with ChatGPT" primary CTA and "Use API key instead" toggle.
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { RadioGroup } from "@/components/ui/radio-group";
import { ProviderCard } from "./ProviderCard";
import type { ProviderAuthStatus } from "@/lib/types";

// Top-level spy target for useStartOpenAiOAuth — allows tests to override behavior.
// Hoisted mocks can reference this because it is defined at module scope.
const mockMutateAsync = vi.fn().mockResolvedValue({});

vi.mock("@/hooks/useTauri", async (importOriginal) => {
  const original = await importOriginal<typeof import("@/hooks/useTauri")>();
  return {
    ...original,
    useStartOpenAiOAuth: () => ({
      mutateAsync: mockMutateAsync,
      isPending: false,
    }),
  };
});

beforeEach(() => {
  mockMutateAsync.mockClear();
});

// Helper: wrap with required providers
function renderCard(status: ProviderAuthStatus) {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });

  return render(
    <QueryClientProvider client={queryClient}>
      <RadioGroup value="">
        <ProviderCard provider={status.provider as "anthropic" | "openai" | "gemini" | "ollama"} status={status} />
      </RadioGroup>
    </QueryClientProvider>,
  );
}

// Helper: make an unauthenticated status for a given provider
function makeUnauthStatus(provider: string): ProviderAuthStatus {
  return {
    provider,
    authenticated: false,
    method: "none",
    displayName: null,
    model: null,
    isActive: false,
  };
}

// Helper: expand the card by clicking the header row or chevron
function expandCard() {
  // The collapsed row acts as a button (role="button")
  const header = screen.getByRole("button", { name: /expand/i });
  fireEvent.click(header);
}

describe("ProviderCard (AIPV-05 radio-disabled invariant)", () => {
  it("disables radio when status.authenticated is false", () => {
    const unauthenticatedStatus: ProviderAuthStatus = {
      provider: "openai",
      authenticated: false,
      method: "none",
      displayName: null,
      model: null,
      isActive: false,
    };

    renderCard(unauthenticatedStatus);

    const radio = screen.getByRole("radio");

    // Radix UI sets data-disabled="true" on the root button element when disabled
    const isDisabled =
      radio.hasAttribute("disabled") ||
      radio.getAttribute("data-disabled") === "true" ||
      radio.getAttribute("aria-disabled") === "true";

    expect(isDisabled).toBe(true);
  });

  it("enables radio when status.authenticated is true", () => {
    const authenticatedStatus: ProviderAuthStatus = {
      provider: "anthropic",
      authenticated: true,
      method: "oauth",
      displayName: "Claude",
      model: "claude-haiku-4-5-20251001",
      isActive: true,
    };

    renderCard(authenticatedStatus);

    const radio = screen.getByRole("radio");

    // Radio must NOT have disabled or data-disabled set
    const isDisabled =
      radio.hasAttribute("disabled") ||
      radio.getAttribute("data-disabled") === "true" ||
      radio.getAttribute("aria-disabled") === "true";

    expect(isDisabled).toBe(false);
  });
});

describe("ProviderCard — two-mode OpenAI card (D-25, AIPV-02)", () => {
  it("renders 'Sign in with ChatGPT' as primary CTA for OpenAI when not connected", () => {
    renderCard(makeUnauthStatus("openai"));

    // Expand the card
    expandCard();

    // Primary OAuth CTA must be present
    expect(
      screen.getByRole("button", { name: /sign in with chatgpt/i }),
    ).toBeTruthy();
  });

  it("shows 'Use API key instead' toggle in OAuth mode for OpenAI when not connected", () => {
    renderCard(makeUnauthStatus("openai"));
    expandCard();

    // Secondary toggle must be present
    expect(screen.getByText(/use api key instead/i)).toBeTruthy();
  });

  it("switches to api-key mode when 'Use API key instead' is clicked", () => {
    renderCard(makeUnauthStatus("openai"));
    expandCard();

    // Click the secondary toggle
    const toggle = screen.getByText(/use api key instead/i);
    fireEvent.click(toggle);

    // Now API key input must be visible (Plan 05 form)
    expect(screen.getByLabelText(/api key/i)).toBeTruthy();

    // And the back-to-OAuth toggle must be visible
    expect(screen.getByText(/sign in with chatgpt instead/i)).toBeTruthy();
  });

  it("invokes start_openai_oauth mutation on primary CTA click", async () => {
    // Uses the top-level vi.mock for useStartOpenAiOAuth (hoisted above all tests).
    renderCard(makeUnauthStatus("openai"));

    // Expand
    expandCard();

    // Click "Sign in with ChatGPT"
    const ctaButton = screen.getByRole("button", { name: /sign in with chatgpt/i });
    fireEvent.click(ctaButton);

    // mutateAsync should have been called (top-level mock)
    expect(mockMutateAsync).toHaveBeenCalled();
  });
});

describe("ProviderCard — Anthropic and Ollama regression guard", () => {
  it("Anthropic card has no 'Sign in with' CTA", () => {
    renderCard(makeUnauthStatus("anthropic"));
    expandCard();

    // No "Sign in with" text should appear on Anthropic card
    const signInEl = screen.queryByText(/sign in with/i);
    expect(signInEl).toBeNull();
  });

  it("Ollama card has no 'Sign in with' CTA", () => {
    renderCard(makeUnauthStatus("ollama"));
    expandCard();

    // No "Sign in with" text should appear on Ollama card
    const signInEl = screen.queryByText(/sign in with/i);
    expect(signInEl).toBeNull();
  });
});

describe("ProviderCard — Gemini card (Option C: API-key-only)", () => {
  it("Gemini card has no 'Sign in with Google' CTA (Option C confirmed)", () => {
    renderCard(makeUnauthStatus("gemini"));
    expandCard();

    // Gemini stays API-key-only per Option C
    const signInEl = screen.queryByText(/sign in with google/i);
    expect(signInEl).toBeNull();

    // But the API key form is present
    expect(screen.getByLabelText(/api key/i)).toBeTruthy();
  });
});
