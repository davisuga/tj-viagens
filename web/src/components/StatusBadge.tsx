import { Badge } from '@/components/ui/badge';

const LABELS: Record<string, string> = {
  DRAFT: 'Rascunho',
  OPEN: 'Aberta',
  CLOSED: 'Encerrada',
  AWARDED: 'Adjudicada',
  TICKETED: 'Bilhete enviado',
  COMPLETED: 'Concluída',
  PENDING: 'Pendente',
  ACTIVE: 'Credenciado',
  REJECTED: 'Rejeitado',
  SUSPENDED: 'Suspenso',
};

const CLASSES: Record<string, string> = {
  DRAFT: 'bg-slate-200 text-slate-700',
  OPEN: 'bg-green-100 text-green-800',
  CLOSED: 'bg-amber-100 text-amber-800',
  AWARDED: 'bg-blue-100 text-blue-800',
  TICKETED: 'bg-violet-100 text-violet-800',
  COMPLETED: 'bg-emerald-100 text-emerald-800',
  PENDING: 'bg-amber-100 text-amber-800',
  ACTIVE: 'bg-emerald-100 text-emerald-800',
  REJECTED: 'bg-red-100 text-red-800',
  SUSPENDED: 'bg-slate-200 text-slate-700',
};

export function StatusBadge({ status }: { status: string }) {
  return (
    <Badge className={CLASSES[status] ?? 'bg-slate-200 text-slate-700'} variant="secondary">
      {LABELS[status] ?? status}
    </Badge>
  );
}
