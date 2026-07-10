import { FileWarning } from "lucide-react";

interface SizeGuardCardProps {
  sizeMB: number;
  onLoad: () => void;
  onOpenExternal: () => void;
}

export function SizeGuardCard({ sizeMB, onLoad, onOpenExternal }: SizeGuardCardProps) {
  return (
    <div className="h-full w-full flex items-center justify-center bg-bg-primary p-6">
      <div className="card p-8 max-w-md text-center space-y-4">
        <FileWarning size={32} className="text-amber-400 mx-auto" />
        <h3 className="section-header text-text-primary">Large file</h3>
        <p className="text-text-secondary text-sm">
          This file is {sizeMB} MB. Preview may take a moment to load.
        </p>
        <div className="flex flex-col gap-2">
          <button
            type="button"
            onClick={onLoad}
            className="btn-primary"
          >
            Load preview
          </button>
          <button
            type="button"
            onClick={onOpenExternal}
            className="btn-secondary"
          >
            Open in default app
          </button>
        </div>
      </div>
    </div>
  );
}
