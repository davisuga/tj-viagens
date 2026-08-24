import { useState, type FormEvent } from 'react';
import { Link } from 'react-router-dom';
import { toast } from 'sonner';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { api } from '@/lib/api';
import { isValidCnpj } from '@/lib/domain';
import { errorMessage } from '@/lib/errors';

export function Register() {
  const [form, setForm] = useState({
    cnpj: '',
    legalName: '',
    contactEmail: '',
    phone: '',
    userName: '',
    password: '',
  });
  const [done, setDone] = useState(false);
  const [pending, setPending] = useState(false);
  const set = (key: keyof typeof form) => (e: { target: { value: string } }) =>
    setForm({ ...form, [key]: e.target.value });

  async function onSubmit(e: FormEvent) {
    e.preventDefault();
    if (!isValidCnpj(form.cnpj)) {
      toast.error('CNPJ inválido — confira os dígitos.');
      return;
    }
    setPending(true);
    try {
      await api('/suppliers/register', {
        method: 'POST',
        body: { ...form, phone: form.phone || null },
      });
      setDone(true);
    } catch (err) {
      toast.error(errorMessage(err));
    } finally {
      setPending(false);
    }
  }

  if (done) {
    return (
      <div className="flex min-h-screen items-center justify-center bg-primary/95 p-4">
        <Card className="w-full max-w-md text-center">
          <CardHeader>
            <CardTitle>Solicitação enviada ✔</CardTitle>
          </CardHeader>
          <CardContent className="space-y-3">
            <p className="text-sm text-muted-foreground">
              Entre com seu e-mail e senha para enviar os documentos obrigatórios e acompanhar a
              análise do credenciamento.
            </p>
            <Button asChild className="w-full">
              <Link to="/login">Ir para o login</Link>
            </Button>
          </CardContent>
        </Card>
      </div>
    );
  }

  const fields: Array<[keyof typeof form, string, string]> = [
    ['cnpj', 'CNPJ', '00.000.000/0000-00'],
    ['legalName', 'Razão social', 'Agência Exemplo Viagens LTDA'],
    ['contactEmail', 'E-mail de contato', 'contato@agencia.com.br'],
    ['phone', 'Telefone (opcional)', '(95) 99999-0000'],
    ['userName', 'Nome do responsável', 'Nome completo'],
    ['password', 'Senha (mínimo 8 caracteres)', ''],
  ];

  return (
    <div className="flex min-h-screen items-center justify-center bg-primary/95 p-4">
      <Card className="w-full max-w-md">
        <CardHeader>
          <CardTitle>Credenciamento de fornecedor</CardTitle>
          <p className="text-sm text-muted-foreground">
            Agências de viagens e companhias aéreas — credenciamento permanente do TJRR.
          </p>
        </CardHeader>
        <CardContent>
          <form onSubmit={onSubmit} className="space-y-3">
            {fields.map(([key, label, placeholder]) => (
              <div key={key} className="space-y-1.5">
                <Label htmlFor={key}>{label}</Label>
                <Input
                  id={key}
                  type={key === 'password' ? 'password' : key === 'contactEmail' ? 'email' : 'text'}
                  inputMode={key === 'cnpj' || key === 'phone' ? 'numeric' : undefined}
                  placeholder={placeholder}
                  value={form[key]}
                  onChange={set(key)}
                  required={key !== 'phone'}
                  minLength={key === 'password' ? 8 : undefined}
                />
              </div>
            ))}
            <Button type="submit" className="w-full" disabled={pending}>
              {pending ? 'Enviando…' : 'Solicitar credenciamento'}
            </Button>
          </form>
        </CardContent>
      </Card>
    </div>
  );
}
