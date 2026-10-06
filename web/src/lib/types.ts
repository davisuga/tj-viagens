export type DocType = 'CONTRATO_SOCIAL' | 'CND_FEDERAL' | 'CRF_FGTS' | 'CNDT';
export type DocState = 'OK' | 'EXPIRING' | 'EXPIRED' | 'MISSING';

/** One required document as of today (Boa Vista). `validUntil` is null for
 *  documents without expiry (contrato social) and for missing ones. */
export type DocStatus = {
  docType: DocType;
  validUntil: string | null;
  state: DocState;
  daysLeft: number | null;
};

export type ChecklistInfo = { missing: string[]; expired: string[]; ok: boolean; docs: DocStatus[] };

export type SupplierInfo = {
  id: string;
  cnpj: string;
  legalName: string;
  contactEmail: string;
  phone: string | null;
  status: string;
  statusReason: string | null;
};

export type SupplierMe = { supplier: SupplierInfo; checklist: ChecklistInfo };

export type NotificationItem = {
  id: string;
  quotationId: string | null;
  kind: string;
  message: string;
  createdAt: string;
};

/** Structured itinerary — null on bids recorded before it existed. */
export type Itinerary = {
  airline: string | null;
  departsAt: string | null;
  durationMinutes: number | null;
  connections: number | null;
};

export type ProposalInfo = Itinerary & {
  id: string;
  supplierId: string;
  totalPriceCents: number;
  flightInfo: string;
  notes: string | null;
  submittedAt: string;
};

export type QuotationBase = {
  id: string;
  code: string;
  status: string;
  origin: string;
  destination: string;
  departureAt: string;
  returnAt: string | null;
  referenceFlight: string;
  opensAt: string | null;
  closesAt: string | null;
  serverNow: string;
};

export type SupplierQuotation = QuotationBase & {
  myProposal: ProposalInfo | null;
  isWinner: boolean;
  passenger?: { name: string; cpf: string; sex: string; birth: string };
  ticketDeadlineAt?: string | null;
};

export type AwardReason =
  | 'MENOR_PRECO'
  | 'MENOR_DURACAO'
  | 'MENOS_CONEXOES'
  | 'HORARIO_COMPATIVEL'
  | 'OUTRO';

export type StaffQuotation = QuotationBase & {
  passenger: { name: string; cpf: string; sex: string; birth: string };
  seiProcess: string | null;
  awardReason: AwardReason | null;
  referencePriceCents: number;
  awardedProposalId: string | null;
  awardedAt: string | null;
  awardJustification: string | null;
  ticketDeadlineAt: string | null;
  proposals: { count: number } | ProposalInfo[];
};

export function proposalsCount(p: StaffQuotation['proposals']): number {
  return Array.isArray(p) ? p.length : p.count;
}

export type RankingRow = Itinerary & {
  position: number;
  proposalId: string;
  supplier: { id: string; legalName: string; cnpj: string };
  totalPriceCents: number;
  flightInfo: string;
  notes: string | null;
  submittedAt: string;
  deltaFromReferenceCents: number;
  /** Eligibility right now: credenciado and every certidão valid. */
  habilitacao: { eligible: boolean; supplierStatus: string | null; docs: DocStatus[] };
};

export type Metrics = {
  awardedCount: number;
  totalSavedCents: number;
  avgParticipants: number;
  ticketsOnTimePct: number;
};

export type SupplierListItem = { supplier: SupplierInfo; checklist: ChecklistInfo };

export type Report = {
  quotation: { code: string; status: string; passengerName: string; passengerCpfMasked: string };
  notifiedSuppliers: number;
  economy: { saved_cents: number; saved_pct: number } | null;
  serviceOrder: { number: string } | null;
  ticket: {
    fileName: string;
    late: boolean;
    divergences: string[];
    uploadedAt: string;
    confirmedAt: string | null;
  } | null;
};
