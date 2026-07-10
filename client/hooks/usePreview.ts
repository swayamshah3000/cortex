/**
 * usePreview — React Query hook for fetching text content of a document.
 *
 * Used by TextPreview and MarkdownPreview to load file content via the
 * read_document_text IPC command. Size guards are applied in the caller.
 *
 * PAGE-13 / D-16: usePreview hook.
 */

import { useQuery } from "@tanstack/react-query";
import { tauriInvoke } from "@/lib/tauri";
import { queryKeys } from "./useTauri";
import type { DocumentTextPreview } from "@/lib/types";

/**
 * Fetches the text content of a document by ID.
 *
 * @param documentId - The document ID to fetch content for.
 * @returns React Query result with DocumentTextPreview data.
 */
export function usePreview(documentId: string) {
  return useQuery({
    queryKey: queryKeys.documentText(documentId),
    queryFn: () =>
      tauriInvoke<DocumentTextPreview>(
        "read_document_text",
        { docId: documentId, maxBytes: 5 * 1024 * 1024 },
        () => ({ text: "(mock preview text)", truncated: false, size: 100 }),
      ),
    enabled: Boolean(documentId),
  });
}
