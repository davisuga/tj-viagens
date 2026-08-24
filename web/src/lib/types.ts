export type ChecklistInfo = { missing: string[]; expired: string[]; ok: boolean };

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

export type ProposalInfo = {
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

export type StaffQuotation = QuotationBase & {
  passenger: { name: string; cpf: string; sex: string; birth: string };
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

export type RankingRow = {
  position: number;
  proposalId: string;
  supplier: { id: string; legalName: string; cnpj: string };
  totalPriceCents: number;
  flightInfo: string;
  notes: string | null;
  submittedAt: string;
  deltaFromReferenceCents: number;
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
  economy: { saved_cents: number; saved_pct: number } | null;
  serviceOrder: { number: string } | null;
  ticket: { fileName: string; late: boolean; divergences: string[] } | null;
};
