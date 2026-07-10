/**
 * SplitAliasDialog — shadcn AlertDialog wrapper for confirming an alias split.
 *
 * Per UI-SPEC: Split is RECOVERABLE — uses accent-primary on the confirm button,
 * not the error/red color palette (split can be reversed by merging again).
 *
 * Plan 06-07 Task 1
 */

import {
  AlertDialog,
  AlertDialogContent,
  AlertDialogHeader,
  AlertDialogFooter,
  AlertDialogTitle,
  AlertDialogDescription,
  AlertDialogCancel,
  AlertDialogAction,
} from "@/components/ui/alert-dialog";

interface SplitAliasDialogProps {
  alias: string;
  open: boolean;
  onOpenChange: (v: boolean) => void;
  onConfirm: () => void;
}

export function SplitAliasDialog({
  alias,
  open,
  onOpenChange,
  onConfirm,
}: SplitAliasDialogProps) {
  return (
    <AlertDialog open={open} onOpenChange={onOpenChange}>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>Split &quot;{alias}&quot; off?</AlertDialogTitle>
          <AlertDialogDescription>
            This alias will become its own entity. Documents currently linked through this alias
            will be re-linked to the new entity. You can do this anytime if Cortex merged two
            distinct entities incorrectly.
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel>Cancel</AlertDialogCancel>
          <AlertDialogAction
            onClick={onConfirm}
            className="bg-accent-primary text-white hover:bg-accent-hover"
          >
            Split alias
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}
