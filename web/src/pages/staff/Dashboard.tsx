import { useQuery } from '@tanstack/react-query';
import { useMemo } from 'react';
import { Link } from 'react-router-dom';
import { Countdown } from '@/components/Countdown';
import { Layout } from '@/components/Layout';
import { StatusBadge } from '@/components/StatusBadge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { api } from '@/lib/api';
import { fmtDateTime, formatBRL } from '@/lib/domain';
import { errorMessage } from '@/lib/errors';
import { proposalsCount, type Metrics, type StaffQuotation } from '@/lib/types';

function QuotationLine({ q, cta }: { q: StaffQuotation; cta: string }) {
  return (
    <li className="flex flex-wrap items-center justify-between gap-2 rounded border p-3">
      <div className="min-w-0">
        <p className="font-medium">
          {q.code} · {q.origin} → {q.destination}
        </p>
        <p className="text-sm text-muted-foreground">
          {q.passenger.name} · embarque {fmtDateTime(q.departureAt)} · {proposalsCount(q.proposals)}{' '}
          proposta(s)
        </p>
      </div>
      <div className="flex shrink-0 items-center gap-3">
        {q.status === 'OPEN' && q.closesAt && (
          <Countdown deadline={q.closesAt} serverNow={q.serverNow} size="sm" />
        )}
        {q.status === 'AWARDED' && q.ticketDeadlineAt && (
          <Countdown deadline={q.ticketDeadlineAt} serverNow={q.serverNow} size="sm" />
        )}
        <Button asChild size="sm">
          <Link to={`/cotacoes/${q.id}`}>{cta}</Link>
        </Button>
      </div>
    </li>
  );
}

export function StaffDashboard() {
  // KPIs are a bonus strip, not the page's job — a metrics failure degrades
  // silently (the row just doesn't render) instead of blocking the queue below.
  const metrics = useQuery({ queryKey: ['metrics'], queryFn: () => api<Metrics>('/metrics/summary') });
  const quotations = useQuery({
    queryKey: ['staff-quotations'],
    queryFn: () => api<StaffQuotation[]>('/quotations'),
    refetchInterval: 10000,
  });

  const groups = useMemo(() => {
    const g = {
      toAward: [] as StaffQuotation[],
      toConfirm: [] as StaffQuotation[],
      draft: [] as StaffQuotation[],
      open: [] as StaffQuotation[],
      done: [] as StaffQuotation[],
    };
    for (const q of quotations.data ?? []) {
      if (q.status === 'CLOSED') g.toAward.push(q);
      else if (q.status === 'TICKETED') g.toConfirm.push(q);
      else if (q.status === 'DRAFT') g.draft.push(q);
      else if (q.status === 'OPEN' || q.status === 'AWARDED') g.open.push(q);
      else g.done.push(q);
    }
    return g;
  }, [quotations.data]);

  if (quotations.isError) {
    return (
      <Layout>
        <div className="rounded-lg border bg-card p-6 text-center">
          <p className="text-sm text-destructive">{errorMessage(quotations.error)}</p>
          <Button className="mt-3" onClick={() => void quotations.refetch()}>
            Tentar novamente
          </Button>
        </div>
      </Layout>
    );
  }

  if (!quotations.data) {
    return (
      <Layout>
        <p className="text-muted-foreground">Carregando…</p>
      </Layout>
    );
  }

  const kpis = metrics.data;
  const queue: Array<[string, StaffQuotation[], string]> = [
    ['Encerradas — declarar vencedora', groups.toAward, 'Declarar'],
    ['Bilhetes para conferir', groups.toConfirm, 'Conferir'],
    ['Rascunhos — abrir disputa', groups.draft, 'Abrir'],
    ['Em andamento', groups.open, 'Acompanhar'],
  ];

  return (
    <Layout>
      <div className="mb-4 flex flex-wrap items-center justify-between gap-2">
        <h1 className="text-xl font-bold">Painel de cotações</h1>
        <div className="flex gap-2">
          {/* Fluid Functionalism Button has no "outline" variant (primary/secondary/tertiary/ghost) —
              "tertiary" is its bordered/transparent equivalent. */}
          <Button variant="tertiary" asChild>
            <Link to="/fornecedores">Fornecedores</Link>
          </Button>
          <Button asChild>
            <Link to="/cotacoes/nova">Nova cotação</Link>
          </Button>
        </div>
      </div>

      {kpis && (
        <div className="mb-4 grid grid-cols-2 gap-3 md:grid-cols-4">
          {(
            [
              ['Economia acumulada', formatBRL(kpis.totalSavedCents), 'text-emerald-700'],
              ['Cotações adjudicadas', String(kpis.awardedCount), ''],
              ['Média de participantes', String(kpis.avgParticipants), ''],
              ['E-tickets no prazo', `${kpis.ticketsOnTimePct}%`, ''],
            ] as const
          ).map(([label, value, cls]) => (
            <Card key={label}>
              <CardContent className="p-4">
                <p className="text-xs text-muted-foreground">{label}</p>
                <p className={`text-xl font-bold ${cls}`}>{value}</p>
              </CardContent>
            </Card>
          ))}
        </div>
      )}

      {queue.map(([title, items, cta]) =>
        items.length === 0 ? null : (
          <Card key={title} className="mb-4">
            <CardHeader>
              <CardTitle className="text-base">{title}</CardTitle>
            </CardHeader>
            <CardContent>
              <ul className="space-y-2">
                {items.map((q) => (
                  <QuotationLine key={q.id} q={q} cta={cta} />
                ))}
              </ul>
            </CardContent>
          </Card>
        ),
      )}

      {groups.done.length > 0 && (
        <Card>
          <CardHeader>
            <CardTitle className="text-base">Concluídas</CardTitle>
          </CardHeader>
          <CardContent>
            <ul className="space-y-2 text-sm">
              {groups.done.map((q) => (
                <li key={q.id} className="flex items-center justify-between rounded border p-3">
                  <span>
                    {q.code} · {q.origin} → {q.destination}
                  </span>
                  <span className="flex items-center gap-2">
                    <StatusBadge status={q.status} />
                    <Link className="text-primary underline" to={`/cotacoes/${q.id}`}>
                      Ver dossiê
                    </Link>
                  </span>
                </li>
              ))}
            </ul>
          </CardContent>
        </Card>
      )}

      {quotations.data.length === 0 && (
        <Card>
          <CardContent className="p-8 text-center text-sm text-muted-foreground">
            Nenhuma cotação ainda. Clique em “Nova cotação” para registrar a primeira demanda.
          </CardContent>
        </Card>
      )}
    </Layout>
  );
}
