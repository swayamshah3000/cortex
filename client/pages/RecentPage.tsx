import { useMemo } from "react";
import { Link } from "react-router-dom";
import {
  Clock,
  FileText,
  FileSpreadsheet,
  FileImage,
  File,
  FileCode,
} from "lucide-react";
import { isToday, isYesterday, isThisWeek } from "date-fns";
import { safeDistance } from "../lib/utils";
import { useRecentDocuments } from "../hooks/useTauri";
import { DocumentContextMenu } from "../components/documents/DocumentContextMenu";
import type { Document } from "../lib/types";

const fileTypeIcons: Record<string, typeof FileText> = {
  pdf: FileText,
  docx: FileText,
  txt: FileCode,
  md: FileCode,
  xlsx: FileSpreadsheet,
  csv: FileSpreadsheet,
  png: FileImage,
  jpg: FileImage,
};

function getFileIcon(docType: string) {
  return fileTypeIcons[docType] ?? File;
}

interface DateGroup {
  label: string;
  documents: Document[];
}

function groupByDate(documents: Document[]): DateGroup[] {
  const today: Document[] = [];
  const yesterday: Document[] = [];
  const thisWeek: Document[] = [];
  const older: Document[] = [];

  for (const doc of documents) {
    const date = new Date(doc.modifiedAt);
    if (isToday(date)) {
      today.push(doc);
    } else if (isYesterday(date)) {
      yesterday.push(doc);
    } else if (isThisWeek(date)) {
      thisWeek.push(doc);
    } else {
      older.push(doc);
    }
  }

  const groups: DateGroup[] = [];
  if (today.length > 0) groups.push({ label: "Today", documents: today });
  if (yesterday.length > 0) groups.push({ label: "Yesterday", documents: yesterday });
  if (thisWeek.length > 0) groups.push({ label: "This Week", documents: thisWeek });
  if (older.length > 0) groups.push({ label: "Older", documents: older });
  return groups;
}

function SkeletonRows() {
  return (
    <div className="space-y-3">
      {Array.from({ length: 5 }).map((_, i) => (
        <div key={i} className="card p-4 animate-pulse">
          <div className="flex items-center gap-3">
            <div className="w-10 h-10 rounded-lg bg-bg-tertiary" />
            <div className="flex-1 space-y-2">
              <div className="h-4 w-48 rounded bg-bg-tertiary" />
              <div className="h-3 w-32 rounded bg-bg-tertiary" />
            </div>
          </div>
        </div>
      ))}
    </div>
  );
}

export default function RecentPage() {
  const { data: documents, isLoading } = useRecentDocuments(50);

  const groups = useMemo(() => {
    if (!documents) return [];
    return groupByDate(documents);
  }, [documents]);

  if (isLoading) {
    return (
      <div className="space-y-6">
        <div className="space-y-2">
          <h1 className="page-title text-text-primary">Recent Documents</h1>
          <p className="text-text-secondary">Loading your recent activity...</p>
        </div>
        <SkeletonRows />
      </div>
    );
  }

  if (!documents || documents.length === 0) {
    return (
      <div className="flex items-center justify-center min-h-[60vh]">
        <div className="text-center space-y-4">
          <div className="mx-auto w-16 h-16 rounded-full bg-bg-tertiary flex items-center justify-center">
            <Clock size={32} className="text-text-tertiary" />
          </div>
          <h2 className="text-xl font-semibold text-text-primary">No recent documents</h2>
          <p className="text-text-secondary max-w-sm">
            Add a watched folder to start indexing your documents.
          </p>
          <Link
            to="/watched"
            className="inline-block mt-2 text-sm font-medium text-accent-primary hover:text-accent-hover transition-colors"
          >
            Add Watched Folder
          </Link>
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-8">
      <div className="space-y-2">
        <h1 className="page-title text-text-primary">Recent Documents</h1>
        <p className="text-text-secondary">
          {documents.length} document{documents.length !== 1 ? "s" : ""} across your spaces
        </p>
      </div>

      {groups.map((group) => (
        <div key={group.label} className="space-y-3">
          <h2 className="section-header text-text-secondary">{group.label}</h2>
          <div className="space-y-2">
            {group.documents.map((doc) => {
              const Icon = getFileIcon(doc.docType);
              return (
                <DocumentContextMenu key={doc.id} doc={doc}>
                  <Link
                    to={`/document/${doc.id}`}
                    className="card p-4 flex items-center gap-4 hover:border-accent-primary/50 transition-all"
                  >
                    <div className="p-2 rounded-lg bg-accent-subtle text-accent-primary flex-shrink-0">
                      <Icon size={20} />
                    </div>
                    <div className="flex-1 min-w-0">
                      <p className="font-medium text-text-primary text-sm truncate">
                        {doc.name}
                      </p>
                      {doc.excerpt && (
                        <p className="text-xs text-text-tertiary mt-1 truncate">
                          {doc.excerpt}
                        </p>
                      )}
                    </div>
                    {doc.spaceIds.length > 0 && (
                      <div className="hidden sm:flex gap-1 flex-shrink-0">
                        {doc.spaceIds.slice(0, 2).map((sid) => (
                          <span
                            key={sid}
                            className="text-xs px-2 py-0.5 rounded-full bg-bg-tertiary text-text-secondary"
                          >
                            {sid.replace("space-", "")}
                          </span>
                        ))}
                      </div>
                    )}
                    <span className="text-xs text-text-tertiary whitespace-nowrap flex-shrink-0">
                      {safeDistance(doc.modifiedAt)}
                    </span>
                  </Link>
                </DocumentContextMenu>
              );
            })}
          </div>
        </div>
      ))}
    </div>
  );
}
