/**
 * Tests for WatchedPage native folder picker (UX-05 / D-19).
 *
 * These tests verify:
 * 1. Happy path: isTauri + open returns path + exists + isDirectory → addFolder called
 * 2. User cancel: open returns null → addFolder NOT called, no error toast
 * 3. open throws → error toast shown
 * 4. Browser dev fallback: isTauri false → button disabled with tooltip
 * 5. D-19 path no longer exists: exists returns false → addFolder NOT called, toast error
 * 6. D-19 path is a file: stat returns { isDirectory: false } → addFolder NOT called, toast error
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import React from "react";

// Use vi.hoisted so these vars are initialized before vi.mock factories run
const {
  mockOpen,
  mockExists,
  mockStat,
  mockIsTauri,
  mockToastError,
  mockAddFolderMutate,
} = vi.hoisted(() => ({
  mockOpen: vi.fn(),
  mockExists: vi.fn(),
  mockStat: vi.fn(),
  mockIsTauri: vi.fn(() => true),
  mockToastError: vi.fn(),
  mockAddFolderMutate: vi.fn(),
}));

vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: mockOpen,
}));

vi.mock("@tauri-apps/plugin-fs", () => ({
  exists: mockExists,
  stat: mockStat,
}));

vi.mock("../lib/tauri", () => ({
  isTauri: mockIsTauri,
}));

vi.mock("sonner", () => ({
  toast: {
    error: mockToastError,
  },
  Toaster: () => null,
}));

vi.mock("../hooks/useTauri", () => ({
  useWatchedFolders: () => ({
    data: [
      {
        id: "folder-1",
        path: "/Users/test/Documents",
        documentCount: 10,
        lastScan: new Date().toISOString(),
        status: "watching",
      },
    ],
    isLoading: false,
  }),
  useAddWatchedFolder: () => ({
    mutate: mockAddFolderMutate,
    isPending: false,
  }),
  useRemoveWatchedFolder: () => ({ mutate: vi.fn() }),
  useTriggerScan: () => ({ mutate: vi.fn() }),
}));

vi.mock("date-fns", async (importOriginal) => {
  const actual = await importOriginal<typeof import("date-fns")>();
  return {
    ...actual,
    formatDistanceToNow: () => "2 minutes ago",
  };
});

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

// Import the component under test AFTER mocks are set up
import WatchedPage from "./WatchedPage";
import { MemoryRouter } from "react-router-dom";

function renderWatchedPage() {
  return render(
    <MemoryRouter>
      <WatchedPage />
    </MemoryRouter>,
  );
}

describe("WatchedPage — native folder picker (UX-05 / D-19)", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    // Reset default after clearAllMocks
    mockIsTauri.mockReturnValue(true);
  });

  it("Test 1: happy path — open returns path, exists+isDirectory pass → addFolder called", async () => {
    mockIsTauri.mockReturnValue(true);
    mockOpen.mockResolvedValue("/Users/test/NewFolder");
    mockExists.mockResolvedValue(true);
    mockStat.mockResolvedValue({ isDirectory: true });

    renderWatchedPage();

    const btn = screen.getByRole("button", { name: /add folder/i });
    fireEvent.click(btn);

    await waitFor(() => {
      expect(mockAddFolderMutate).toHaveBeenCalledWith("/Users/test/NewFolder");
    });
    expect(mockToastError).not.toHaveBeenCalled();
  });

  it("Test 2: user cancel — open returns null → addFolder NOT called, no error toast", async () => {
    mockIsTauri.mockReturnValue(true);
    mockOpen.mockResolvedValue(null);

    renderWatchedPage();

    const btn = screen.getByRole("button", { name: /add folder/i });
    fireEvent.click(btn);

    await waitFor(() => {
      expect(mockOpen).toHaveBeenCalled();
    });
    // Give time for any async ops to settle
    await new Promise((r) => setTimeout(r, 50));
    expect(mockAddFolderMutate).not.toHaveBeenCalled();
    expect(mockToastError).not.toHaveBeenCalled();
  });

  it("Test 3: open throws → error toast shown", async () => {
    mockIsTauri.mockReturnValue(true);
    mockOpen.mockRejectedValue(new Error("Permission denied"));

    renderWatchedPage();

    const btn = screen.getByRole("button", { name: /add folder/i });
    fireEvent.click(btn);

    await waitFor(() => {
      expect(mockToastError).toHaveBeenCalledWith(
        "That folder could not be added. It may not exist or be inaccessible.",
      );
    });
    expect(mockAddFolderMutate).not.toHaveBeenCalled();
  });

  it("Test 4: browser dev fallback — isTauri false → button is disabled", () => {
    mockIsTauri.mockReturnValue(false);

    renderWatchedPage();

    const btn = screen.getByRole("button", { name: /add folder/i });
    expect(btn).toBeDisabled();
  });

  it("Test 5: D-19 — path no longer exists → addFolder NOT called, toast error shown", async () => {
    mockIsTauri.mockReturnValue(true);
    mockOpen.mockResolvedValue("/Users/test/DeletedFolder");
    mockExists.mockResolvedValue(false);

    renderWatchedPage();

    const btn = screen.getByRole("button", { name: /add folder/i });
    fireEvent.click(btn);

    await waitFor(() => {
      expect(mockToastError).toHaveBeenCalledWith("Selected path is no longer a directory");
    });
    expect(mockAddFolderMutate).not.toHaveBeenCalled();
  });

  it("Test 6: D-19 — path exists but is a file → addFolder NOT called, toast error shown", async () => {
    mockIsTauri.mockReturnValue(true);
    mockOpen.mockResolvedValue("/Users/test/some-file.txt");
    mockExists.mockResolvedValue(true);
    mockStat.mockResolvedValue({ isDirectory: false });

    renderWatchedPage();

    const btn = screen.getByRole("button", { name: /add folder/i });
    fireEvent.click(btn);

    await waitFor(() => {
      expect(mockToastError).toHaveBeenCalledWith("Selected path is no longer a directory");
    });
    expect(mockAddFolderMutate).not.toHaveBeenCalled();
  });
});
