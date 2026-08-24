import { useState, type FormEvent } from 'react';
import { useNavigate } from 'react-router-dom';
import { toast } from 'sonner';
import { Layout } from '@/components/Layout';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger } from '@/components/ui/select';
import { api } from '@/lib/api';
import { formatBRL, parseBRL } from '@/lib/domain';
import { errorMessage } from '@/lib/errors';

const SEX_OPTIONS = [
  ['F', 'Feminino'],
  ['M', 'Masculino'],
  ['O', 'Outro'],
] as const;

export function NewQuotation() {
  const navigate = useNavigate();
  const [pending, setPending] = useState(false);
  const [sex, setSex] = useState('F');
  const [form, setForm] = useState({
    passengerName: '',
    passengerCpf: '',
    passengerBirth: '',
    origin: 'BVB',
    destination: '',
    departureAt: '',
    returnAt: '',
    referenceFlight: '',
    referencePrice: '',
  });
  const set = (key: keyof typeof form) => (e: { target: { value: string } }) =>
    setForm({ ...form, [key]: e.target.value });

  async function onSubmit(e: FormEvent) {
    e.preventDefault();
    const cents = parseBRL(form.referencePrice);
    if (cents === null) {
      toast.error('Preço de referência inválido — use o formato 1.850,00');
      return;
    }
    setPending(true);
    try {
      const q = await api<{ id: string }>('/quotations', {
        method: 'POST',
        body: {
          passengerName: form.passengerName,
          passengerCpf: form.passengerCpf,
          passengerSex: sex,
          passengerBirth: form.passengerBirth,
          origin: form.origin.toUpperCase(),
          destination: form.destination.toUpperCase(),
          departureAt: new Date(form.departureAt).toISOString(),
          returnAt: form.returnAt ? new Date(form.returnAt).toISOString() : null,
          referenceFlight: form.referenceFlight,
          referencePriceCents: cents,
        },
      });
      toast.success('Rascunho criado. Revise e abra a disputa.');
      navigate(`/cotacoes/${q.id}`);
    } catch (err) {
      toast.error(errorMessage(err));
    } finally {
      setPending(false);
    }
  }

  return (
    <Layout>
      <Card className="mx-auto max-w-2xl">
        <CardHeader>
          <CardTitle>Nova demanda de passagem</CardTitle>
          <p className="text-sm text-muted-foreground">
            Dados do formulário do passageiro (Nome, CPF, Sexo, Nascimento) preenchem a Ordem de
            Serviço automaticamente.
          </p>
        </CardHeader>
        <CardContent>
          <form onSubmit={onSubmit} className="grid gap-3 md:grid-cols-2">
            <div className="space-y-1.5 md:col-span-2">
              <Label htmlFor="passengerName">Nome do passageiro</Label>
              <Input
                id="passengerName"
                value={form.passengerName}
                onChange={set('passengerName')}
                required
              />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="passengerCpf">CPF</Label>
              <Input
                id="passengerCpf"
                placeholder="000.000.000-00"
                inputMode="numeric"
                value={form.passengerCpf}
                onChange={set('passengerCpf')}
                required
              />
            </div>
            <div className="space-y-1.5">
              <Label>Sexo</Label>
              {/* This project's Select (Fluid Functionalism) renders its own value
                  display from `placeholder` — it takes no children/SelectValue, and
                  each SelectItem needs its list `index` for the proximity-hover overlay. */}
              <Select value={sex} onValueChange={setSex}>
                <SelectTrigger placeholder="Sexo" className="w-full" />
                <SelectContent>
                  {SEX_OPTIONS.map(([value, label], index) => (
                    <SelectItem key={value} index={index} value={value}>
                      {label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="passengerBirth">Nascimento</Label>
              <Input
                id="passengerBirth"
                type="date"
                value={form.passengerBirth}
                onChange={set('passengerBirth')}
                required
              />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="origin">Origem</Label>
              <Input id="origin" value={form.origin} onChange={set('origin')} required />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="destination">Destino</Label>
              <Input
                id="destination"
                placeholder="BSB"
                value={form.destination}
                onChange={set('destination')}
                required
              />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="departureAt">Embarque</Label>
              <Input
                id="departureAt"
                type="datetime-local"
                value={form.departureAt}
                onChange={set('departureAt')}
                required
              />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="returnAt">Retorno (opcional)</Label>
              <Input
                id="returnAt"
                type="datetime-local"
                value={form.returnAt}
                onChange={set('returnAt')}
              />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="referenceFlight">Voo de referência</Label>
              <Input
                id="referenceFlight"
                placeholder="LA-4001"
                value={form.referenceFlight}
                onChange={set('referenceFlight')}
                required
              />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="referencePrice">Preço de referência (R$)</Label>
              <Input
                id="referencePrice"
                inputMode="decimal"
                placeholder="1.850,00"
                value={form.referencePrice}
                onChange={set('referencePrice')}
                required
              />
              {form.referencePrice && (
                <p className="text-xs text-muted-foreground">
                  {parseBRL(form.referencePrice) !== null
                    ? `= ${formatBRL(parseBRL(form.referencePrice)!)}`
                    : 'Formato: 1.850,00'}
                </p>
              )}
              <p className="text-xs text-muted-foreground">
                🔒 Sigiloso — nunca é exibido aos fornecedores durante a disputa.
              </p>
            </div>
            <div className="md:col-span-2">
              <Button type="submit" disabled={pending} className="w-full md:w-auto">
                {pending ? 'Criando…' : 'Criar rascunho'}
              </Button>
            </div>
          </form>
        </CardContent>
      </Card>
    </Layout>
  );
}
