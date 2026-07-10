/**
 * DeleteTripleButton — confirm-then-delete control for a single triple row.
 *
 * Used on both the EntityRelationsPanel (Task 1) and OwnershipAssetSection
 * (Task 3) triple rows. Confirms via AlertDialog before calling
 * useDeleteTriple, matching the Phase 11.5 D-12 user-override guarantee:
 * manual triples reappear on backfill, LLM-inserted triples do not.
 *
 * Phase 11.5 Plan 07, Task 1.
 */

import { useState } from "react";
import { Trash2 } from "lucide-react";
import { toast } from "sonner";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogTrigger,
} from "@/components/ui/alert-dialog";
import { useDeleteTriple } from "@/hooks/useTauri";

export interface DeleteTripleButtonProps {
  tripleId: string;
  affectedEntityIds: string[];
  label?: string;
}

export function DeleteTripleButton({
  tripleId,
  affectedEntityIds,
  label,
}: DeleteTripleButtonProps) {
  const [open, setOpen] = useState(false);
  const deleteTripleMutation = useDeleteTriple();

  const handleDelete = async () => {
    try {
      await deleteTripleMutation.mutateAsync({ tripleId, affectedEntityIds });
      toast.success("Relation deleted");
      setOpen(false);
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      toast.error(message);
    }
  };

  return (
    <AlertDialog open={open} onOpenChange={setOpen}>
      <AlertDialogTrigger asChild>
        <button
          type="button"
          disabled={deleteTripleMutation.isPending}
          aria-label={label ?? "Delete this relation"}
          className="inline-flex items-center justify-center text-text-tertiary hover:text-red-400 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
        >
          <Trash2 size={14} />
        </button>
      </AlertDialogTrigger>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>Delete this relation?</AlertDialogTitle>
          <AlertDialogDescription>
            The relation will be removed from Cortex. If it was auto-inverted (e.g.{" "}
            <code>owns</code>/<code>owned_by</code>), the paired relation is also deleted. This
            cannot be undone.
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel disabled={deleteTripleMutation.isPending}>Cancel</AlertDialogCancel>
          <AlertDialogAction
            onClick={(e) => {
              e.preventDefault();
              handleDelete();
            }}
            disabled={deleteTripleMutation.isPending}
            className="bg-red-600 text-white hover:bg-red-500 focus:ring-red-500"
          >
            Delete
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}
