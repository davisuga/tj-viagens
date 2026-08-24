import { useState, type FormEvent } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { toast } from 'sonner';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { api } from '@/lib/api';
import { parseJwt, useAuth } from '@/lib/auth';

export function Login() {
  const { signIn } = useAuth();
  const navigate = useNavigate();
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [pending, setPending] = useState(false);

  async function onSubmit(e: FormEvent) {
    e.preventDefault();
    setPending(true);
    try {
      const res = await api<{ token: string }>('/auth/login', {
        method: 'POST',
        body: { email, password },
      });
      signIn(res.token);
      const user = parseJwt(res.token);
      navigate(user?.role === 'FORNECEDOR' ? '/fornecedor' : '/');
    } catch {
      toast.error('E-mail ou senha inválidos.');
    } finally {
      setPending(false);
    }
  }

  return (
    <div className="flex min-h-screen items-center justify-center bg-primary/95 p-4">
      <Card className="w-full max-w-sm">
        <CardHeader>
          <CardTitle className="text-2xl">TJ-Viagens</CardTitle>
          <p className="text-sm text-muted-foreground">
            Cotações competitivas de passagens aéreas · TJRR
          </p>
        </CardHeader>
        <CardContent>
          <form onSubmit={onSubmit} className="space-y-4">
            <div className="space-y-1.5">
              <Label htmlFor="email">E-mail</Label>
              <Input id="email" type="email" value={email} onChange={(e) => setEmail(e.target.value)} required autoFocus />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="password">Senha</Label>
              <Input id="password" type="password" value={password} onChange={(e) => setPassword(e.target.value)} required />
            </div>
            <Button type="submit" className="w-full" disabled={pending}>
              {pending ? 'Entrando…' : 'Entrar'}
            </Button>
            <p className="text-center text-sm text-muted-foreground">
              Agência de viagens ainda sem acesso?{' '}
              <Link to="/registro" className="text-primary underline">
                Solicite credenciamento
              </Link>
            </p>
          </form>
        </CardContent>
      </Card>
    </div>
  );
}
