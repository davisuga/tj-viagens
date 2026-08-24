import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useState, type FormEvent } from 'react';
import { Link } from 'react-router-dom';
import { toast } from 'sonner';
import { Countdown } from '@/components/Countdown';
import { Layout } from '@/components/Layout';
import { StatusBadge } from '@/components/StatusBadge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger } from '@/components/ui/select';
import { api } from '@/lib/api';
import { fmtDateTime, formatBRL } from '@/lib/domain';
import { errorMessage } from '@/lib/errors';
import type { NotificationItem, SupplierMe, SupplierQuotation } from '@/lib/types';

const DOC_TYPES = [
  ['CONTRATO_SOCIAL', 'Contrato social'],
  ['CND_FEDERAL', 'CND Federal (regularidade fiscal)'],
  ['CRF_FGTS', 'CRF do FGTS'],
  ['CNDT', 'CNDT (débitos trabalhistas)'],
] as const;

export function SupplierHome() {
  const queryClient = useQueryClient();
  const me = useQuery({ queryKey: ['me'], queryFn: () => api<SupplierMe>('/suppliers/me') });
  const notifications = useQuery({
    queryKey: ['notifications'],
    queryFn: () => api<NotificationItem[]>('/notifications'),
    refetchInterval: 15000,
  });
  const active = me.data?.supplier.status === 'ACTIVE';
  const quotations = useQuery({
    queryKey: ['quotations'],
    queryFn: () => api<SupplierQuotation[]>('/quotations'),
    enabled: active,
    refetchInterval: 15000,
  });

  const [docType, setDocType] = useState<string>(DOC_TYPES[0][0]);
  const [validUntil, setValidUntil] = useState('');
  const [file, setFile] = useState<File | null>(null);
  const [uploading, setUploading] = useState(false);

  async function uploadDoc(e: FormEvent) {
    e.preventDefault();
    if (!file) return;
    setUploading(true);
    const form = new FormData();
    form.append('type', docType);
    if (validUntil) form.append('validUntil', validUntil);
    form.append('file', file);
    try {
      await api('/suppliers/me/documents', { method: 'POST', form });
      toast.success('Documento enviado.');
      setFile(null);
      setValidUntil('');
      await queryClient.invalidateQueries({ queryKey: ['me'] });
    } catch (err) {
      toast.error(errorMessage(err));
    } finally {
      setUploading(false);
    }
  }

  if (me.isError) {
    return (
      <Layout>
        <div className="rounded-lg border bg-card p-6 text-center">
          <p className="text-sm text-destructive">{errorMessage(me.error)}</p>
          <Button className="mt-3" onClick={() => void me.refetch()}>Tentar novamente</Button>
        </div>
      </Layout>
    );
  }

  if (!me.data) {
    return (
      <Layout>
        <p className="text-muted-foreground">Carregando…</p>
      </Layout>
    );
  }
  const { supplier, checklist } = me.data;

  return (
    <Layout>
      <div className="grid gap-4 md:grid-cols-2">
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center justify-between text-base">
              Credenciamento <StatusBadge status={supplier.status} />
            </CardTitle>
            <p className="text-sm text-muted-foreground">
              {supplier.legalName} · CNPJ {supplier.cnpj}
            </p>
          </CardHeader>
          <CardContent className="space-y-4">
            {supplier.statusReason && (
              <p className="rounded bg-amber-50 p-2 text-sm">{supplier.statusReason}</p>
            )}
            <ul className="space-y-1 text-sm">
              {DOC_TYPES.map(([key, label]) => {
                const state = checklist.missing.includes(key)
                  ? '✖ pendente'
                  : checklist.expired.includes(key)
                    ? '⚠ vencido — reenvie'
                    : '✔ ok';
                return (
                  <li key={key} className="flex justify-between gap-2">
                    <span>{label}</span>
                    <span className="text-muted-foreground">{state}</span>
                  </li>
                );
              })}
            </ul>
            <form onSubmit={uploadDoc} className="space-y-2 border-t pt-3">
              <p className="text-sm font-medium">Enviar / atualizar documento</p>
              {/* This project's Select (Fluid Functionalism) renders its own value
                  display from `placeholder` — it takes no children/SelectValue, and
                  each SelectItem needs its list `index` for the proximity-hover overlay. */}
              <Select value={docType} onValueChange={setDocType}>
                <SelectTrigger placeholder="Tipo de documento" className="w-full" />
                <SelectContent>
                  {DOC_TYPES.map(([key, label], index) => (
                    <SelectItem key={key} index={index} value={key}>
                      {label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
              <div className="space-y-1.5">
                <Label htmlFor="validUntil">Válido até</Label>
                <Input id="validUntil" type="date" value={validUntil} onChange={(e) => setValidUntil(e.target.value)} />
              </div>
              <div className="space-y-1.5">
                <Label htmlFor="docFile">Arquivo do documento</Label>
                <Input
                  id="docFile"
                  type="file"
                  onChange={(e) => setFile(e.target.files?.[0] ?? null)}
                  required
                />
              </div>
              <Button type="submit" disabled={uploading || !file} className="w-full md:w-auto">
                {uploading ? 'Enviando…' : 'Enviar documento'}
              </Button>
            </form>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle className="text-base">Notificações</CardTitle>
          </CardHeader>
          <CardContent>
            <ul className="space-y-2 text-sm">
              {(notifications.data ?? []).map((n) => (
                <li key={n.id} className="rounded bg-muted p-2">
                  <p>{n.message}</p>
                  <p className="mt-1 text-xs text-muted-foreground">{fmtDateTime(n.createdAt)}</p>
                </li>
              ))}
              {(notifications.data ?? []).length === 0 && (
                <li className="text-muted-foreground">
                  Nenhuma notificação. Novas cotações aparecem aqui e no seu e-mail.
                </li>
              )}
            </ul>
          </CardContent>
        </Card>
      </div>

      {active && (
        <Card className="mt-4">
          <CardHeader>
            <CardTitle className="text-base">Cotações</CardTitle>
          </CardHeader>
          <CardContent className="grid gap-3 md:grid-cols-2">
            {(quotations.data ?? []).map((q) => (
              <Link key={q.id} to={`/fornecedor/cotacoes/${q.id}`} className="block">
                <Card className="transition hover:border-primary">
                  <CardContent className="flex items-center justify-between gap-2 p-4">
                    <div>
                      <p className="font-semibold">
                        {q.origin} → {q.destination}
                        {q.isWinner && ' 🏆'}
                      </p>
                      <p className="text-sm text-muted-foreground">
                        {q.code} · embarque {fmtDateTime(q.departureAt)}
                      </p>
                      <p className="text-sm">
                        {q.myProposal
                          ? `Minha proposta: ${formatBRL(q.myProposal.totalPriceCents)}`
                          : 'Sem proposta ainda'}
                      </p>
                    </div>
                    <div className="text-right">
                      <StatusBadge status={q.status} />
                      {q.status === 'OPEN' && q.closesAt && (
                        <div className="mt-1">
                          <Countdown deadline={q.closesAt} serverNow={q.serverNow} size="sm" />
                        </div>
                      )}
                    </div>
                  </CardContent>
                </Card>
              </Link>
            ))}
            {(quotations.data ?? []).length === 0 && (
              <p className="text-sm text-muted-foreground">
                Nenhuma cotação aberta no momento. Você será notificado por e-mail e neste painel.
              </p>
            )}
          </CardContent>
        </Card>
      )}
    </Layout>
  );
}
