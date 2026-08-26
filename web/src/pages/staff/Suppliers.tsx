import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useState } from 'react';
import { toast } from 'sonner';
import { ConfirmDialog } from '@/components/ConfirmDialog';
import { Layout } from '@/components/Layout';
import { StatusBadge } from '@/components/StatusBadge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import {
  Table, TableBody, TableCell, TableHead, TableHeader, TableRow,
} from '@/components/ui/table';
import { Textarea } from '@/components/ui/textarea';
import { api } from '@/lib/api';
import { errorMessage } from '@/lib/errors';
import type { SupplierInfo, SupplierListItem } from '@/lib/types';

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
  const suppliers = useQuery({
    queryKey: ['suppliers'],
    queryFn: () => api<SupplierListItem[]>('/suppliers'),
  });
  const [pending, setPending] = useState(false);
  const [approving, setApproving] = useState<SupplierInfo | null>(null);
  const [rejecting, setRejecting] = useState<string | null>(null);
  const [reason, setReason] = useState('');

  async function decide(
    id: string,
    decision: 'APPROVE' | 'REJECT',
    why?: string,
  ): Promise<boolean> {
    setPending(true);
    try {
      await api(`/suppliers/${id}/decision`, {
        method: 'POST',
        body: { decision, reason: why ?? null },
      });
      toast.success(decision === 'APPROVE' ? 'Fornecedor homologado.' : 'Credenciamento rejeitado.');
      await queryClient.invalidateQueries({ queryKey: ['suppliers'] });
      return true;
    } catch (err) {
      toast.error(errorMessage(err));
      return false;
    } finally {
      setPending(false);
    }
  }

  if (suppliers.isError) {
    return (
      <Layout back>
        <div className="rounded-lg border bg-card p-6 text-center">
          <p className="text-sm text-destructive">{errorMessage(suppliers.error)}</p>
          <Button className="mt-3" onClick={() => void suppliers.refetch()}>
            Tentar novamente
          </Button>
        </div>
      </Layout>
    );
  }

  if (!suppliers.data) {
    return (
      <Layout back>
        <p className="text-muted-foreground">Carregando…</p>
      </Layout>
    );
  }
  const rows = suppliers.data;

  return (
    <Layout back>
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
              {rows.map(({ supplier, checklist }, index) => (
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
                          onClick={() => setApproving(supplier)}
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
          {rows.length === 0 && (
            <p className="p-4 text-center text-sm text-muted-foreground">
              Nenhuma solicitação de credenciamento ainda.
            </p>
          )}
        </CardContent>
      </Card>

      <ConfirmDialog
        open={approving !== null}
        onOpenChange={(open) => !open && setApproving(null)}
        title={approving ? `Homologar ${approving.legalName}?` : ''}
        confirmLabel="Confirmar homologação"
        pending={pending}
        onConfirm={() =>
          void (async () => {
            if (approving && (await decide(approving.id, 'APPROVE'))) setApproving(null);
          })()
        }
      >
        <p className="text-sm text-muted-foreground">
          O checklist documental está completo. Após a homologação, o fornecedor será notificado de
          todas as novas cotações do TJRR.
        </p>
      </ConfirmDialog>

      <ConfirmDialog
        open={rejecting !== null}
        onOpenChange={(open) => !open && setRejecting(null)}
        title="Justificativa da rejeição"
        confirmLabel="Confirmar rejeição"
        destructive
        pending={pending}
        confirmDisabled={reason.trim().length < 5}
        onConfirm={() =>
          void (async () => {
            if (rejecting && (await decide(rejecting, 'REJECT', reason))) {
              setRejecting(null);
              setReason('');
            }
          })()
        }
      >
        <Textarea
          value={reason}
          onChange={(e) => setReason(e.target.value)}
          placeholder="Ex.: Documentação fiscal vencida e não reapresentada."
        />
      </ConfirmDialog>
    </Layout>
  );
}
