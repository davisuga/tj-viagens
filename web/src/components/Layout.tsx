import type { ReactNode } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { Button } from '@/components/ui/button';
import { useAuth } from '@/lib/auth';

export function Layout({ children, back = false }: { children: ReactNode; back?: boolean }) {
  const { user, signOut } = useAuth();
  const navigate = useNavigate();
  const home = user?.role === 'FORNECEDOR' ? '/fornecedor' : '/';
  return (
    <div className="min-h-screen bg-muted/40">
      <header className="border-b bg-background">
        <div className="mx-auto flex max-w-5xl items-center justify-between p-3 md:p-4">
          <Link to={home} className="text-lg font-bold text-primary">
            TJ-Viagens <span className="text-sm font-normal text-muted-foreground">· TJRR</span>
          </Link>
          {user && (
            <div className="flex items-center gap-3 text-sm">
              <span className="hidden text-muted-foreground md:inline">{user.name}</span>
              {/* Fluid Functionalism Button has no "outline" variant (primary/secondary/tertiary/ghost) —
                  "tertiary" is its bordered/transparent equivalent. */}
              <Button variant="tertiary" size="sm" onClick={signOut}>
                Sair
              </Button>
            </div>
          )}
        </div>
      </header>
      <main className="mx-auto max-w-5xl p-3 md:p-4">
        {back && (
          <Button
            variant="ghost"
            size="sm"
            className="-ml-3 mb-2"
            onClick={() => {
              // react-router stamps an idx on each in-app history entry; at
              // idx 0 (deep link / fresh tab) "back" would leave the app, so
              // fall back to the role home instead.
              if ((window.history.state?.idx ?? 0) > 0) navigate(-1);
              else navigate(home);
            }}
          >
            ← Voltar
          </Button>
        )}
        {children}
      </main>
    </div>
  );
}
