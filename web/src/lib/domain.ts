export function isValidCnpj(input: string): boolean {
  const digits = input.replace(/\D/g, '');
  if (digits.length !== 14) return false;
  if (/^(\d)\1{13}$/.test(digits)) return false;
  const dv = (len: 12 | 13): number => {
    const weights =
      len === 12 ? [5, 4, 3, 2, 9, 8, 7, 6, 5, 4, 3, 2] : [6, 5, 4, 3, 2, 9, 8, 7, 6, 5, 4, 3, 2];
    const sum = weights.reduce((acc, w, i) => acc + w * Number(digits[i]), 0);
    const mod = sum % 11;
    return mod < 2 ? 0 : 11 - mod;
  };
  return dv(12) === Number(digits[12]) && dv(13) === Number(digits[13]);
}

export function parseBRL(input: string): number | null {
  const cleaned = input.replace(/[R$\s.]/g, '').replace(',', '.');
  if (!/^\d+(\.\d{1,2})?$/.test(cleaned)) return null;
  return Math.round(parseFloat(cleaned) * 100);
}

export function formatBRL(cents: number): string {
  return new Intl.NumberFormat('pt-BR', { style: 'currency', currency: 'BRL' }).format(cents / 100);
}

export function fmtCpf(cpf: string): string {
  const d = cpf.replace(/\D/g, '');
  if (d.length !== 11) return cpf;
  return `${d.slice(0, 3)}.${d.slice(3, 6)}.${d.slice(6, 9)}-${d.slice(9)}`;
}

export function fmtDateOnly(isoDate: string): string {
  const [y, m, d] = isoDate.split('-');
  return `${d}/${m}/${y}`;
}

/** UX rule 1: everything renders in Boa Vista local time, never raw UTC. */
export function fmtDateTime(iso: string): string {
  return new Intl.DateTimeFormat('pt-BR', {
    dateStyle: 'short',
    timeStyle: 'short',
    timeZone: 'America/Boa_Vista',
  }).format(new Date(iso));
}

/** pt-BR labels for the deterministic ticket-conference divergence codes
 *  (api/src/domain/divergence.rs). Unknown codes fall back to the raw code. */
const DIVERGENCE_LABELS: Record<string, string> = {
  PASSAGEIRO_DIVERGENTE: 'Nome do passageiro diverge do pedido',
  VALOR_DIVERGENTE: 'Valor emitido diverge da proposta vencedora',
  DATA_DIVERGENTE: 'Data de embarque diverge do pedido',
};

export function divergenceLabel(code: string): string {
  return DIVERGENCE_LABELS[code] ?? code;
}

export function serverOffsetMs(serverNowIso: string, clientNowMs: number): number {
  return new Date(serverNowIso).getTime() - clientNowMs;
}

export function remainingMs(deadlineIso: string, offsetMs: number, clientNowMs: number): number {
  return Math.max(0, new Date(deadlineIso).getTime() - (clientNowMs + offsetMs));
}

export function formatMmSs(ms: number): string {
  const totalS = Math.floor(ms / 1000);
  const mm = String(Math.floor(totalS / 60)).padStart(2, '0');
  const ss = String(totalS % 60).padStart(2, '0');
  return `${mm}:${ss}`;
}
