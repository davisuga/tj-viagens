const API = import.meta.env.VITE_API_URL ?? 'http://localhost:3001';

export function getToken(): string | null {
  return localStorage.getItem('tj_token');
}

export function setToken(token: string | null): void {
  if (token === null) localStorage.removeItem('tj_token');
  else localStorage.setItem('tj_token', token);
}

export function apiUrl(path: string): string {
  return `${API}${path}`;
}

export async function api<T>(
  path: string,
  opts: { method?: string; body?: unknown; form?: FormData } = {},
): Promise<T> {
  const headers: Record<string, string> = {};
  const token = getToken();
  if (token) headers.Authorization = `Bearer ${token}`;
  let body: BodyInit | undefined;
  if (opts.form) {
    body = opts.form;
  } else if (opts.body !== undefined) {
    headers['Content-Type'] = 'application/json';
    body = JSON.stringify(opts.body);
  }
  const res = await fetch(apiUrl(path), { method: opts.method ?? 'GET', headers, body });
  if (res.status === 401) {
    setToken(null);
    if (!location.pathname.startsWith('/login')) location.href = '/login';
  }
  if (!res.ok) {
    const detail = (await res.json().catch(() => ({}))) as { error?: string };
    throw new Error(detail.error ?? `HTTP ${res.status}`);
  }
  return res.json() as Promise<T>;
}

/** Opens a printable page (OS / relatório) in a new tab, authenticated via ?token=. */
export function openPage(path: string): void {
  window.open(`${apiUrl(path)}?token=${getToken()}`, '_blank');
}

/** SSE subscription with react-query invalidation on events. Returns cleanup. */
export function subscribeQuotation(id: string, onEvent: (event: string, data: unknown) => void): () => void {
  const source = new EventSource(`${apiUrl(`/quotations/${id}/events`)}?token=${getToken()}`);
  for (const name of ['hello', 'tick', 'status', 'proposal']) {
    source.addEventListener(name, (e) => onEvent(name, JSON.parse((e as MessageEvent).data)));
  }
  source.onerror = () => {
    // Any error means live updates are stale (the browser may still be
    // auto-retrying); consumers surface this and fall back to polling.
    onEvent('down', {});
    if (source.readyState === EventSource.CLOSED) onEvent('closed', {});
  };
  return () => source.close();
}
