import { useState } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { openPath } from "@tauri-apps/plugin-opener";
import { isTauri } from "@/lib/tauri";
import type { Document } from "@/lib/types";
import { SizeGuardCard } from "./SizeGuardCard";
import { UnsupportedPreview } from "./UnsupportedPreview";

const PDF_SIZE_LIMIT = 50 * 1024 * 1024; // 50 MB

interface PdfPreviewProps {
  doc: Document;
}

export function PdfPreview({ doc }: PdfPreviewProps) {
  const [forceLoad, setForceLoad] = useState(false);

  // Browser-dev fallback: PDF preview requires the desktop app
  if (!isTauri()) {
    return <UnsupportedPreview doc={{ ...doc, name: doc.name }} />;
  }

  const handleOpenExternal = async () => {
    try {
      await openPath(doc.path);
    } catch {
      toast.error("Could not open file. Open it manually from the file manager.");
    }
  };

  // Size guard: files > 50 MB require explicit user consent before loading
  if (doc.size > PDF_SIZE_LIMIT && !forceLoad) {
    const sizeMB = Math.round(doc.size / (1024 * 1024));
    return (
      <SizeGuardCard
        sizeMB={sizeMB}
        onLoad={() => setForceLoad(true)}
        onOpenExternal={handleOpenExternal}
      />
    );
  }

  const assetUrl = convertFileSrc(doc.path);

  return (
    <div className="h-full w-full bg-bg-primary">
      <iframe
        src={assetUrl}
        className="w-full h-full border-0"
        title={doc.name}
      />
    </div>
  );
}
