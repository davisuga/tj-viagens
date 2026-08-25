import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useEffect, useState } from 'react';
import { useParams } from 'react-router-dom';
import { toast } from 'sonner';
import { Countdown } from '@/components/Countdown';
import { Layout } from '@/components/Layout';
import { StatusBadge } from '@/components/StatusBadge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import {
  Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle, DialogTrigger,
} from '@/components/ui/dialog';
import {
  Table, TableBody, TableCell, TableHead, TableHeader, TableRow,
} from '@/components/ui/table';
import { Textarea } from '@/components/ui/textarea';
import { api, openPage, subscribeQuotation } from '@/lib/api';
import { fmtCpf, fmtDateTime, formatBRL } from '@/lib/domain';
import { errorMessage } from '@/lib/errors';
import {
  proposalsCount, type RankingRow, type Report, type StaffQuotation,
} from '@/lib/types';

export function StaffQuotationDetail() {
  const { id = '' } = useParams<{ id: string }>();
  const queryClient = useQueryClient();
  const [pending, setPending] = useState(false);
  const [selected, setSelected] = useState<string | null>(null);
  const [justification, setJustification] = useState('Menor preço entre as propostas válidas.');

  const quotationQuery = useQuery({
    queryKey: ['staff-quotation', id],
    queryFn: () => api<StaffQuotation>(`/quotations/${id}`),
    // SSE fallback: the live countdown/status still updates even if the browser
    // misses (or never opens) the EventSource connection.
    refetchInterval: 15000,
  });
  const q = quotationQuery.data;
  const closedOrLater = q && ['CLOSED'].includes(q.status);
  const ranking = useQuery({
    queryKey: ['ranking', id],
    queryFn: () => api<{ ranking: RankingRow[] }>(`/quotations/${id}/ranking`),
    enabled: Boolean(closedOrLater),
  });
  const showDossier = q && ['TICKETED', 'COMPLETED'].includes(q.status);
  const report = useQuery({
    queryKey: ['report', id],
    queryFn: () => api<Report>(`/quotations/${id}/report.json`),
    enabled: Boolean(showDossier),
  });
  const audit = useQuery({
    queryKey: ['audit'],
    queryFn: () => api<{ ok: boolean }>('/audit/verify'),
    enabled: Boolean(showDossier),
  });

  useEffect(() => {
    return subscribeQuotation(id, (event) => {
      if (event === 'status' || event === 'proposal') {
        void queryClient.invalidateQueries({ queryKey: ['staff-quotation', id] });
        void queryClient.invalidateQueries({ queryKey: ['ranking', id] });
        void queryClient.invalidateQueries({ queryKey: ['report', id] });
      }
    });
  }, [id, queryClient]);

  // Route param change reuses this component instance — reset per-quotation state.
  useEffect(() => {
    setSelected(null);
    setJustification('Menor preço entre as propostas válidas.');
  }, [id]);

  // recommended winner: lowest price pre-selected (UX: one-click adjudication)
  useEffect(() => {
    const first = ranking.data?.ranking[0];
    if (first && selected === null) setSelected(first.proposalId);
  }, [ranking.data, selected]);

  async function act(path: string, body?: unknown, success?: string) {
    setPending(true);
    try {
      await api(`/quotations/${id}/${path}`, { method: 'POST', body });
      if (success) toast.success(success);
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ['staff-quotation', id] }),
        queryClient.invalidateQueries({ queryKey: ['ranking', id] }),
        queryClient.invalidateQueries({ queryKey: ['report', id] }),
        queryClient.invalidateQueries({ queryKey: ['audit'] }),
        queryClient.invalidateQueries({ queryKey: ['staff-quotations'] }),
        queryClient.invalidateQueries({ queryKey: ['metrics'] }),
      ]);
    } catch (err) {
      toast.error(errorMessage(err));
    } finally {
      setPending(false);
    }
  }

  if (quotationQuery.isError) {
    return (
      <Layout>
        <div className="rounded-lg border bg-card p-6 text-center">
          <p className="text-sm text-destructive">{errorMessage(quotationQuery.error)}</p>
          <Button className="mt-3" onClick={() => void quotationQuery.refetch()}>
            Tentar novamente
          </Button>
        </div>
      </Layout>
    );
  }

  if (!q) {
    return (
      <Layout>
        <p className="text-muted-foreground">Carregando…</p>
      </Layout>
    );
  }

  return (
    <Layout>
      <Card>
        <CardHeader>
          <CardTitle className="flex flex-wrap items-center justify-between gap-2 text-lg">
            <span>
              {q.code} · {q.origin} → {q.destination}
            </span>
            <StatusBadge status={q.status} />
          </CardTitle>
          <div className="grid gap-1 text-sm text-muted-foreground md:grid-cols-2">
            <p>
              Passageiro: {q.passenger.name} · CPF {fmtCpf(q.passenger.cpf)}
            </p>
            <p>Embarque: {fmtDateTime(q.departureAt)}</p>
            <p>Voo de referência: {q.referenceFlight}</p>
            <p className="font-medium text-foreground">
              🔒 Preço de referência (sigiloso): {formatBRL(q.referencePriceCents)}
            </p>
          </div>
        </CardHeader>
      </Card>

      {q.status === 'DRAFT' && (
        <Card className="mt-4">
          <CardContent className="p-6 text-center">
            <p className="mb-3 text-sm text-muted-foreground">
              Ao abrir, todos os fornecedores credenciados ativos serão notificados simultaneamente
              e a janela de propostas começará a contar no horário oficial do servidor.
            </p>
            <Dialog>
              <DialogTrigger asChild>
                <Button size="lg">Abrir cotação</Button>
              </DialogTrigger>
              <DialogContent>
                <DialogHeader>
                  <DialogTitle>Abrir a disputa {q.code}?</DialogTitle>
                </DialogHeader>
                <p className="text-sm text-muted-foreground">
                  A notificação é irreversível e o cronômetro oficial inicia imediatamente.
                </p>
                <DialogFooter>
                  <Button
                    disabled={pending}
                    onClick={() => void act('open', undefined, 'Cotação aberta — fornecedores notificados.')}
                  >
                    Confirmar abertura
                  </Button>
                </DialogFooter>
              </DialogContent>
            </Dialog>
          </CardContent>
        </Card>
      )}

      {q.status === 'OPEN' && q.closesAt && (
        <Card className="mt-4">
          <CardContent className="p-8 text-center">
            <p className="text-sm">Janela de propostas em andamento</p>
            <Countdown
              deadline={q.closesAt}
              serverNow={q.serverNow}
              onExpire={() => void queryClient.invalidateQueries({ queryKey: ['staff-quotation', id] })}
            />
            <p className="mt-4 text-4xl font-bold">{proposalsCount(q.proposals)}</p>
            <p className="text-sm text-muted-foreground">
              propostas recebidas — valores lacrados até o encerramento (isonomia)
            </p>
          </CardContent>
        </Card>
      )}

      {q.status === 'CLOSED' && ranking.data && (
        <Card className="mt-4">
          <CardHeader>
            <CardTitle className="text-base">Propostas — menor para maior</CardTitle>
            <p className="text-sm text-muted-foreground">
              A 1ª colocada já vem selecionada. Confira a conformidade e declare a vencedora.
            </p>
          </CardHeader>
          <CardContent className="space-y-3">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead />
                  <TableHead>Fornecedor</TableHead>
                  <TableHead>Valor</TableHead>
                  <TableHead>Δ vs referência</TableHead>
                  <TableHead>Voo</TableHead>
                  <TableHead>Enviada às</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {ranking.data.ranking.map((r, index) => (
                  <TableRow
                    key={r.proposalId}
                    index={index}
                    className={selected === r.proposalId ? 'bg-primary/5' : ''}
                    onClick={() => setSelected(r.proposalId)}
                  >
                    <TableCell>
                      <input
                        type="radio"
                        name="winner"
                        aria-label={`Selecionar ${r.supplier.legalName}`}
                        checked={selected === r.proposalId}
                        onChange={() => setSelected(r.proposalId)}
                      />
                    </TableCell>
                    <TableCell>
                      {r.position}º {r.supplier.legalName}
                    </TableCell>
                    <TableCell className="font-semibold">{formatBRL(r.totalPriceCents)}</TableCell>
                    <TableCell
                      className={r.deltaFromReferenceCents < 0 ? 'text-emerald-700' : 'text-red-700'}
                    >
                      {formatBRL(r.deltaFromReferenceCents)}
                    </TableCell>
                    <TableCell>{r.flightInfo}</TableCell>
                    <TableCell>{fmtDateTime(r.submittedAt)}</TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
            <Textarea
              value={justification}
              onChange={(e) => setJustification(e.target.value)}
              aria-label="Justificativa"
            />
            <Button
              disabled={pending || selected === null || justification.trim().length < 5}
              onClick={() =>
                void act(
                  'award',
                  { proposalId: selected, justification },
                  'Vencedora declarada — Ordem de Serviço emitida.',
                )
              }
            >
              Declarar vencedora e emitir OS
            </Button>
          </CardContent>
        </Card>
      )}

      {q.status === 'AWARDED' && (
        <Card className="mt-4">
          <CardHeader>
            <CardTitle className="text-base">Aguardando e-ticket da vencedora</CardTitle>
          </CardHeader>
          <CardContent className="space-y-3">
            {q.ticketDeadlineAt && (
              <p>
                Prazo de 30 minutos:{' '}
                <Countdown deadline={q.ticketDeadlineAt} serverNow={q.serverNow} size="sm" />
              </p>
            )}
            {/* Fluid Functionalism Button has no "outline" variant (primary/secondary/tertiary/ghost) —
                "tertiary" is its bordered/transparent equivalent. */}
            <Button variant="tertiary" onClick={() => openPage(`/quotations/${q.id}/service-order`)}>
              Ver Ordem de Serviço
            </Button>
          </CardContent>
        </Card>
      )}

      {showDossier && report.data && (
        <div className="mt-4 grid gap-4 md:grid-cols-2">
          <Card>
            <CardHeader>
              <CardTitle className="text-base">Conferência do e-ticket</CardTitle>
            </CardHeader>
            <CardContent className="space-y-2 text-sm">
              {report.data.ticket && (
                <>
                  <div className="grid grid-cols-2 gap-2">
                    <div className="rounded bg-muted p-2">
                      <p className="text-xs text-muted-foreground">Pedido</p>
                      <p>{report.data.quotation.passengerName}</p>
                      <p>{fmtDateTime(q.departureAt)}</p>
                    </div>
                    <div className="rounded bg-muted p-2">
                      <p className="text-xs text-muted-foreground">Bilhete</p>
                      <p>{report.data.ticket.fileName}</p>
                      <p>
                        {report.data.ticket.late
                          ? '⚠ FORA do prazo de 30 min'
                          : '✔ dentro do prazo'}
                      </p>
                    </div>
                  </div>
                  <p>
                    {report.data.ticket.divergences.length === 0
                      ? '✔ Sem divergências detectadas'
                      : `⚠ Divergências: ${report.data.ticket.divergences.join(', ')}`}
                  </p>
                  {q.status === 'TICKETED' && (
                    <Button
                      disabled={pending}
                      onClick={() =>
                        void act('ticket/confirm', undefined, 'Cotação concluída.')
                      }
                    >
                      Confirmar e concluir
                    </Button>
                  )}
                </>
              )}
            </CardContent>
          </Card>
          <Card>
            <CardHeader>
              <CardTitle className="text-base">Economicidade e dossiê</CardTitle>
            </CardHeader>
            <CardContent className="space-y-2">
              {report.data.economy && (
                <p className="text-2xl font-bold text-emerald-700">
                  {formatBRL(report.data.economy.saved_cents)}{' '}
                  <span className="text-base font-normal">
                    ({report.data.economy.saved_pct.toFixed(2).replace('.', ',')}% abaixo da referência)
                  </span>
                </p>
              )}
              {report.data.serviceOrder && (
                <p className="text-sm text-muted-foreground">
                  {report.data.serviceOrder.number}
                </p>
              )}
              <p className="text-sm">
                {audit.data?.ok === true
                  ? '🔒 Trilha de auditoria íntegra'
                  : audit.data
                    ? '❌ Trilha de auditoria VIOLADA'
                    : ''}
              </p>
              <div className="flex flex-wrap gap-2">
                <Button variant="tertiary" onClick={() => openPage(`/quotations/${q.id}/report`)}>
                  Relatório (imprimir/PDF)
                </Button>
                <Button variant="tertiary" onClick={() => openPage(`/quotations/${q.id}/service-order`)}>
                  Ordem de Serviço
                </Button>
              </div>
            </CardContent>
          </Card>
        </div>
      )}
    </Layout>
  );
}
