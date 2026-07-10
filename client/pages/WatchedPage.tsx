import { useState, useEffect, useCallback, useRef } from "react";
import {
  FolderOpen,
  Plus,
  Play,
  Pause,
  Trash2,
  RefreshCw,
  Loader2,
} from "lucide-react";
import { safeDistance } from "../lib/utils";
import {
  useWatchedFolders,
  useAddWatchedFolder,
  useRemoveWatchedFolder,
  useTriggerScan,
} from "../hooks/useTauri";
import { isTauri } from "../lib/tauri";
import { toast } from "sonner";
import { open } from "@tauri-apps/plugin-dialog";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "../components/ui/tooltip";
import type { WatchedFolder } from "../lib/types";

const statusConfig: Record<string, { label: string; className: string }> = {
  watching: { label: "Watching", className: "bg-green-500/10 text-green-400" },
  paused: { label: "Paused", className: "bg-yellow-500/10 text-yellow-400" },
  error: { label: "Error", className: "bg-red-500/10 text-red-400" },
};

interface ScanningState {
  [folderId: string]: boolean;
}

interface PerFolderProgress {
  processed: number;
  currentFile: string | null;
  status: "idle" | "indexing" | "complete";
}

export default function WatchedPage() {
  const { data: folders, isLoading } = useWatchedFolders();
  const { mutate: addFolder, isPending: isAdding } = useAddWatchedFolder();
  const { mutate: removeFolder } = useRemoveWatchedFolder();
  const { mutate: triggerScan } = useTriggerScan();

  const [confirmRemoveId, setConfirmRemoveId] = useState<string | null>(null);
  const [scanning, setScanning] = useState<ScanningState>({});
  const [progress, setProgress] = useState<Record<string, PerFolderProgress>>({});
  const idleTimersRef = useRef<Record<string, ReturnType<typeof setTimeout>>>({});

  // Listen for Tauri index-progress events + track per-folder live progress
  useEffect(() => {
    if (!isTauri()) return;
    let unlisten: (() => void) | undefined;

    (async () => {
      try {
        const { listen } = await import("@tauri-apps/api/event");
        unlisten = await listen<{
          folderId: string | null;
          status: string;
          filePath: string;
        }>("index-progress", (event) => {
          const { folderId, status, filePath } = event.payload;
          if (!folderId) return;

          // Reset the per-folder idle timer whenever a fresh event arrives.
          // File-watcher path never emits "complete", so we auto-stop the
          // animation after 3s of silence.
          const clearIdleTimer = () => {
            const t = idleTimersRef.current[folderId];
            if (t) {
              clearTimeout(t);
              delete idleTimersRef.current[folderId];
            }
          };
          const armIdleTimer = () => {
            clearIdleTimer();
            idleTimersRef.current[folderId] = setTimeout(() => {
              setProgress((prev) => {
                const p = prev[folderId];
                if (!p || p.status !== "indexing") return prev;
                return {
                  ...prev,
                  [folderId]: { ...p, currentFile: null, status: "complete" },
                };
              });
              delete idleTimersRef.current[folderId];
            }, 3000);
          };

          if (status === "indexing") {
            setProgress((prev) => ({
              ...prev,
              [folderId]: {
                processed: prev[folderId]?.processed ?? 0,
                currentFile: filePath,
                status: "indexing",
              },
            }));
            armIdleTimer();
          } else if (status === "indexed" || status === "skipped") {
            setProgress((prev) => ({
              ...prev,
              [folderId]: {
                processed: (prev[folderId]?.processed ?? 0) + 1,
                currentFile: filePath,
                status: "indexing",
              },
            }));
            armIdleTimer();
          } else if (status === "complete") {
            clearIdleTimer();
            setScanning((prev) => ({ ...prev, [folderId]: false }));
            setProgress((prev) => ({
              ...prev,
              [folderId]: {
                processed: prev[folderId]?.processed ?? 0,
                currentFile: null,
                status: "complete",
              },
            }));
          } else if (status === "error") {
            clearIdleTimer();
            setScanning((prev) => ({ ...prev, [folderId]: false }));
          }
        });
      } catch {
        // Not in Tauri environment
      }
    })();

    return () => {
      unlisten?.();
    };
  }, []);

  const handleAddFolder = useCallback(async () => {
    if (!isTauri()) return; // Browser dev — button is disabled per UI-SPEC
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: "Add Watched Folder",
      });
      if (!selected || typeof selected !== "string") {
        // null / array = user cancelled — do nothing silently per D-19
        return;
      }
      // D-19: validate the returned path exists and is a directory before submitting
      // to addWatchedFolder. The user could have deleted the folder between the picker
      // opening and confirming, or selected a stale alias. Validate client-side via
      // @tauri-apps/plugin-fs before calling the backend IPC.
      const { exists, stat } = await import("@tauri-apps/plugin-fs");
      const present = await exists(selected);
      if (!present) {
        toast.error("Selected path is no longer a directory");
        return;
      }
      const meta = await stat(selected);
      if (!meta.isDirectory) {
        toast.error("Selected path is no longer a directory");
        return;
      }
      addFolder(selected);
    } catch {
      toast.error("That folder could not be added. It may not exist or be inaccessible.");
    }
  }, [addFolder]);

  const handleRemove = useCallback(
    (id: string) => {
      removeFolder(id, {
        onSuccess: () => setConfirmRemoveId(null),
      });
    },
    [removeFolder],
  );

  const handleScan = useCallback(
    (folderId: string) => {
      setScanning((prev) => ({ ...prev, [folderId]: true }));
      triggerScan(folderId, {
        onSuccess: () => {
          setScanning((prev) => ({ ...prev, [folderId]: false }));
        },
        onError: () => {
          setScanning((prev) => ({ ...prev, [folderId]: false }));
        },
      });
    },
    [triggerScan],
  );

  function AddFolderButton({ className }: { className?: string }) {
    const disabled = !isTauri();
    const btn = (
      <button
        type="button"
        onClick={handleAddFolder}
        disabled={disabled || isAdding}
        className={
          className ??
          "inline-flex items-center gap-2 px-4 py-2 bg-accent-primary text-white rounded-lg hover:bg-accent-hover transition-colors text-sm font-medium disabled:opacity-50 disabled:cursor-not-allowed"
        }
      >
        <Plus size={16} />
        Add Folder
      </button>
    );
    if (disabled) {
      return (
        <TooltipProvider>
          <Tooltip>
            <TooltipTrigger asChild>{btn}</TooltipTrigger>
            <TooltipContent>
              <p>Folder picking requires the desktop app.</p>
            </TooltipContent>
          </Tooltip>
        </TooltipProvider>
      );
    }
    return btn;
  }

  if (isLoading) {
    return (
      <div className="space-y-6">
        <div className="space-y-2">
          <h1 className="page-title text-text-primary">Watched Folders</h1>
          <p className="text-text-secondary">Loading folders...</p>
        </div>
        <div className="space-y-4">
          {Array.from({ length: 3 }).map((_, i) => (
            <div key={i} className="card p-6 animate-pulse">
              <div className="flex items-center gap-4">
                <div className="w-10 h-10 rounded-lg bg-bg-tertiary" />
                <div className="flex-1 space-y-2">
                  <div className="h-4 w-64 rounded bg-bg-tertiary" />
                  <div className="h-3 w-40 rounded bg-bg-tertiary" />
                </div>
              </div>
            </div>
          ))}
        </div>
      </div>
    );
  }

  if (!folders || folders.length === 0) {
    return (
      <div className="space-y-6">
        <div className="flex items-center justify-between">
          <div className="space-y-2">
            <h1 className="page-title text-text-primary">Watched Folders</h1>
            <p className="text-text-secondary">Configure which folders Cortex monitors</p>
          </div>
        </div>
        <div className="flex items-center justify-center min-h-[50vh]">
          <div className="text-center space-y-4">
            <div className="mx-auto w-16 h-16 rounded-full bg-bg-tertiary flex items-center justify-center">
              <FolderOpen size={32} className="text-text-tertiary" />
            </div>
            <h2 className="text-xl font-semibold text-text-primary">No folders being watched</h2>
            <p className="text-text-secondary max-w-sm">
              Add a folder to start discovering and organizing your documents.
            </p>
            <AddFolderButton className="inline-flex items-center gap-2 mt-2 px-4 py-2 bg-accent-primary text-white rounded-lg hover:bg-accent-hover transition-colors text-sm font-medium disabled:opacity-50 disabled:cursor-not-allowed" />
          </div>
        </div>
      </div>
    );
  }

  function renderConfirmDialog(folder: WatchedFolder) {
    if (confirmRemoveId !== folder.id) return null;
    return (
      <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
        <div className="card p-6 w-full max-w-sm space-y-4">
          <h3 className="text-lg font-semibold text-text-primary">Remove Folder</h3>
          <p className="text-sm text-text-secondary">
            Stop watching <span className="font-mono text-text-primary">{folder.path}</span>?
            Documents already indexed will remain in your library.
          </p>
          <div className="flex justify-end gap-2">
            <button
              type="button"
              onClick={() => setConfirmRemoveId(null)}
              className="px-3 py-1.5 text-sm text-text-secondary hover:text-text-primary transition-colors"
            >
              Cancel
            </button>
            <button
              type="button"
              onClick={() => handleRemove(folder.id)}
              className="px-4 py-1.5 bg-red-600 text-white rounded-lg hover:bg-red-700 transition-colors text-sm font-medium"
            >
              Remove
            </button>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div className="space-y-2">
          <h1 className="page-title text-text-primary">Watched Folders</h1>
          <p className="text-text-secondary">
            {folders.length} folder{folders.length !== 1 ? "s" : ""} being monitored
          </p>
        </div>
        <AddFolderButton />
      </div>

      <div className="space-y-4">
        {folders.map((folder) => {
          const status = statusConfig[folder.status] ?? statusConfig.error;
          const live = progress[folder.id];
          // Any active indexing counts as "scanning" for the UI, regardless
          // of whether the user clicked "Scan Now" or the file-watcher fired
          // the event. Merge explicit-scan flag with live-progress state.
          const isScanning =
            (scanning[folder.id] ?? false) || live?.status === "indexing";
          // Prefer live-processed count during a scan so the number ticks up
          // in real time; fall back to registry count when idle.
          const displayCount = live?.status === "indexing"
            ? Math.max(live.processed, folder.documentCount)
            : folder.documentCount;
          const shortCurrent = live?.currentFile
            ? live.currentFile.split("/").slice(-2).join("/")
            : null;

          return (
            <div key={folder.id} className="card p-6">
              <div className="flex items-start gap-4">
                <div className="p-2.5 rounded-lg bg-accent-subtle text-accent-primary flex-shrink-0">
                  <FolderOpen size={22} />
                </div>

                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-3">
                    <p className="font-mono text-sm text-text-primary truncate">
                      {folder.path}
                    </p>
                    <span className={`text-xs px-2 py-0.5 rounded-full flex-shrink-0 ${status.className}`}>
                      {status.label}
                    </span>
                  </div>
                  <div className="flex items-center gap-4 mt-2 text-xs text-text-tertiary">
                    <span>{displayCount.toLocaleString()} documents</span>
                    <span>
                      Last scan:{" "}
                      {safeDistance(folder.lastScan)}
                    </span>
                  </div>

                  {isScanning && (
                    <div className="mt-3 space-y-2">
                      <div className="flex items-center gap-2 text-xs text-accent-primary">
                        <Loader2 size={14} className="animate-spin" />
                        <span>
                          Indexing {live?.processed.toLocaleString() ?? 0} files
                          {shortCurrent ? " — " : ""}
                        </span>
                        {shortCurrent && (
                          <span className="font-mono text-text-tertiary truncate">
                            {shortCurrent}
                          </span>
                        )}
                      </div>
                      {/* Indeterminate progress bar — total unknown until scan completes */}
                      <div className="h-1 w-full bg-bg-tertiary rounded overflow-hidden">
                        <div
                          className="h-full bg-accent-primary/60 animate-pulse rounded"
                          style={{ width: "40%" }}
                        />
                      </div>
                    </div>
                  )}
                </div>

                <div className="flex items-center gap-1 flex-shrink-0">
                  <button
                    type="button"
                    onClick={() => handleScan(folder.id)}
                    disabled={isScanning}
                    className="p-2 rounded-lg hover:bg-bg-tertiary text-text-tertiary hover:text-text-primary transition-colors disabled:opacity-50"
                    title="Scan now"
                  >
                    {isScanning ? (
                      <Loader2 size={16} className="animate-spin" />
                    ) : (
                      <RefreshCw size={16} />
                    )}
                  </button>
                  {folder.status === "watching" ? (
                    <button
                      type="button"
                      className="p-2 rounded-lg hover:bg-bg-tertiary text-text-tertiary hover:text-text-primary transition-colors"
                      title="Pause (coming soon)"
                      disabled
                    >
                      <Pause size={16} />
                    </button>
                  ) : (
                    <button
                      type="button"
                      className="p-2 rounded-lg hover:bg-bg-tertiary text-text-tertiary hover:text-text-primary transition-colors"
                      title="Resume (coming soon)"
                      disabled
                    >
                      <Play size={16} />
                    </button>
                  )}
                  <button
                    type="button"
                    onClick={() => setConfirmRemoveId(folder.id)}
                    className="p-2 rounded-lg hover:bg-red-500/10 text-text-tertiary hover:text-red-400 transition-colors"
                    title="Remove folder"
                  >
                    <Trash2 size={16} />
                  </button>
                </div>
              </div>

              {renderConfirmDialog(folder)}
            </div>
          );
        })}
      </div>
    </div>
  );
}
