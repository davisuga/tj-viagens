import type { ReactNode } from 'react';
import { Button } from '@/components/ui/button';
import {
  Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle,
} from '@/components/ui/dialog';

// Fluid Functionalism Button has no "destructive" variant (primary/secondary/
// tertiary/ghost) — tertiary plus a red text override is the combination that
// reads as destructive (see the rationale in pages/staff/Suppliers.tsx).
const DESTRUCTIVE_BUTTON = 'text-red-600 hover:text-red-700';

type ConfirmDialogProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  title: string;
  confirmLabel: string;
  onConfirm: () => void;
  pending?: boolean;
  confirmDisabled?: boolean;
  destructive?: boolean;
  children?: ReactNode;
};

/** Confirmation dialog for irreversible actions. Always offers "Cancelar",
 *  shows the spinner while pending and never closes itself — the caller closes
 *  on success, so a failed action keeps the dialog (and the error toast's
 *  context) on screen. Dismissal is blocked while the action is in flight. */
export function ConfirmDialog({
  open,
  onOpenChange,
  title,
  confirmLabel,
  onConfirm,
  pending = false,
  confirmDisabled = false,
  destructive = false,
  children,
}: ConfirmDialogProps) {
  return (
    <Dialog open={open} onOpenChange={(next) => !pending && onOpenChange(next)}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{title}</DialogTitle>
        </DialogHeader>
        {children}
        <DialogFooter>
          <Button variant="tertiary" disabled={pending} onClick={() => onOpenChange(false)}>
            Cancelar
          </Button>
          <Button
            variant={destructive ? 'tertiary' : 'primary'}
            className={destructive ? DESTRUCTIVE_BUTTON : undefined}
            loading={pending}
            disabled={confirmDisabled || pending}
            onClick={onConfirm}
          >
            {confirmLabel}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
