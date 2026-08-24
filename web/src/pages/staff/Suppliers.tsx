import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useState } from 'react';
import { toast } from 'sonner';
import { Layout } from '@/components/Layout';
import { StatusBadge } from '@/components/StatusBadge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import {
  Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle,
} from '@/components/ui/dialog';
import {
  Table, TableBody, TableCell, TableHead, TableHeader, TableRow,
} from '@/components/ui/table';
import { Textarea } from '@/components/ui/textarea';
import { api } from '@/lib/api';
import { errorMessage } from '@/lib/errors';
import type { SupplierListItem } from '@/lib/types';

// Fluid Functionalism Button has no "destructive" variant (primary/secondary/
// tertiary/ghost). Its fill is painted by an internal absolutely-positioned
// layer keyed only off `variant` (see bgVariants in ui/button.tsx) — a
// background utility in `className` lands on the outer element and never
// becomes visible (verified: the inner layer still renders solid
// foreground/near-black over it). Text color DOES inherit through to the
// label, so "tertiary" (bordered, transparent) plus a red text override is
// the combination that actually reads as destructive.
const DESTRUCTIVE_BUTTON = 'text-red-600 hover:text-red-700';

export function StaffSuppliers() {
  const queryClient = useQueryClient();
  const { data: rows } = useQuery({
    queryKey: ['suppliers'],
    queryFn: () => api<SupplierListItem[]>('/suppliers'),
  });
  const [pending, setPending] = useState(false);
  const [rejecting, setRejecting] = useState<string | null>(null);
  const [reason, setReason] = useState('');

  async function decide(id: string, decision: 'APPROVE' | 'REJECT', why?: string) {
    setPending(true);
    try {
      await api(`/suppliers/${id}/decision`, {
        method: 'POST',
        body: { decision, reason: why ?? null },
      });
      toast.success(decision === 'APPROVE' ? 'Fornecedor homologado.' : 'Credenciamento rejeitado.');
      setRejecting(null);
      setReason('');
      await queryClient.invalidateQueries({ queryKey: ['suppliers'] });
    } catch (err) {
      toast.error(errorMessage(err));
    } finally {
      setPending(false);
    }
  }

  return (
    <Layout>
      <Card>
        <CardHeader>
          <CardTitle>Credenciamento de fornecedores</CardTitle>
          <p className="text-sm text-muted-foreground">
            A pré-triagem do checklist é automática e determinística; a homologação é decisão do
            servidor.
          </p>
        </CardHeader>
        <CardContent>
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Razão social</TableHead>
                <TableHead>CNPJ</TableHead>
                <TableHead>Checklist</TableHead>
                <TableHead>Status</TableHead>
                <TableHead />
              </TableRow>
            </TableHeader>
            <TableBody>
              {(rows ?? []).map(({ supplier, checklist }, index) => (
                <TableRow key={supplier.id} index={index}>
                  <TableCell>{supplier.legalName}</TableCell>
                  <TableCell>{supplier.cnpj}</TableCell>
                  <TableCell className="text-sm">
                    {checklist.ok
                      ? '✔ completo'
                      : `✖ ${[...checklist.missing.map((m) => `${m} pendente`), ...checklist.expired.map((x) => `${x} vencido`)].join(', ')}`}
                  </TableCell>
                  <TableCell>
                    <StatusBadge status={supplier.status} />
                  </TableCell>
                  <TableCell className="space-x-2">
                    {supplier.status === 'PENDING' && (
                      <>
                        <Button
                          size="sm"
                          disabled={pending || !checklist.ok}
                          onClick={() => void decide(supplier.id, 'APPROVE')}
                        >
                          Homologar
                        </Button>
                        <Button
                          size="sm"
                          variant="tertiary"
                          className={DESTRUCTIVE_BUTTON}
                          disabled={pending}
                          onClick={() => setRejecting(supplier.id)}
                        >
                          Rejeitar
                        </Button>
                      </>
                    )}
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
          {(rows ?? []).length === 0 && (
            <p className="p-4 text-center text-sm text-muted-foreground">
              Nenhuma solicitação de credenciamento ainda.
            </p>
          )}
        </CardContent>
      </Card>

      <Dialog open={rejecting !== null} onOpenChange={(open) => !open && setRejecting(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Justificativa da rejeição</DialogTitle>
          </DialogHeader>
          <Textarea
            value={reason}
            onChange={(e) => setReason(e.target.value)}
            placeholder="Ex.: Documentação fiscal vencida e não reapresentada."
          />
          <DialogFooter>
            <Button
              variant="tertiary"
              className={DESTRUCTIVE_BUTTON}
              disabled={pending || reason.trim().length < 5}
              onClick={() => rejecting && void decide(rejecting, 'REJECT', reason)}
            >
              Confirmar rejeição
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </Layout>
  );
}
