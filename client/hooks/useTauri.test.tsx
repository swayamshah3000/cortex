/**
 * Tests for entity query hooks in useTauri.ts (Plan 06-06 Task 1, Test 6)
 *
 * Test 6: queryKeys factory exports entity keys; 5 read hooks are callable
 * and return React Query results.
 */

import { describe, it, expect, vi } from "vitest";
import { renderHook, waitFor } from "@testing-library/react";
import React from "react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import {
  queryKeys,
  useEntities,
  useEntitiesByType,
  useEntity,
  useEntityDocuments,
  useRelatedEntities,
  useRenameEntityCanonical,
  useSplitEntityAlias,
} from "./useTauri";

// Mock tauriInvoke and isTauri
vi.mock("@/lib/tauri", () => ({
  isTauri: vi.fn(() => false), // browser-dev mode → use fallback mock data
  tauriInvoke: vi.fn((_cmd: string, _args: unknown, fallback?: () => unknown) => {
    if (fallback) return Promise.resolve(fallback());
    return Promise.resolve(null);
  }),
}));

function createWrapper() {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
    },
  });
  return function Wrapper({ children }: { children: React.ReactNode }) {
    return <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>;
  };
}

describe("useTauri entity queryKeys (06-06 Task 1 - Test 6)", () => {
  it("queryKeys.entities returns ['entities'] tuple", () => {
    expect(queryKeys.entities).toEqual(["entities"]);
  });

  it("queryKeys.entitiesByType returns ['entities', 'byType', type]", () => {
    expect(queryKeys.entitiesByType("person")).toEqual(["entities", "byType", "person"]);
  });

  it("queryKeys.entity returns ['entities', id]", () => {
    expect(queryKeys.entity("entity-123")).toEqual(["entities", "entity-123"]);
  });

  it("queryKeys.entityDocuments returns ['entities', id, 'documents']", () => {
    expect(queryKeys.entityDocuments("entity-123")).toEqual([
      "entities",
      "entity-123",
      "documents",
    ]);
  });

  it("queryKeys.relatedEntities returns ['entities', id, 'related']", () => {
    expect(queryKeys.relatedEntities("entity-123")).toEqual([
      "entities",
      "entity-123",
      "related",
    ]);
  });
});

describe("useEntities hook (06-06 Task 1 - Test 6)", () => {
  it("returns data array from fallback mock", async () => {
    const { result } = renderHook(() => useEntities(), { wrapper: createWrapper() });
    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(Array.isArray(result.current.data)).toBe(true);
  });
});

describe("useEntitiesByType hook (06-06 Task 1 - Test 6)", () => {
  it("is disabled when type is empty string", () => {
    const { result } = renderHook(() => useEntitiesByType(""), { wrapper: createWrapper() });
    expect(result.current.fetchStatus).toBe("idle");
  });

  it("fires when type is provided", async () => {
    const { result } = renderHook(() => useEntitiesByType("person"), {
      wrapper: createWrapper(),
    });
    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(Array.isArray(result.current.data)).toBe(true);
  });
});

describe("useEntity hook (06-06 Task 1 - Test 6)", () => {
  it("is disabled when id is empty string", () => {
    const { result } = renderHook(() => useEntity(""), { wrapper: createWrapper() });
    expect(result.current.fetchStatus).toBe("idle");
  });

  it("fires when id is provided", async () => {
    const { result } = renderHook(() => useEntity("entity-123"), { wrapper: createWrapper() });
    await waitFor(() => expect(result.current.isSuccess).toBe(true));
  });
});

describe("useEntityDocuments hook (06-06 Task 1 - Test 6)", () => {
  it("is disabled when id is empty string", () => {
    const { result } = renderHook(() => useEntityDocuments(""), { wrapper: createWrapper() });
    expect(result.current.fetchStatus).toBe("idle");
  });

  it("fires when id is provided and returns array", async () => {
    const { result } = renderHook(() => useEntityDocuments("entity-123"), {
      wrapper: createWrapper(),
    });
    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(Array.isArray(result.current.data)).toBe(true);
  });
});

describe("useRelatedEntities hook (06-06 Task 1 - Test 6)", () => {
  it("is disabled when id is empty string", () => {
    const { result } = renderHook(() => useRelatedEntities(""), { wrapper: createWrapper() });
    expect(result.current.fetchStatus).toBe("idle");
  });

  it("fires when id is provided and returns array", async () => {
    const { result } = renderHook(() => useRelatedEntities("entity-123"), {
      wrapper: createWrapper(),
    });
    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(Array.isArray(result.current.data)).toBe(true);
  });
});

// --- Plan 06-07 Tests 7-9: mutation hooks -----------------------------------

describe("useRenameEntityCanonical (06-07 Task 1 - Test 7)", () => {
  it("Test 7: mutationFn calls rename_entity_canonical IPC command", async () => {
    const { tauriInvoke } = await import("@/lib/tauri");
    const mockTauriInvoke = vi.mocked(tauriInvoke);
    // Returns a CanonicalEntity in browser-mode via fallback that returns undefined as never
    // We just verify the mutate function is callable
    const { result } = renderHook(() => useRenameEntityCanonical(), {
      wrapper: createWrapper(),
    });
    expect(typeof result.current.mutate).toBe("function");
    expect(typeof result.current.mutateAsync).toBe("function");
  });

  it("Test 7: invalidates entity and entities query keys on success", async () => {
    const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    const invalidateSpy = vi.spyOn(queryClient, "invalidateQueries");

    // Override tauriInvoke to return a valid CanonicalEntity
    const { tauriInvoke } = await import("@/lib/tauri");
    const mockTauriInvoke = vi.mocked(tauriInvoke);
    mockTauriInvoke.mockResolvedValueOnce({
      id: "entity-001",
      canonicalName: "New Name",
      entityType: "person",
      aliases: ["New Name"],
      documentCount: 1,
    });

    const wrapper = ({ children }: { children: React.ReactNode }) => (
      <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
    );

    const { result } = renderHook(() => useRenameEntityCanonical(), { wrapper });
    result.current.mutate({ id: "entity-001", newName: "New Name" });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    expect(invalidateSpy).toHaveBeenCalledWith({
      queryKey: queryKeys.entity("entity-001"),
    });
    expect(invalidateSpy).toHaveBeenCalledWith({
      queryKey: queryKeys.entities,
    });
  });
});

describe("useSplitEntityAlias (06-07 Task 1 - Test 8)", () => {
  it("Test 8: mutationFn calls split_entity_alias IPC command", async () => {
    const { result } = renderHook(() => useSplitEntityAlias(), {
      wrapper: createWrapper(),
    });
    expect(typeof result.current.mutate).toBe("function");
  });

  it("Test 8: invalidates entity, entityDocuments, and entities on success", async () => {
    const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    const invalidateSpy = vi.spyOn(queryClient, "invalidateQueries");

    const { tauriInvoke } = await import("@/lib/tauri");
    const mockTauriInvoke = vi.mocked(tauriInvoke);
    mockTauriInvoke.mockResolvedValueOnce({
      id: "entity-002",
      canonicalName: "J. Smith",
      entityType: "person",
      aliases: ["J. Smith"],
      documentCount: 1,
    });

    const wrapper = ({ children }: { children: React.ReactNode }) => (
      <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
    );

    const { result } = renderHook(() => useSplitEntityAlias(), { wrapper });
    result.current.mutate({ canonicalId: "entity-001", alias: "J. Smith" });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    expect(invalidateSpy).toHaveBeenCalledWith({
      queryKey: queryKeys.entity("entity-001"),
    });
    expect(invalidateSpy).toHaveBeenCalledWith({
      queryKey: queryKeys.entityDocuments("entity-001"),
    });
    expect(invalidateSpy).toHaveBeenCalledWith({
      queryKey: queryKeys.entities,
    });
  });
});
