import { useState, useEffect } from "react";
import { Loader2 } from "lucide-react";
import { toast } from "sonner";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Button } from "@/components/ui/button";
import { useSaveSearch } from "@/hooks/useTauri";
import type { SavedSearchFilters } from "@/lib/types";

/**
 * shadcn Dialog for naming and persisting the current search via the save_search IPC.
 *
 * Behavior (UI-SPEC §3, D-09, 11-CONTEXT.md):
 *   - Name field auto-focuses when dialog opens and resets when it closes.
 *   - Enter key in the name field submits (same as clicking "Save search").
 *   - Save button disabled when name is empty/whitespace or IPC is in flight.
 *   - On success: sonner success toast + dialog closes.
 *   - On error: sonner error toast + dialog stays open (user can retry).
 *
 * Variant-safety note (UI-SPEC §3):
 *   "Keep searching" uses ghost variant — no accent at rest.
 *   "Save search" uses btn-primary class — accent fill is the Dialog's focal point.
 */
export function SaveSearchDialog({
  open,
  onOpenChange,
  query,
  filters,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  query: string;
  filters: SavedSearchFilters;
}) {
  const [name, setName] = useState("");
  const saveSearchMutation = useSaveSearch();

  // Reset name input each time the dialog opens (false → true transition).
  useEffect(() => {
    if (open) {
      setName("");
    }
  }, [open]);

  async function handleSave() {
    if (!name.trim()) return;
    try {
      const saved = await saveSearchMutation.mutateAsync({
        name: name.trim(),
        query,
        filters,
      });
      toast.success(`Search saved as "${saved.name}". Find it in the sidebar.`, {
        duration: 4000,
      });
      setName("");
      onOpenChange(false);
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      toast.error(`Failed to save search. ${message}`, { duration: 6000 });
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[400px]">
        <DialogHeader>
          <DialogTitle>Save search</DialogTitle>
          <DialogDescription>
            Name this search to find it quickly in the sidebar.
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4 py-4">
          <div className="space-y-2">
            <Label htmlFor="search-name" className="text-sm font-semibold text-text-primary">
              Name
            </Label>
            <Input
              id="search-name"
              placeholder="e.g. Property Tax 2024"
              value={name}
              onChange={(e) => setName(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && handleSave()}
              autoFocus
              className="focus-visible:ring-accent-primary"
            />
          </div>
        </div>

        <DialogFooter>
          <Button variant="ghost" onClick={() => onOpenChange(false)}>
            Keep searching
          </Button>
          <Button
            className="btn-primary"
            onClick={handleSave}
            disabled={!name.trim() || saveSearchMutation.isPending}
          >
            {saveSearchMutation.isPending && (
              <Loader2 size={14} className="animate-spin mr-2" />
            )}
            Save search
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
