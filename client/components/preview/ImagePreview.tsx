import { useState } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { openPath } from "@tauri-apps/plugin-opener";
import { isTauri } from "@/lib/tauri";
import type { Document } from "@/lib/types";
import { SizeGuardCard } from "./SizeGuardCard";
import { UnsupportedPreview } from "./UnsupportedPreview";

const IMAGE_SIZE_LIMIT = 20 * 1024 * 1024; // 20 MB

interface ImagePreviewProps {
  doc: Document;
}

export function ImagePreview({ doc }: ImagePreviewProps) {
  const [forceLoad, setForceLoad] = useState(false);
  const [imgError, setImgError] = useState(false);

  // Browser-dev fallback: image preview requires the desktop app
  if (!isTauri()) {
    return <UnsupportedPreview doc={{ ...doc }} />;
  }

  const handleOpenExternal = async () => {
    try {
      await openPath(doc.path);
    } catch {
      toast.error("Could not open file. Open it manually from the file manager.");
    }
  };

  // Size guard: images > 20 MB require explicit user consent before loading
  if (doc.size > IMAGE_SIZE_LIMIT && !forceLoad) {
    const sizeMB = Math.round(doc.size / (1024 * 1024));
    return (
      <SizeGuardCard
        sizeMB={sizeMB}
        onLoad={() => setForceLoad(true)}
        onOpenExternal={handleOpenExternal}
      />
    );
  }

  // Image load error — show unsupported fallback
  if (imgError) {
    return <UnsupportedPreview doc={{ ...doc, name: "image file" }} />;
  }

  const assetUrl = convertFileSrc(doc.path);

  return (
    <div className="h-full w-full flex items-center justify-center bg-bg-primary p-6">
      <img
        src={assetUrl}
        alt={doc.name}
        className="max-w-full max-h-full object-contain rounded-md shadow-md"
        onError={() => setImgError(true)}
      />
    </div>
  );
}
