import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useEffect, useRef, useState, type FormEvent } from 'react';
import { useParams } from 'react-router-dom';
import { toast } from 'sonner';
import { Countdown } from '@/components/Countdown';
import { Layout } from '@/components/Layout';
import { LivePill } from '@/components/LivePill';
import { StatusBadge } from '@/components/StatusBadge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger } from '@/components/ui/select';
import { Textarea } from '@/components/ui/textarea';
import { api, openPage, subscribeQuotation } from '@/lib/api';
import {
  divergenceLabel, docIssue, fmtConnections, fmtCpf, fmtDateOnly, fmtDateTime, fmtDuration,
  formatBRL, parseBRL, parseDuration, toBoaVistaLocalInput,
} from '@/lib/domain';
import { errorMessage } from '@/lib/errors';
import type { SupplierMe, SupplierQuotation } from '@/lib/types';

const CONNECTION_OPTIONS = [
  ['0', 'Direto (sem conexão)'],
  ['1', '1 conexão'],
  ['2', '2 conexões'],
  ['3', '3 ou mais'],
] as const;

/** 340 -> "5:40", the format the duration field accepts. */
function durationInput(minutes: number): string {
  return `${Math.floor(minutes / 60)}:${String(minutes % 60).padStart(2, '0')}`;
}

export function SupplierQuotationPage() {
  const { id = '' } = useParams<{ id: string }>();
  const queryClient = useQueryClient();
  const quotationQuery = useQuery({
    queryKey: ['quotation', id],
    queryFn: () => api<SupplierQuotation>(`/quotations/${id}`),
    refetchInterval: 15000,
  });
  const q = quotationQuery.data;
  const [liveDown, setLiveDown] = useState(false);
  // Own certidões: an agency with a lapsed one can still bid, but cannot be awarded.
  const me = useQuery({ queryKey: ['me'], queryFn: () => api<SupplierMe>('/suppliers/me') });
  const docIssues = (me.data?.checklist.docs ?? []).flatMap((d) => {
    const issue = docIssue(d);
    return issue ? [issue] : [];
  });
  const blocked = docIssues.some((i) => i.level === 'block');

  useEffect(() => {
    return subscribeQuotation(id, (event) => {
      if (event === 'down' || event === 'closed') {
        setLiveDown(true);
        return;
      }
      setLiveDown(false);
      if (event === 'status' || event === 'proposal') {
        void queryClient.invalidateQueries({ queryKey: ['quotation', id] });
      }
    });
  }, [id, queryClient]);

  const [price, setPrice] = useState('');
  const [airline, setAirline] = useState('');
  const [flightInfo, setFlightInfo] = useState('');
  const [departsAt, setDepartsAt] = useState('');
  const [duration, setDuration] = useState('');
  const [connections, setConnections] = useState('1');
  const [notes, setNotes] = useState('');
  const [pending, setPending] = useState(false);
  const [ticketPrice, setTicketPrice] = useState('');
  const [ticketFile, setTicketFile] = useState<File | null>(null);

  // Pre-fill from the registered proposal exactly once per proposal id, so
  // "Substituir proposta" edits start from what is on file (and a blank
  // resubmit can't silently wipe the notes). Later refetches never clobber typing.
  const hydratedProposalId = useRef<string | null>(null);
  useEffect(() => {
    const mine = q?.myProposal;
    if (mine && hydratedProposalId.current !== mine.id) {
      hydratedProposalId.current = mine.id;
      setPrice((mine.totalPriceCents / 100).toFixed(2).replace('.', ','));
      setFlightInfo(mine.flightInfo);
      setNotes(mine.notes ?? '');
      // Bids from before the structured itinerary come back blank and must be completed.
      setAirline(mine.airline ?? '');
      setDepartsAt(mine.departsAt ? toBoaVistaLocalInput(mine.departsAt) : '');
      setDuration(mine.durationMinutes !== null ? durationInput(mine.durationMinutes) : '');
      setConnections(String(Math.min(mine.connections ?? 1, 3)));
    }
  }, [q?.myProposal]);

  // Until the agency edits it, departure shows the requested one, so a first bid
  // only adjusts the time.
  const departsValue = departsAt || (q ? toBoaVistaLocalInput(q.departureAt) : '');

  async function submitBid(e: FormEvent) {
    e.preventDefault();
    const cents = parseBRL(price);
    if (cents === null) {
      toast.error('Valor inválido — use o formato 1.523,00');
      return;
    }
    const minutes = parseDuration(duration);
    if (minutes === null) {
      toast.error('Duração inválida — use horas:minutos, ex.: 5:40');
      return;
    }
    setPending(true);
    try {
      await api(`/quotations/${id}/proposals`, {
        method: 'POST',
        body: {
          totalPriceCents: cents,
          airline,
          flightInfo,
          // datetime-local has no timezone; the field is labeled Boa Vista time.
          departsAt: new Date(departsValue + ':00-04:00').toISOString(),
          durationMinutes: minutes,
          connections: Number(connections),
          notes: notes || null,
        },
      });
      toast.success('Proposta registrada. Enviar novamente substitui o valor.');
      await queryClient.invalidateQueries({ queryKey: ['quotation', id] });
    } catch (err) {
      toast.error(errorMessage(err));
    } finally {
      setPending(false);
    }
  }

  async function submitTicket(e: FormEvent) {
    e.preventDefault();
    if (!ticketFile || !q?.passenger) return;
    const cents = parseBRL(ticketPrice);
    if (cents === null) {
      toast.error('Valor do bilhete inválido.');
      return;
    }
    setPending(true);
    const form = new FormData();
    form.append('passengerName', q.passenger.name);
    form.append('flightInfo', q.myProposal?.flightInfo ?? '');
    form.append('departureAt', q.departureAt);
    form.append('priceCents', String(cents));
    form.append('file', ticketFile);
    try {
      const res = await api<{ late: boolean; divergences: string[] }>(`/quotations/${id}/ticket`, {
        method: 'POST',
        form,
      });
      if (res.divergences.length === 0) toast.success('E-ticket enviado sem divergências.');
      else
        toast.warning(
          `E-ticket enviado com divergências: ${res.divergences.map(divergenceLabel).join('; ')}`,
        );
      await queryClient.invalidateQueries({ queryKey: ['quotation', id] });
    } catch (err) {
      toast.error(errorMessage(err));
    } finally {
      setPending(false);
    }
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

  return (
    <Layout back>
      {/* The brief: everything needed to quote, readable in 5 seconds */}
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
          <p className="text-sm text-muted-foreground">
            Embarque {fmtDateTime(q.departureAt)}
            {q.returnAt ? ` · retorno ${fmtDateTime(q.returnAt)}` : ''} · voo de referência{' '}
            {q.referenceFlight}
          </p>
        </CardHeader>
        {q.status === 'OPEN' && q.closesAt && (
          <CardContent className="rounded-lg bg-muted/60 py-6 text-center">
            <p className="mb-1 text-sm">Tempo restante para propostas</p>
            <Countdown
              deadline={q.closesAt}
              serverNow={q.serverNow}
              onExpire={() => void queryClient.invalidateQueries({ queryKey: ['quotation', id] })}
            />
            <p className="mt-1 text-xs text-muted-foreground">
              Encerramento pelo horário oficial do servidor · {q.closesAt && fmtDateTime(q.closesAt)}{' '}
              (horário de Boa Vista)
            </p>
          </CardContent>
        )}
      </Card>

      {q.status === 'OPEN' && docIssues.length > 0 && (
        <div
          className={`mt-4 rounded-lg border p-3 text-sm ${
            blocked ? 'border-red-200 bg-red-50 text-red-800' : 'border-amber-200 bg-amber-50 text-amber-900'
          }`}
        >
          <p className="font-medium">{docIssues.map((i) => i.text).join(' · ')}</p>
          <p className="mt-1">
            {blocked
              ? 'Você pode enviar a proposta, mas o TJRR não pode adjudicar fornecedor com certidão vencida ou faltando. Atualize em Início antes do encerramento.'
              : 'Renove antes de vencer: com certidão vencida a proposta não pode ser adjudicada.'}
          </p>
        </div>
      )}

      {q.status === 'OPEN' && (
        <Card className="mt-4">
          <CardHeader>
            <CardTitle className="text-base">Minha proposta (sigilosa)</CardTitle>
            {q.myProposal && (
              <p className="text-sm text-muted-foreground">
                Registrada: {formatBRL(q.myProposal.totalPriceCents)}
                {q.myProposal.durationMinutes !== null && q.myProposal.connections !== null
                  ? ` · ${fmtDuration(q.myProposal.durationMinutes)}, ${fmtConnections(q.myProposal.connections)}`
                  : ''}{' '}
                às {fmtDateTime(q.myProposal.submittedAt)} — enviar novamente substitui.
              </p>
            )}
          </CardHeader>
          <CardContent>
            <form onSubmit={submitBid} className="space-y-3">
              <div className="space-y-1.5">
                <Label htmlFor="price">Valor total (R$)</Label>
                <Input
                  id="price"
                  inputMode="decimal"
                  placeholder="1.523,00"
                  value={price}
                  onChange={(e) => setPrice(e.target.value)}
                  required
                  className="text-lg"
                />
                {price && (
                  <p className="text-xs text-muted-foreground">
                    {parseBRL(price) !== null ? `= ${formatBRL(parseBRL(price)!)}` : 'Formato: 1.523,00'}
                  </p>
                )}
              </div>
              {/* Phone-first: the datetime and the flight numbers need the full width
                  at 390px or the time and the second flight get clipped. */}
              <div className="grid grid-cols-2 gap-3">
                {/* min-w-0: grid items default to min-content width, which let the
                    select's trigger push past the card edge at 390px. */}
                <div className="min-w-0 space-y-1.5">
                  <Label htmlFor="airline">Companhia</Label>
                  <Input
                    id="airline"
                    list="airlines"
                    placeholder="GOL"
                    value={airline}
                    onChange={(e) => setAirline(e.target.value)}
                    required
                  />
                  <datalist id="airlines">
                    <option value="LATAM" />
                    <option value="GOL" />
                    <option value="Azul" />
                  </datalist>
                </div>
                <div className="min-w-0 space-y-1.5">
                  <Label htmlFor="connections">Conexões</Label>
                  {/* Fluid Functionalism Select: value display comes from the selected
                      item; each SelectItem needs its list `index`. */}
                  <Select value={connections} onValueChange={setConnections}>
                    {/* The trigger ships with min-w-[160px]; a phone half-column is ~150px. */}
                    <SelectTrigger id="connections" placeholder="Conexões" className="w-full min-w-0" />
                    <SelectContent>
                      {CONNECTION_OPTIONS.map(([value, label], index) => (
                        <SelectItem key={value} index={index} value={value}>
                          {label}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                </div>
              </div>
              <div className="space-y-1.5">
                <Label htmlFor="flight">Voo(s)</Label>
                <Input
                  id="flight"
                  placeholder="G3-1720 / G3-1450"
                  value={flightInfo}
                  onChange={(e) => setFlightInfo(e.target.value)}
                  required
                />
              </div>
              <div className="space-y-1.5">
                <Label htmlFor="departsAt">Saída (horário de Boa Vista)</Label>
                <Input
                  id="departsAt"
                  type="datetime-local"
                  value={departsValue}
                  onChange={(e) => setDepartsAt(e.target.value)}
                  required
                />
              </div>
              <div className="space-y-1.5">
                <Label htmlFor="duration">Duração total da viagem</Label>
                <Input
                  id="duration"
                  placeholder="5:40"
                  inputMode="numeric"
                  value={duration}
                  onChange={(e) => setDuration(e.target.value)}
                  className="max-w-[10rem]"
                  required
                />
                <p className="text-xs text-muted-foreground">
                  {parseDuration(duration) !== null
                    ? `= ${fmtDuration(parseDuration(duration)!)}, da saída à chegada, contando conexões`
                    : 'horas:minutos, da saída à chegada, contando conexões'}
                </p>
              </div>
              <div className="space-y-1.5">
                <Label htmlFor="notes">Observações (bagagem, tarifa…)</Label>
                <Textarea id="notes" value={notes} onChange={(e) => setNotes(e.target.value)} />
              </div>
              <Button type="submit" className="w-full md:w-auto" loading={pending} disabled={pending}>
                {q.myProposal ? 'Substituir proposta' : 'Enviar proposta'}
              </Button>
              <p className="text-xs text-muted-foreground">
                Você não vê as propostas concorrentes nem o preço de referência do Tribunal — a
                disputa é cega e isonômica.
              </p>
            </form>
          </CardContent>
        </Card>
      )}

      {q.status === 'CLOSED' && (
        <Card className="mt-4">
          <CardContent className="p-6 text-center text-sm text-muted-foreground">
            Janela encerrada. Aguardando a declaração da vencedora pelo TJRR — você será notificado.
          </CardContent>
        </Card>
      )}

      {q.status === 'AWARDED' && !q.isWinner && (
        <Card className="mt-4">
          <CardContent className="space-y-2 p-6 text-center text-sm">
            <p className="font-medium">Disputa encerrada</p>
            <p className="text-muted-foreground">
              A cotação foi adjudicada a outra proposta. Sua proposta não foi a selecionada desta
              vez.
            </p>
            <p className="text-muted-foreground">
              {q.myProposal
                ? `Sua proposta: ${formatBRL(q.myProposal.totalPriceCents)} · registrada às ${fmtDateTime(q.myProposal.submittedAt)}.`
                : 'Você não chegou a registrar proposta nesta cotação.'}
            </p>
            <p className="text-muted-foreground">
              Agradecemos a participação — você continuará recebendo as próximas cotações do TJRR.
            </p>
          </CardContent>
        </Card>
      )}

      {q.isWinner && q.status === 'AWARDED' && q.passenger && (
        <Card className="mt-4 border-primary">
          <CardHeader>
            <CardTitle className="text-base">🏆 Sua proposta venceu — emita e anexe o e-ticket</CardTitle>
            {q.ticketDeadlineAt && (
              <p className="text-sm">
                Prazo de emissão:{' '}
                <Countdown deadline={q.ticketDeadlineAt} serverNow={q.serverNow} size="sm" />
              </p>
            )}
          </CardHeader>
          <CardContent className="space-y-3">
            <div className="rounded bg-muted p-3 text-sm">
              <p className="font-semibold">Dados do passageiro</p>
              <p>
                {q.passenger.name} · CPF {fmtCpf(q.passenger.cpf)} · {q.passenger.sex} · nasc.{' '}
                {fmtDateOnly(q.passenger.birth)}
              </p>
            </div>
            {/* Fluid Functionalism Button has no "outline" variant (primary/secondary/tertiary/ghost) —
                "tertiary" is its bordered/transparent equivalent. */}
            <Button variant="tertiary" onClick={() => openPage(`/quotations/${q.id}/service-order`)}>
              Ver Ordem de Serviço
            </Button>
            <form onSubmit={submitTicket} className="space-y-3 border-t pt-3">
              <div className="space-y-1.5">
                <Label htmlFor="ticketPrice">Valor emitido (R$)</Label>
                <Input
                  id="ticketPrice"
                  inputMode="decimal"
                  placeholder="1.523,00"
                  value={ticketPrice}
                  onChange={(e) => setTicketPrice(e.target.value)}
                  required
                />
                {ticketPrice && (
                  <p className="text-xs text-muted-foreground">
                    {parseBRL(ticketPrice) !== null
                      ? `= ${formatBRL(parseBRL(ticketPrice)!)}`
                      : 'Formato: 1.523,00'}
                  </p>
                )}
              </div>
              <div className="space-y-1.5">
                <Label htmlFor="ticketFile">Arquivo do e-ticket (PDF)</Label>
                <Input
                  id="ticketFile"
                  type="file"
                  onChange={(e) => setTicketFile(e.target.files?.[0] ?? null)}
                  required
                />
              </div>
              <Button
                type="submit"
                loading={pending}
                disabled={pending || !ticketFile}
                className="w-full md:w-auto"
              >
                Anexar e-ticket
              </Button>
            </form>
          </CardContent>
        </Card>
      )}

      {(q.status === 'TICKETED' || q.status === 'COMPLETED') && (
        <Card className="mt-4">
          <CardContent className="p-6 text-center text-sm">
            {q.isWinner
              ? 'E-ticket enviado. O TJRR fará a conferência final.'
              : 'Cotação concluída. Sua proposta não foi a selecionada nesta disputa — agradecemos a participação.'}
          </CardContent>
        </Card>
      )}
    </Layout>
  );
}
