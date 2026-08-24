import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useEffect, useState, type FormEvent } from 'react';
import { useParams } from 'react-router-dom';
import { toast } from 'sonner';
import { Countdown } from '@/components/Countdown';
import { Layout } from '@/components/Layout';
import { StatusBadge } from '@/components/StatusBadge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import { api, openPage, subscribeQuotation } from '@/lib/api';
import { fmtDateTime, formatBRL, parseBRL } from '@/lib/domain';
import { errorMessage } from '@/lib/errors';
import type { SupplierQuotation } from '@/lib/types';

export function SupplierQuotationPage() {
  const { id = '' } = useParams<{ id: string }>();
  const queryClient = useQueryClient();
  const { data: q } = useQuery({
    queryKey: ['quotation', id],
    queryFn: () => api<SupplierQuotation>(`/quotations/${id}`),
  });

  useEffect(() => {
    return subscribeQuotation(id, (event) => {
      if (event === 'status' || event === 'proposal') {
        void queryClient.invalidateQueries({ queryKey: ['quotation', id] });
      }
    });
  }, [id, queryClient]);

  const [price, setPrice] = useState('');
  const [flightInfo, setFlightInfo] = useState('');
  const [notes, setNotes] = useState('');
  const [pending, setPending] = useState(false);
  const [ticketPrice, setTicketPrice] = useState('');
  const [ticketFile, setTicketFile] = useState<File | null>(null);

  async function submitBid(e: FormEvent) {
    e.preventDefault();
    const cents = parseBRL(price);
    if (cents === null) {
      toast.error('Valor inválido — use o formato 1.523,00');
      return;
    }
    setPending(true);
    try {
      await api(`/quotations/${id}/proposals`, {
        method: 'POST',
        body: { totalPriceCents: cents, flightInfo, notes: notes || null },
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
      else toast.warning(`E-ticket enviado com divergências: ${res.divergences.join(', ')}`);
      await queryClient.invalidateQueries({ queryKey: ['quotation', id] });
    } catch (err) {
      toast.error(errorMessage(err));
    } finally {
      setPending(false);
    }
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
      {/* The brief: everything needed to quote, readable in 5 seconds */}
      <Card>
        <CardHeader>
          <CardTitle className="flex flex-wrap items-center justify-between gap-2 text-lg">
            <span>
              {q.code} · {q.origin} → {q.destination}
            </span>
            <StatusBadge status={q.status} />
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

      {q.status === 'OPEN' && (
        <Card className="mt-4">
          <CardHeader>
            <CardTitle className="text-base">Minha proposta (sigilosa)</CardTitle>
            {q.myProposal && (
              <p className="text-sm text-muted-foreground">
                Registrada: {formatBRL(q.myProposal.totalPriceCents)} às{' '}
                {fmtDateTime(q.myProposal.submittedAt)} — enviar novamente substitui.
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
              <div className="space-y-1.5">
                <Label htmlFor="flight">Voo ofertado</Label>
                <Input
                  id="flight"
                  placeholder="G3-1720 · 10/09 08:15"
                  value={flightInfo}
                  onChange={(e) => setFlightInfo(e.target.value)}
                  required
                />
              </div>
              <div className="space-y-1.5">
                <Label htmlFor="notes">Observações (bagagem, conexões…)</Label>
                <Textarea id="notes" value={notes} onChange={(e) => setNotes(e.target.value)} />
              </div>
              <Button type="submit" className="w-full md:w-auto" disabled={pending}>
                {pending ? 'Enviando…' : q.myProposal ? 'Substituir proposta' : 'Enviar proposta'}
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
                {q.passenger.name} · CPF {q.passenger.cpf} · {q.passenger.sex} · nasc.{' '}
                {q.passenger.birth}
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
              <Button type="submit" disabled={pending || !ticketFile} className="w-full md:w-auto">
                {pending ? 'Enviando…' : 'Anexar e-ticket'}
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
              : 'Cotação concluída.'}
          </CardContent>
        </Card>
      )}
    </Layout>
  );
}
