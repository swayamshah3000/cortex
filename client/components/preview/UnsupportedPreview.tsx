import { FileQuestion } from "lucide-react";
import { openPath, revealItemInDir } from "@tauri-apps/plugin-opener";
import { toast } from "sonner";
import type { Document } from "@/lib/types";

/**
 * Returns the OS-appropriate label for "show file location" action.
 * macOS: "Reveal in Finder" | others: "Show in file manager"
 */
export function revealLabel(): string {
  if (typeof navigator !== "undefined" && /Mac/i.test(navigator.userAgent)) {
    return "Reveal in Finder";
  }
  return "Show in file manager";
}

interface UnsupportedPreviewProps {
  doc: Document;
}

export function UnsupportedPreview({ doc }: UnsupportedPreviewProps) {
  // Extract file extension from doc.name
  const lastDot = doc.name.lastIndexOf(".");
  const ext = lastDot >= 0 ? doc.name.slice(lastDot).toLowerCase() : "";

  const handleOpen = async () => {
    try {
      await openPath(doc.path);
    } catch {
      toast.error("Could not open file. Open it manually from the file manager.");
    }
  };

  const handleReveal = async () => {
    try {
      await revealItemInDir(doc.path);
    } catch {
      const label = revealLabel();
      toast.error(
        `Could not reveal file in ${label === "Reveal in Finder" ? "Finder" : "file manager"}.`,
      );
    }
  };

  return (
    <div className="h-full w-full flex items-center justify-center bg-bg-primary p-6">
      <div className="card p-8 max-w-md text-center space-y-4">
        <FileQuestion size={32} className="text-text-tertiary mx-auto" />
        <h3 className="section-header text-text-primary">Preview not supported</h3>
        <p className="text-text-secondary text-sm">
          Cortex does not preview {ext} files yet. Open in the default app to view.
        </p>
        <div className="flex flex-col gap-2">
          <button
            type="button"
            onClick={handleOpen}
            className="btn-primary"
          >
            Open in default app
          </button>
          <button
            type="button"
            onClick={handleReveal}
            className="btn-secondary"
          >
            {revealLabel()}
          </button>
        </div>
      </div>
    </div>
  );
}
