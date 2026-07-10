import { useNavigate } from "react-router-dom";
import {
  ContextMenu,
  ContextMenuTrigger,
  ContextMenuContent,
  ContextMenuItem,
} from "@/components/ui/context-menu";
import { openPath, revealItemInDir } from "@tauri-apps/plugin-opener";
import { FileText, ExternalLink, FolderOpen } from "lucide-react";
import { toast } from "sonner";
import { isTauri } from "@/lib/tauri";
import type { Document } from "@/lib/types";

/**
 * Returns the OS-appropriate label for "show file location" action.
 * macOS: "Reveal in Finder" | others: "Show in file manager"
 */
function revealLabel(): string {
  if (typeof navigator !== "undefined" && /Mac/i.test(navigator.userAgent)) {
    return "Reveal in Finder";
  }
  return "Show in file manager";
}

/**
 * DocumentContextMenu wraps any document row trigger and provides right-click
 * context menu with Open / Open in default app / Reveal in Finder actions.
 *
 * UX-06 / D-17 / D-18: Native file actions via @tauri-apps/plugin-opener.
 * Browser dev mode: handlers guard with isTauri() and return early.
 */
export function DocumentContextMenu({
  doc,
  children,
}: {
  doc: Document;
  children: React.ReactNode;
}) {
  const navigate = useNavigate();

  const handleOpen = () => {
    navigate(`/document/${doc.id}`);
  };

  const handleOpenExternal = async () => {
    if (!isTauri()) return;
    try {
      await openPath(doc.path);
    } catch {
      toast.error("Could not open file. Open it manually from the file manager.");
    }
  };

  const handleReveal = async () => {
    if (!isTauri()) return;
    try {
      await revealItemInDir(doc.path);
    } catch {
      toast.error("Could not reveal file in Finder.");
    }
  };

  return (
    <ContextMenu>
      <ContextMenuTrigger asChild>{children}</ContextMenuTrigger>
      <ContextMenuContent>
        <ContextMenuItem onClick={handleOpen}>
          <FileText className="mr-2 h-4 w-4" /> Open
        </ContextMenuItem>
        <ContextMenuItem onClick={handleOpenExternal}>
          <ExternalLink className="mr-2 h-4 w-4" /> Open in default app
        </ContextMenuItem>
        <ContextMenuItem onClick={handleReveal}>
          <FolderOpen className="mr-2 h-4 w-4" /> {revealLabel()}
        </ContextMenuItem>
      </ContextMenuContent>
    </ContextMenu>
  );
}
