import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useEffect, useState } from 'react';
import { Link, useParams } from 'react-router-dom';
import { toast } from 'sonner';
import { ConfirmDialog } from '@/components/ConfirmDialog';
import { Countdown } from '@/components/Countdown';
import { Layout } from '@/components/Layout';
import { LivePill } from '@/components/LivePill';
import { StatusBadge } from '@/components/StatusBadge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import {
  Table, TableBody, TableCell, TableHead, TableHeader, TableRow,
} from '@/components/ui/table';
import { Textarea } from '@/components/ui/textarea';
import { api, openPage, subscribeQuotation } from '@/lib/api';
import { divergenceLabel, fmtCpf, fmtDateTime, formatBRL } from '@/lib/domain';
import { errorMessage } from '@/lib/errors';
import {
  proposalsCount, type RankingRow, type Report, type StaffQuotation,
} from '@/lib/types';

/** One line of the deterministic conference: the expected value (which the SPA
 *  has) plus the backend's verdict for that field. The ticket's own extracted
 *  values are not exposed by the API — only the verdicts are. */
function TicketVerdictRow({
  label, expected, diverged, divergedLabel,
}: {
  label: string;
  expected: string;
  diverged: boolean;
  divergedLabel: string;
}) {
  return (
    <div className="flex items-start justify-between gap-2 rounded bg-muted p-2">
      <div>
        <p className="text-xs text-muted-foreground">{label}</p>
        <p>{expected}</p>
      </div>
      <span className={`shrink-0 text-xs ${diverged ? 'text-amber-700' : 'text-emerald-700'}`}>
        {diverged ? `⚠ ${divergedLabel}` : '✔ Confere'}
      </span>
    </div>
  );
}

export function StaffQuotationDetail() {
  const { id = '' } = useParams<{ id: string }>();
  const queryClient = useQueryClient();
  const [pending, setPending] = useState(false);
  const [selected, setSelected] = useState<string | null>(null);
  const [justification, setJustification] = useState('Menor preço entre as propostas válidas.');
  const [confirming, setConfirming] = useState<null | 'open' | 'award' | 'conclude'>(null);
  const [liveDown, setLiveDown] = useState(false);

  const quotationQuery = useQuery({
    queryKey: ['staff-quotation', id],
    queryFn: () => api<StaffQuotation>(`/quotations/${id}`),
    // SSE fallback: the live countdown/status still updates even if the browser
    // misses (or never opens) the EventSource connection.
    refetchInterval: 15000,
  });
  const q = quotationQuery.data;
  const closedOrLater =
    q && ['CLOSED', 'AWARDED', 'TICKETED', 'COMPLETED'].includes(q.status);
  const ranking = useQuery({
    queryKey: ['ranking', id],
    queryFn: () => api<{ ranking: RankingRow[] }>(`/quotations/${id}/ranking`),
    enabled: Boolean(closedOrLater),
  });
  const showDossier = q && ['TICKETED', 'COMPLETED'].includes(q.status);
  // The report is staff-only but not status-gated — from OPEN on it carries the
  // notified-supplier count, and from TICKETED the conference verdicts.
  const report = useQuery({
    queryKey: ['report', id],
    queryFn: () => api<Report>(`/quotations/${id}/report.json`),
    enabled: Boolean(q && q.status !== 'DRAFT'),
  });
  const audit = useQuery({
    queryKey: ['audit'],
    queryFn: () => api<{ ok: boolean }>('/audit/verify'),
    enabled: Boolean(showDossier),
  });

  useEffect(() => {
    return subscribeQuotation(id, (event) => {
      if (event === 'down' || event === 'closed') {
        setLiveDown(true);
        return;
      }
      setLiveDown(false);
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
    setConfirming(null);
  }, [id]);

  // recommended winner: lowest price pre-selected (UX: one-click adjudication).
  // Only while CLOSED — historical views (AWARDED+) are read-only.
  useEffect(() => {
    const first = ranking.data?.ranking[0];
    if (q?.status === 'CLOSED' && first && selected === null) setSelected(first.proposalId);
  }, [q?.status, ranking.data, selected]);

  async function act(path: string, body?: unknown, success?: string): Promise<boolean> {
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
      return true;
    } catch (err) {
      toast.error(errorMessage(err));
      return false;
    } finally {
      setPending(false);
    }
  }

  // The confirm dialogs close only on success — a failure keeps the dialog
  // open next to its error toast.
  async function confirmAction(path: string, body: unknown | undefined, success: string) {
    if (await act(path, body, success)) setConfirming(null);
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

  const isClosed = q.status === 'CLOSED';
  const winnerRow = q.awardedProposalId
    ? ranking.data?.ranking.find((r) => r.proposalId === q.awardedProposalId) ?? null
    : null;
  const selectedRow = ranking.data?.ranking.find((r) => r.proposalId === selected) ?? null;
  const ticket = report.data?.ticket ?? null;

  return (
    <Layout>
      <Card>
        <CardHeader>
          <CardTitle className="flex flex-wrap items-center justify-between gap-2 text-lg">
            <span>
              {q.code} · {q.origin} → {q.destination}
            </span>
            <span className="flex flex-wrap items-center gap-2">
              {liveDown && <LivePill />}
              <StatusBadge status={q.status} />
            </span>
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
            <Button size="lg" onClick={() => setConfirming('open')}>
              Abrir cotação
            </Button>
            <ConfirmDialog
              open={confirming === 'open'}
              onOpenChange={(open) => !open && setConfirming(null)}
              title={`Abrir a disputa ${q.code}?`}
              confirmLabel="Confirmar abertura"
              pending={pending}
              onConfirm={() =>
                void confirmAction('open', undefined, 'Cotação aberta — fornecedores notificados.')
              }
            >
              <p className="text-sm text-muted-foreground">
                A notificação é irreversível e o cronômetro oficial inicia imediatamente.
              </p>
            </ConfirmDialog>
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
            {report.data && (
              <p className="mt-2 text-xs text-muted-foreground">
                🔔 {report.data.notifiedSuppliers} fornecedores ativos notificados simultaneamente
              </p>
            )}
          </CardContent>
        </Card>
      )}

      {closedOrLater && (
        <Card className="mt-4">
          <CardHeader>
            <CardTitle className="text-base">
              {isClosed ? 'Propostas — menor para maior' : 'Propostas da disputa'}
            </CardTitle>
            <p className="text-sm text-muted-foreground">
              {!isClosed
                ? 'Disputa encerrada — proposta vencedora destacada.'
                : ranking.data?.ranking.length === 0
                  ? 'A janela encerrou sem propostas.'
                  : 'A 1ª colocada já vem selecionada. Confira a conformidade e declare a vencedora.'}
            </p>
          </CardHeader>
          <CardContent className="space-y-3">
            {ranking.isPending ? (
              <p className="text-sm text-muted-foreground">Carregando propostas…</p>
            ) : ranking.isError ? (
              <div className="text-center">
                <p className="text-sm text-destructive">{errorMessage(ranking.error)}</p>
                <Button
                  variant="tertiary"
                  className="mt-2"
                  loading={ranking.isFetching}
                  onClick={() => void ranking.refetch()}
                >
                  Tentar novamente
                </Button>
              </div>
            ) : ranking.data.ranking.length === 0 ? (
              <div className="p-4 text-center">
                <p className="text-sm font-medium">
                  Disputa deserta — nenhuma proposta recebida no prazo.
                </p>
                <p className="mt-1 text-sm text-muted-foreground">
                  Nenhum fornecedor apresentou proposta. Registre uma nova cotação para repetir a
                  disputa.
                </p>
                <Button variant="tertiary" className="mt-3" asChild>
                  <Link to="/cotacoes/nova">Nova cotação</Link>
                </Button>
              </div>
            ) : (
              <>
                <Table>
                  <TableHeader>
                    <TableRow>
                      {isClosed && <TableHead />}
                      <TableHead>Fornecedor</TableHead>
                      <TableHead>Valor</TableHead>
                      <TableHead>Δ vs referência</TableHead>
                      <TableHead>Voo</TableHead>
                      <TableHead>Enviada às</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {ranking.data.ranking.map((r, index) => {
                      const isWinnerRow = r.proposalId === q.awardedProposalId;
                      return (
                        <TableRow
                          key={r.proposalId}
                          index={index}
                          className={
                            isClosed
                              ? selected === r.proposalId
                                ? 'bg-primary/5'
                                : ''
                              : isWinnerRow
                                ? 'bg-primary/5 font-medium'
                                : ''
                          }
                          onClick={isClosed ? () => setSelected(r.proposalId) : undefined}
                        >
                          {isClosed && (
                            <TableCell>
                              <input
                                type="radio"
                                name="winner"
                                aria-label={`Selecionar ${r.supplier.legalName}`}
                                checked={selected === r.proposalId}
                                onChange={() => setSelected(r.proposalId)}
                              />
                            </TableCell>
                          )}
                          <TableCell>
                            {r.position}º {r.supplier.legalName}
                            {!isClosed && isWinnerRow && (
                              <span className="ml-2 rounded-full bg-emerald-100 px-2 py-0.5 text-xs font-medium text-emerald-800">
                                Vencedora
                              </span>
                            )}
                          </TableCell>
                          <TableCell className="font-semibold">
                            {formatBRL(r.totalPriceCents)}
                          </TableCell>
                          <TableCell
                            className={
                              r.deltaFromReferenceCents < 0 ? 'text-emerald-700' : 'text-red-700'
                            }
                          >
                            {formatBRL(r.deltaFromReferenceCents)}
                          </TableCell>
                          <TableCell>{r.flightInfo}</TableCell>
                          <TableCell>{fmtDateTime(r.submittedAt)}</TableCell>
                        </TableRow>
                      );
                    })}
                  </TableBody>
                </Table>
                {isClosed ? (
                  <>
                    <Textarea
                      value={justification}
                      onChange={(e) => setJustification(e.target.value)}
                      aria-label="Justificativa"
                    />
                    <Button
                      disabled={selected === null || justification.trim().length < 5}
                      onClick={() => setConfirming('award')}
                    >
                      Declarar vencedora e emitir OS
                    </Button>
                    <ConfirmDialog
                      open={confirming === 'award'}
                      onOpenChange={(open) => !open && setConfirming(null)}
                      title="Declarar vencedora e emitir a OS?"
                      confirmLabel="Confirmar adjudicação"
                      pending={pending}
                      onConfirm={() =>
                        void confirmAction(
                          'award',
                          { proposalId: selected, justification },
                          'Vencedora declarada — Ordem de Serviço emitida.',
                        )
                      }
                    >
                      {selectedRow && (
                        <div className="space-y-2 text-sm">
                          <p>
                            <span className="font-semibold">{selectedRow.supplier.legalName}</span>{' '}
                            — {formatBRL(selectedRow.totalPriceCents)} (
                            {formatBRL(selectedRow.deltaFromReferenceCents)} vs referência)
                          </p>
                          <p className="text-muted-foreground">
                            A adjudicação é definitiva: a Ordem de Serviço será emitida e a
                            vencedora terá 30 minutos para anexar o e-ticket.
                          </p>
                        </div>
                      )}
                    </ConfirmDialog>
                  </>
                ) : (
                  winnerRow && (
                    <div className="text-sm">
                      <p>
                        Vencedora:{' '}
                        <span className="font-semibold">{winnerRow.supplier.legalName}</span> —{' '}
                        {formatBRL(winnerRow.totalPriceCents)}
                        {q.awardedAt && <> · adjudicada em {fmtDateTime(q.awardedAt)}</>}
                      </p>
                      {q.awardJustification && (
                        <p className="mt-1 italic text-muted-foreground">
                          Justificativa: “{q.awardJustification}”
                        </p>
                      )}
                    </div>
                  )
                )}
              </>
            )}
          </CardContent>
        </Card>
      )}

      {q.status === 'AWARDED' && (
        <Card className="mt-4">
          <CardHeader>
            <CardTitle className="text-base">Aguardando e-ticket da vencedora</CardTitle>
          </CardHeader>
          <CardContent className="space-y-3">
            {winnerRow && (
              <p className="text-sm">
                Vencedora: <span className="font-semibold">{winnerRow.supplier.legalName}</span> —{' '}
                {formatBRL(winnerRow.totalPriceCents)}
              </p>
            )}
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

      {showDossier && (
        <div className="mt-4 grid gap-4 md:grid-cols-2">
          <Card>
            <CardHeader>
              <CardTitle className="text-base">Conferência do e-ticket</CardTitle>
            </CardHeader>
            <CardContent className="space-y-3 text-sm">
              {report.isPending ? (
                <p className="text-muted-foreground">Carregando conferência…</p>
              ) : report.isError ? (
                <div className="text-center">
                  <p className="text-destructive">{errorMessage(report.error)}</p>
                  <Button
                    variant="tertiary"
                    className="mt-2"
                    loading={report.isFetching}
                    onClick={() => void report.refetch()}
                  >
                    Tentar novamente
                  </Button>
                </div>
              ) : !ticket ? (
                <p className="text-muted-foreground">Aguardando envio do e-ticket.</p>
              ) : (
                <>
                  <div className="flex flex-wrap items-center justify-between gap-2 rounded bg-muted p-2">
                    <div>
                      <p className="font-medium">{ticket.fileName}</p>
                      <p className="text-xs text-muted-foreground">
                        enviado em {fmtDateTime(ticket.uploadedAt)}
                      </p>
                    </div>
                    <span
                      className={`shrink-0 rounded-full px-2 py-0.5 text-xs font-medium ${
                        ticket.late
                          ? 'bg-amber-100 text-amber-800'
                          : 'bg-emerald-100 text-emerald-800'
                      }`}
                    >
                      {ticket.late ? 'Fora do prazo de 30 min' : 'Dentro do prazo'}
                    </span>
                  </div>
                  <div className="space-y-2">
                    <TicketVerdictRow
                      label="Passageiro"
                      expected={q.passenger.name}
                      diverged={ticket.divergences.includes('PASSAGEIRO_DIVERGENTE')}
                      divergedLabel="Diverge do pedido"
                    />
                    <TicketVerdictRow
                      label="Valor (proposta vencedora)"
                      expected={winnerRow ? formatBRL(winnerRow.totalPriceCents) : '—'}
                      diverged={ticket.divergences.includes('VALOR_DIVERGENTE')}
                      divergedLabel="Diverge da proposta"
                    />
                    <TicketVerdictRow
                      label="Data de embarque"
                      expected={fmtDateTime(q.departureAt)}
                      diverged={ticket.divergences.includes('DATA_DIVERGENTE')}
                      divergedLabel="Diverge do pedido"
                    />
                  </div>
                  <p className={ticket.divergences.length === 0 ? 'text-emerald-700' : 'text-amber-700'}>
                    {ticket.divergences.length === 0
                      ? '✔ Sem divergências detectadas — conferência automática aprovada.'
                      : `⚠ ${ticket.divergences.length} divergência(s) detectada(s) — revise antes de concluir.`}
                  </p>
                  {q.status === 'TICKETED' && (
                    <Button onClick={() => setConfirming('conclude')}>Confirmar e concluir</Button>
                  )}
                  <ConfirmDialog
                    open={confirming === 'conclude'}
                    onOpenChange={(open) => !open && setConfirming(null)}
                    title="Concluir a cotação?"
                    confirmLabel="Concluir cotação"
                    pending={pending}
                    onConfirm={() =>
                      void confirmAction('ticket/confirm', undefined, 'Cotação concluída.')
                    }
                  >
                    <div className="space-y-2 text-sm">
                      {ticket.late && (
                        <p className="text-amber-700">
                          ⚠ O e-ticket foi enviado FORA do prazo de 30 minutos.
                        </p>
                      )}
                      {ticket.divergences.length > 0 ? (
                        <>
                          <p>Divergências detectadas na conferência:</p>
                          <ul className="list-disc pl-5 text-amber-700">
                            {ticket.divergences.map((d) => (
                              <li key={d}>{divergenceLabel(d)}</li>
                            ))}
                          </ul>
                          <p className="text-muted-foreground">
                            Ao concluir, você registra ciência das divergências acima.
                          </p>
                        </>
                      ) : (
                        <p className="text-muted-foreground">
                          {ticket.late
                            ? 'Sem divergências detectadas.'
                            : 'Sem divergências detectadas e enviado dentro do prazo.'}
                        </p>
                      )}
                    </div>
                  </ConfirmDialog>
                </>
              )}
            </CardContent>
          </Card>
          <Card>
            <CardHeader>
              <CardTitle className="text-base">Economicidade e dossiê</CardTitle>
            </CardHeader>
            <CardContent className="space-y-2">
              {report.data?.economy && (
                <p className="text-2xl font-bold text-emerald-700">
                  {formatBRL(report.data.economy.saved_cents)}{' '}
                  <span className="text-base font-normal">
                    ({report.data.economy.saved_pct.toFixed(2).replace('.', ',')}% abaixo da referência)
                  </span>
                </p>
              )}
              {winnerRow && (
                <p className="text-sm">
                  Vencedora: <span className="font-semibold">{winnerRow.supplier.legalName}</span>{' '}
                  — {formatBRL(winnerRow.totalPriceCents)}
                </p>
              )}
              {report.data?.serviceOrder && (
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
