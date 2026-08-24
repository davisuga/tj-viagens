import type { ReactNode } from 'react';
import { Link } from 'react-router-dom';
import { Button } from '@/components/ui/button';
import { useAuth } from '@/lib/auth';

export function Layout({ children }: { children: ReactNode }) {
  const { user, signOut } = useAuth();
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
      <main className="mx-auto max-w-5xl p-3 md:p-4">{children}</main>
    </div>
  );
}
