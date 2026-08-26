import { createContext, useContext, useState, type ReactNode } from 'react';
import { Navigate } from 'react-router-dom';
import { getToken, setToken } from './api';

export type SessionUser = {
  sub: string;
  name: string;
  role: 'ADMIN' | 'SERVIDOR' | 'FORNECEDOR';
  supplierId: string | null;
};

export function parseJwt(token: string): SessionUser | null {
  try {
    const payload = JSON.parse(atob(token.split('.')[1]));
    if (typeof payload.exp === 'number' && payload.exp * 1000 < Date.now()) return null;
    return {
      sub: payload.sub,
      name: payload.name,
      role: payload.role,
      supplierId: payload.supplier_id ?? null,
    };
  } catch {
    return null;
  }
}

type AuthCtx = { user: SessionUser | null; signIn: (token: string) => void; signOut: () => void };

const Ctx = createContext<AuthCtx>({ user: null, signIn: () => {}, signOut: () => {} });

export function AuthProvider({ children }: { children: ReactNode }) {
  const [user, setUser] = useState<SessionUser | null>(() => {
    const t = getToken();
    return t ? parseJwt(t) : null;
  });
  const signIn = (token: string) => {
    setToken(token);
    setUser(parseJwt(token));
  };
  const signOut = () => {
    setToken(null);
    setUser(null);
  };
  return <Ctx.Provider value={{ user, signIn, signOut }}>{children}</Ctx.Provider>;
}

export function useAuth(): AuthCtx {
  return useContext(Ctx);
}

/** Landing page for a signed-in user's role. */
export function roleHome(user: SessionUser): string {
  return user.role === 'FORNECEDOR' ? '/fornecedor' : '/';
}

export function RequireRole({
  roles,
  children,
}: {
  roles: SessionUser['role'][];
  children: ReactNode;
}) {
  const { user } = useAuth();
  if (!user) return <Navigate to="/login" replace />;
  // Wrong area for this role: send to the role's own home, not to the login
  // form — the session is still valid.
  if (!roles.includes(user.role)) return <Navigate to={roleHome(user)} replace />;
  return <>{children}</>;
}
