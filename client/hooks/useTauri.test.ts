/**
 * Phase 8 hook tests for extraction settings IPC hooks.
 *
 * RED: Fails until useTauri.ts exports useExtractionSettings,
 * useUpdateExtractionSettings, useTriggerEntityBackfill, and adds
 * queryKeys.extractionSettings.
 *
 * Tests are browser-mode only (isTauri()=false) — tauriInvoke falls back
 * to the provided mock function, so no real Tauri runtime needed.
 */
import { describe, it, expect, beforeEach, vi } from "vitest";
import { renderHook, waitFor, act } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { createElement } from "react";

// Mock @/lib/tauri so tests run without Tauri desktop shell (matches existing test pattern)
vi.mock("@/lib/tauri", () => ({
  isTauri: vi.fn(() => false),
  tauriInvoke: vi.fn((_cmd: string, _args: unknown, fallback?: () => unknown) => {
    if (fallback) return Promise.resolve(fallback());
    return Promise.resolve(null);
  }),
}));

import {
  queryKeys,
  useExtractionSettings,
  useUpdateExtractionSettings,
  useTriggerEntityBackfill,
  useTopics,
} from "./useTauri";

describe("Phase 8: queryKeys extension", () => {
  it("queryKeys.extractionSettings is defined", () => {
    expect(queryKeys.extractionSettings).toBeDefined();
  });

  it("queryKeys.extractionSettings equals ['extraction-settings']", () => {
    expect(queryKeys.extractionSettings).toEqual(["extraction-settings"]);
  });
});

describe("Phase 8: useExtractionSettings hook", () => {
  let queryClient: QueryClient;

  beforeEach(() => {
    queryClient = new QueryClient({
      defaultOptions: { queries: { retry: false } },
    });
  });

  const wrapper = ({ children }: { children: React.ReactNode }) =>
    createElement(QueryClientProvider, { client: queryClient }, children);

  it("is exported from useTauri", () => {
    expect(typeof useExtractionSettings).toBe("function");
  });

  it("returns mock data in browser mode (isTauri()=false)", async () => {
    const { result } = renderHook(() => useExtractionSettings(), { wrapper });
    await waitFor(() => expect(result.current.isLoading).toBe(false));
    expect(result.current.data).toBeDefined();
  });

  it("browser mode data has extractionModel string field", async () => {
    const { result } = renderHook(() => useExtractionSettings(), { wrapper });
    await waitFor(() => expect(result.current.data).toBeDefined());
    expect(typeof result.current.data?.extractionModel).toBe("string");
  });

  it("browser mode data has useLlmExtraction boolean field", async () => {
    const { result } = renderHook(() => useExtractionSettings(), { wrapper });
    await waitFor(() => expect(result.current.data).toBeDefined());
    expect(typeof result.current.data?.useLlmExtraction).toBe("boolean");
  });

  it("browser mode extractionModel defaults to empty string", async () => {
    const { result } = renderHook(() => useExtractionSettings(), { wrapper });
    await waitFor(() => expect(result.current.data).toBeDefined());
    expect(result.current.data?.extractionModel).toBe("");
  });

  it("browser mode useLlmExtraction defaults to true", async () => {
    const { result } = renderHook(() => useExtractionSettings(), { wrapper });
    await waitFor(() => expect(result.current.data).toBeDefined());
    expect(result.current.data?.useLlmExtraction).toBe(true);
  });
});

describe("Phase 8: useUpdateExtractionSettings hook", () => {
  let queryClient: QueryClient;

  beforeEach(() => {
    queryClient = new QueryClient({
      defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
    });
  });

  const wrapper = ({ children }: { children: React.ReactNode }) =>
    createElement(QueryClientProvider, { client: queryClient }, children);

  it("is exported from useTauri", () => {
    expect(typeof useUpdateExtractionSettings).toBe("function");
  });

  it("returns a mutation with mutate function", () => {
    const { result } = renderHook(() => useUpdateExtractionSettings(), { wrapper });
    expect(typeof result.current.mutate).toBe("function");
  });

  it("mutation succeeds in browser mode (no-op)", async () => {
    const { result } = renderHook(() => useUpdateExtractionSettings(), { wrapper });
    await act(async () => {
      result.current.mutate({ extractionModel: "gpt-5-mini", useLlmExtraction: true });
    });
    await waitFor(() => expect(result.current.isIdle).toBe(false));
  });
});

describe("Phase 8: useTriggerEntityBackfill hook", () => {
  let queryClient: QueryClient;

  beforeEach(() => {
    queryClient = new QueryClient({
      defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
    });
  });

  const wrapper = ({ children }: { children: React.ReactNode }) =>
    createElement(QueryClientProvider, { client: queryClient }, children);

  it("is exported from useTauri", () => {
    expect(typeof useTriggerEntityBackfill).toBe("function");
  });

  it("returns a mutation with mutate function", () => {
    const { result } = renderHook(() => useTriggerEntityBackfill(), { wrapper });
    expect(typeof result.current.mutate).toBe("function");
  });

  it("mutation is a no-op in browser mode (isTauri()=false)", async () => {
    const { result } = renderHook(() => useTriggerEntityBackfill(), { wrapper });
    await act(async () => {
      result.current.mutate();
    });
    // Should not error in browser mode
    await waitFor(() => expect(result.current.isIdle).toBe(false));
    // No error expected
    expect(result.current.error).toBeNull();
  });
});

// ──────────────────────────────────────────────────────────────────────────────
// Phase 8 Plan 09: useTopics hook + queryKeys.topics
// ──────────────────────────────────────────────────────────────────────────────

describe("Phase 8 Plan 09: queryKeys.topics", () => {
  it("queryKeys.topics is defined", () => {
    expect(queryKeys.topics).toBeDefined();
  });

  it("queryKeys.topics equals ['topics']", () => {
    expect(queryKeys.topics).toEqual(["topics"]);
  });
});

describe("Phase 8 Plan 09: useTopics hook", () => {
  let queryClient: QueryClient;

  beforeEach(() => {
    queryClient = new QueryClient({
      defaultOptions: { queries: { retry: false } },
    });
  });

  const wrapper = ({ children }: { children: React.ReactNode }) =>
    createElement(QueryClientProvider, { client: queryClient }, children);

  it("is exported from useTauri", () => {
    expect(typeof useTopics).toBe("function");
  });

  it("returns data in browser mode (isTauri()=false)", async () => {
    const { result } = renderHook(() => useTopics(), { wrapper });
    await waitFor(() => expect(result.current.isLoading).toBe(false));
    expect(result.current.data).toBeDefined();
  });

  it("browser mode data is an array", async () => {
    const { result } = renderHook(() => useTopics(), { wrapper });
    await waitFor(() => expect(result.current.data).toBeDefined());
    expect(Array.isArray(result.current.data)).toBe(true);
  });

  it("browser mode data has at least 5 topics (mockTopics coverage)", async () => {
    const { result } = renderHook(() => useTopics(), { wrapper });
    await waitFor(() => expect(result.current.data).toBeDefined());
    expect((result.current.data ?? []).length).toBeGreaterThanOrEqual(5);
  });

  it("each topic entry has topic (string) and count (number)", async () => {
    const { result } = renderHook(() => useTopics(), { wrapper });
    await waitFor(() => expect(result.current.data).toBeDefined());
    const topics = result.current.data ?? [];
    for (const t of topics) {
      expect(typeof t.topic).toBe("string");
      expect(typeof t.count).toBe("number");
    }
  });
});

describe("Phase 8: existing hook regressions", () => {
  let queryClient: QueryClient;

  beforeEach(() => {
    queryClient = new QueryClient({
      defaultOptions: { queries: { retry: false } },
    });
  });

  const wrapper = ({ children }: { children: React.ReactNode }) =>
    createElement(QueryClientProvider, { client: queryClient }, children);

  it("queryKeys.settings still exists (regression guard)", () => {
    expect(queryKeys.settings).toEqual(["settings"]);
  });

  it("queryKeys.providers still exists (regression guard)", () => {
    expect(queryKeys.providers).toEqual(["providers"]);
  });
});
