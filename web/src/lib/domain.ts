import type { AwardReason, DocStatus, DocType, Itinerary } from '@/lib/types';

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

/** 340 -> "5h40", 185 -> "3h05", 720 -> "12h" (mirrors api/src/domain/itinerary.rs). */
export function fmtDuration(minutes: number): string {
  const hours = Math.floor(minutes / 60);
  const rest = minutes % 60;
  return rest === 0 ? `${hours}h` : `${hours}h${String(rest).padStart(2, '0')}`;
}

export function fmtConnections(connections: number): string {
  if (connections === 0) return 'direto';
  if (connections === 1) return '1 conexão';
  return `${connections} conexões`;
}

/** "5:40" or "5h40" -> 340; null when it does not read as hours:minutes. */
export function parseDuration(input: string): number | null {
  const match = /^\s*(\d{1,2})\s*[:h]\s*([0-5]\d)?\s*$/i.exec(input);
  if (!match) return null;
  return Number(match[1]) * 60 + Number(match[2] ?? 0);
}

/** ISO instant -> value for <input type="datetime-local"> in Boa Vista time. */
export function toBoaVistaLocalInput(iso: string): string {
  const local = new Date(new Date(iso).getTime() - 4 * 3600 * 1000);
  return local.toISOString().slice(0, 16);
}

export const DOC_LABELS: Record<DocType, string> = {
  CONTRATO_SOCIAL: 'Contrato social',
  CND_FEDERAL: 'CND Federal',
  CRF_FGTS: 'CRF do FGTS',
  CNDT: 'CNDT',
};

/** What is wrong (or about to be) with one document; null when it is fine. */
export function docIssue(doc: DocStatus): { level: 'warn' | 'block'; text: string } | null {
  const label = DOC_LABELS[doc.docType];
  switch (doc.state) {
    case 'OK':
      return null;
    case 'EXPIRING':
      return {
        level: 'warn',
        text:
          doc.daysLeft === 0
            ? `${label} vence hoje`
            : `${label} vence em ${doc.daysLeft} dia${doc.daysLeft === 1 ? '' : 's'}`,
      };
    case 'EXPIRED':
      return {
        level: 'block',
        text: `${label} vencida em ${doc.validUntil ? fmtDateOnly(doc.validUntil) : '—'}`,
      };
    case 'MISSING':
      return { level: 'block', text: `${label} ausente` };
  }
}

export const AWARD_REASON_LABELS: Record<AwardReason, string> = {
  MENOR_PRECO: 'Menor preço entre as propostas habilitadas',
  MENOR_DURACAO: 'Viagem mais curta que a da proposta de menor preço',
  MENOS_CONEXOES: 'Menos conexões que a proposta de menor preço',
  HORARIO_COMPATIVEL: 'Horário compatível com o compromisso',
  OUTRO: 'Outro motivo',
};

/** What a pricier pick buys over the cheapest eligible bid, in plain words. */
export function tradeOffLine(pick: Itinerary & { totalPriceCents: number }, cheapest: Itinerary & { totalPriceCents: number }): string {
  const delta = pick.totalPriceCents - cheapest.totalPriceCents;
  const pct = ((delta * 100) / cheapest.totalPriceCents).toFixed(1).replace('.', ',');
  const parts = [delta === 0 ? 'Mesmo preço da mais barata' : `${formatBRL(delta)} (+${pct}%) acima da mais barata`];
  if (pick.durationMinutes !== null && cheapest.durationMinutes !== null) {
    const diff = cheapest.durationMinutes - pick.durationMinutes;
    if (diff > 0) parts.push(`${fmtDuration(diff)} a menos de viagem`);
    else if (diff < 0) parts.push(`${fmtDuration(-diff)} a mais de viagem`);
  }
  if (pick.connections !== null && cheapest.connections !== null && pick.connections !== cheapest.connections) {
    parts.push(`${fmtConnections(pick.connections)} contra ${fmtConnections(cheapest.connections)}`);
  }
  return parts.join(' · ');
}

/** Reasons the API accepts for this pick (mirrors api/src/domain/award.rs):
 *  the cheapest eligible bid is justified by price; a pricier one needs a claim
 *  that holds against the cheapest, or a free-text reason. */
export function acceptableReasons(
  pick: Itinerary & { totalPriceCents: number },
  cheapest: Itinerary & { totalPriceCents: number },
): AwardReason[] {
  if (pick.totalPriceCents === cheapest.totalPriceCents) return ['MENOR_PRECO'];
  const reasons: AwardReason[] = [];
  if (pick.durationMinutes !== null && cheapest.durationMinutes !== null && pick.durationMinutes < cheapest.durationMinutes) {
    reasons.push('MENOR_DURACAO');
  }
  if (pick.connections !== null && cheapest.connections !== null && pick.connections < cheapest.connections) {
    reasons.push('MENOS_CONEXOES');
  }
  reasons.push('HORARIO_COMPATIVEL', 'OUTRO');
  return reasons;
}
