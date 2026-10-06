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
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger } from '@/components/ui/select';
import {
  Table, TableBody, TableCell, TableHead, TableHeader, TableRow,
} from '@/components/ui/table';
import { Textarea } from '@/components/ui/textarea';
import { api, openPage, subscribeQuotation } from '@/lib/api';
import {
  AWARD_REASON_LABELS, acceptableReasons, divergenceLabel, docIssue, fmtConnections, fmtCpf,
  fmtDateTime, fmtDuration, formatBRL, tradeOffLine,
} from '@/lib/domain';
import { errorMessage } from '@/lib/errors';
import {
  proposalsCount, type AwardReason, type RankingRow, type Report, type StaffQuotation,
} from '@/lib/types';

const PRICE_JUSTIFICATION = 'Menor preço entre as propostas habilitadas.';
const DEVIATION_MIN = 15;

/** Airline + flights, then departure · duration · connections. Bids from before
 *  the structured itinerary only have the free-text flight. */
function ItineraryCell({ r }: { r: RankingRow }) {
  if (r.airline === null || r.departsAt === null || r.durationMinutes === null || r.connections === null) {
    return <span>{r.flightInfo}</span>;
  }
  return (
    <div>
      <p>
        {r.airline} {r.flightInfo}
      </p>
      {/* Each piece stays whole; lines only break between them. */}
      <p className="text-xs text-muted-foreground">
        <span className="whitespace-nowrap">saída {fmtDateTime(r.departsAt)}</span> ·{' '}
        <span className="whitespace-nowrap">{fmtDuration(r.durationMinutes)}</span> ·{' '}
        <span className="whitespace-nowrap">{fmtConnections(r.connections)}</span>
      </p>
    </div>
  );
}

function HabilitacaoCell({ r }: { r: RankingRow }) {
  const issues = r.habilitacao.docs.flatMap((d) => {
    const issue = docIssue(d);
    return issue ? [issue] : [];
  });
  if (!r.habilitacao.eligible) {
    const why = issues.filter((i) => i.level === 'block').map((i) => i.text);
    if (r.habilitacao.supplierStatus !== 'ACTIVE') why.unshift('credenciamento inativo');
    return <span className="text-xs text-red-700">✘ {why.join('; ')}</span>;
  }
  return (
    <div className="text-xs">
      <p className="text-emerald-700">✔ habilitada</p>
      {issues.map((i) => (
        <p key={i.text} className="text-amber-700">
          {i.text}
        </p>
      ))}
    </div>
  );
}

function Tag({ children, tone }: { children: string; tone: 'emerald' | 'sky' }) {
  const colors = tone === 'emerald' ? 'bg-emerald-100 text-emerald-800' : 'bg-sky-100 text-sky-800';
  return (
    <span className={`ml-2 inline-block whitespace-nowrap rounded-full px-2 py-0.5 text-xs font-medium ${colors}`}>
      {children}
    </span>
  );
}

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
  const [justification, setJustification] = useState(PRICE_JUSTIFICATION);
  const [reason, setReason] = useState<AwardReason>('MENOR_PRECO');
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
    setJustification(PRICE_JUSTIFICATION);
    setReason('MENOR_PRECO');
    setConfirming(null);
  }, [id]);

  // Recommended winner: the lowest ELIGIBLE price, pre-selected (one-click
  // adjudication). A cheaper bid from a supplier with a lapsed certidão cannot win.
  // Only while CLOSED — historical views (AWARDED+) are read-only.
  useEffect(() => {
    const first = ranking.data?.ranking.find((r) => r.habilitacao.eligible);
    if (q?.status === 'CLOSED' && first && selected === null) setSelected(first.proposalId);
  }, [q?.status, ranking.data, selected]);

  // Picking another bid resets the decision text: price justifies the cheapest
  // eligible bid; anything else starts blank and has to be explained.
  function choose(row: RankingRow) {
    if (!row.habilitacao.eligible) return;
    setSelected(row.proposalId);
    const cheapest = ranking.data?.ranking.find((r) => r.habilitacao.eligible);
    if (!cheapest) return;
    const reasons = acceptableReasons(row, cheapest);
    setReason(reasons[0]);
    setJustification(reasons[0] === 'MENOR_PRECO' ? PRICE_JUSTIFICATION : '');
  }

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
      <Layout back>
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
      <Layout back>
        <p className="text-muted-foreground">Carregando…</p>
      </Layout>
    );
  }

  const isClosed = q.status === 'CLOSED';
  const winnerRow = q.awardedProposalId
    ? ranking.data?.ranking.find((r) => r.proposalId === q.awardedProposalId) ?? null
    : null;
  const selectedRow = ranking.data?.ranking.find((r) => r.proposalId === selected) ?? null;
  const eligibleRows = ranking.data?.ranking.filter((r) => r.habilitacao.eligible) ?? [];
  const cheapest = eligibleRows[0] ?? null;
  const fastest = eligibleRows.reduce<RankingRow | null>(
    (best, r) =>
      r.durationMinutes !== null && (best === null || r.durationMinutes < (best.durationMinutes ?? Infinity))
        ? r
        : best,
    null,
  );
  const reasons = selectedRow && cheapest ? acceptableReasons(selectedRow, cheapest) : [];
  const deviation = reasons.length > 0 && reasons[0] !== 'MENOR_PRECO';
  const canAward =
    selectedRow !== null &&
    reasons.includes(reason) &&
    justification.trim().length >= (deviation ? DEVIATION_MIN : 5);
  const ticket = report.data?.ticket ?? null;

  return (
    <Layout back>
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
            <p>Processo SEI: {q.seiProcess ?? '—'}</p>
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
                  : eligibleRows.length === 0
                  ? 'Nenhuma proposta habilitada — todas as fornecedoras têm certidão vencida ou faltando.'
                  : 'A proposta de menor preço habilitada já vem selecionada. Para escolher outra, informe o motivo.'}
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
                      <TableHead>Itinerário</TableHead>
                      {isClosed && <TableHead>Habilitação</TableHead>}
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
                              ? !r.habilitacao.eligible
                                ? 'opacity-60'
                                : selected === r.proposalId
                                  ? 'bg-primary/5'
                                  : ''
                              : isWinnerRow
                                ? 'bg-primary/5 font-medium'
                                : ''
                          }
                          onClick={isClosed ? () => choose(r) : undefined}
                        >
                          {isClosed && (
                            <TableCell>
                              <input
                                type="radio"
                                name="winner"
                                aria-label={`Selecionar ${r.supplier.legalName}`}
                                checked={selected === r.proposalId}
                                disabled={!r.habilitacao.eligible}
                                onChange={() => choose(r)}
                              />
                            </TableCell>
                          )}
                          <TableCell>
                            {r.position}º {r.supplier.legalName}
                            {!isClosed && isWinnerRow && <Tag tone="emerald">Vencedora</Tag>}
                            {isClosed && cheapest?.proposalId === r.proposalId && (
                              <Tag tone="emerald">mais barata</Tag>
                            )}
                            {isClosed &&
                              fastest?.proposalId === r.proposalId &&
                              cheapest?.proposalId !== r.proposalId && <Tag tone="sky">mais rápida</Tag>}
                            <p className="text-xs text-muted-foreground">
                              enviada às {fmtDateTime(r.submittedAt)}
                            </p>
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
                          <TableCell>
                            <ItineraryCell r={r} />
                          </TableCell>
                          {isClosed && (
                            <TableCell>
                              <HabilitacaoCell r={r} />
                            </TableCell>
                          )}
                        </TableRow>
                      );
                    })}
                  </TableBody>
                </Table>
                {isClosed ? (
                  eligibleRows.length === 0 ? null : (
                  <>
                    {deviation && selectedRow && cheapest && (
                      <div className="rounded-lg border border-amber-200 bg-amber-50 p-3 text-sm text-amber-900">
                        <p className="font-medium">
                          Proposta mais cara que a de menor preço habilitada (
                          {cheapest.supplier.legalName}, {formatBRL(cheapest.totalPriceCents)}).
                        </p>
                        <p>{tradeOffLine(selectedRow, cheapest)}.</p>
                      </div>
                    )}
                    {deviation && (
                      <div className="space-y-1.5">
                        <Label htmlFor="awardReason">Motivo da escolha</Label>
                        <Select value={reason} onValueChange={(v) => setReason(v as AwardReason)}>
                          <SelectTrigger id="awardReason" placeholder="Motivo" className="w-full md:w-[28rem]" />
                          <SelectContent>
                            {reasons.map((value, index) => (
                              <SelectItem key={value} index={index} value={value}>
                                {AWARD_REASON_LABELS[value]}
                              </SelectItem>
                            ))}
                          </SelectContent>
                        </Select>
                      </div>
                    )}
                    <div className="space-y-1.5">
                      <Label htmlFor="justification">Justificativa</Label>
                      <Textarea
                        id="justification"
                        value={justification}
                        onChange={(e) => setJustification(e.target.value)}
                        placeholder={
                          deviation
                            ? 'Explique por que esta proposta atende melhor o TJRR (ex.: chegada antes da audiência das 14h).'
                            : undefined
                        }
                      />
                      {deviation && (
                        <p className="text-xs text-muted-foreground">
                          Vai para o relatório anexado ao SEI · mínimo {DEVIATION_MIN} caracteres.
                        </p>
                      )}
                    </div>
                    <Button disabled={!canAward} onClick={() => setConfirming('award')}>
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
                          { proposalId: selected, reason, justification },
                          'Vencedora declarada — Ordem de Serviço emitida.',
                        )
                      }
                    >
                      {selectedRow && (
                        <div className="space-y-2 text-sm">
                          <p>
                            <span className="font-semibold">{selectedRow.supplier.legalName}</span>{' '}
                            — {formatBRL(selectedRow.totalPriceCents)}{' '}
                            <span className="whitespace-nowrap">
                              ({formatBRL(selectedRow.deltaFromReferenceCents)} vs referência)
                            </span>
                          </p>
                          <p>Motivo: {AWARD_REASON_LABELS[reason]}</p>
                          {deviation && cheapest && (
                            <p className="text-amber-800">{tradeOffLine(selectedRow, cheapest)}.</p>
                          )}
                          <p className="text-muted-foreground">
                            A adjudicação é definitiva: a Ordem de Serviço será emitida e a
                            vencedora terá 30 minutos para anexar o e-ticket.
                          </p>
                        </div>
                      )}
                    </ConfirmDialog>
                  </>
                  )
                ) : (
                  winnerRow && (
                    <div className="text-sm">
                      <p>
                        Vencedora:{' '}
                        <span className="font-semibold">{winnerRow.supplier.legalName}</span> —{' '}
                        {formatBRL(winnerRow.totalPriceCents)}
                        {q.awardedAt && <> · adjudicada em {fmtDateTime(q.awardedAt)}</>}
                      </p>
                      {q.awardReason && (
                        <p className="mt-1">Motivo: {AWARD_REASON_LABELS[q.awardReason]}</p>
                      )}
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
