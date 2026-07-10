/**
 * Tests for usePreview hook (PAGE-13 / D-16).
 *
 * Test 8: usePreview(docId) dispatches React Query for ["documents", docId, "text"]
 *         and invokes tauriInvoke read_document_text with { docId, maxBytes: 5_242_880 }.
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, waitFor } from "@testing-library/react";
import React from "react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";

// Use vi.hoisted so variables are available in factory closures
const { mockTauriInvoke, mockIsTauri } = vi.hoisted(() => ({
  mockTauriInvoke: vi.fn(),
  mockIsTauri: vi.fn(() => false),
}));

vi.mock("@/lib/tauri", () => ({
  isTauri: mockIsTauri,
  tauriInvoke: mockTauriInvoke,
}));

// Import after mocks
import { usePreview } from "./usePreview";

function createWrapper() {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: {
        retry: false,
      },
    },
  });
  return function Wrapper({ children }: { children: React.ReactNode }) {
    return <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>;
  };
}

describe("Test 8: usePreview hook", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("invokes tauriInvoke with read_document_text command and correct args", async () => {
    const mockResult = { text: "(mock preview text)", truncated: false, size: 100 };
    mockTauriInvoke.mockResolvedValue(mockResult);
    mockIsTauri.mockReturnValue(true);

    const { result } = renderHook(() => usePreview("doc-123"), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true);
    });

    expect(mockTauriInvoke).toHaveBeenCalledWith(
      "read_document_text",
      { docId: "doc-123", maxBytes: 5 * 1024 * 1024 },
      expect.any(Function),
    );
  });

  it("returns data from tauriInvoke", async () => {
    const mockResult = { text: "file content", truncated: false, size: 200 };
    mockTauriInvoke.mockResolvedValue(mockResult);
    mockIsTauri.mockReturnValue(true);

    const { result } = renderHook(() => usePreview("doc-456"), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true);
    });

    expect(result.current.data).toEqual(mockResult);
  });

  it("is disabled (not called) when docId is empty string", async () => {
    const { result } = renderHook(() => usePreview(""), {
      wrapper: createWrapper(),
    });

    // Wait a tick
    await new Promise((r) => setTimeout(r, 50));
    expect(result.current.status).toBe("pending");
    expect(mockTauriInvoke).not.toHaveBeenCalled();
  });

  it("falls back to mock data when isTauri() is false", async () => {
    mockIsTauri.mockReturnValue(false);
    // tauriInvoke calls fallback when !isTauri
    mockTauriInvoke.mockImplementation(async (_cmd: string, _args: unknown, fallback?: () => unknown) => {
      if (fallback) return fallback();
      throw new Error("no fallback");
    });

    const { result } = renderHook(() => usePreview("doc-789"), {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true);
    });
    expect(result.current.data?.text).toContain("mock");
  });
});
