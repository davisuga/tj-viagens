import { useEffect, useMemo, useState } from 'react';
import { formatMmSs, remainingMs, serverOffsetMs } from '@/lib/domain';

export function Countdown({
  deadline,
  serverNow,
  onExpire,
  size = 'lg',
}: {
  deadline: string;
  serverNow: string;
  onExpire?: () => void;
  size?: 'lg' | 'sm';
}) {
  const offset = useMemo(() => serverOffsetMs(serverNow, Date.now()), [serverNow]);
  const [ms, setMs] = useState(() => remainingMs(deadline, offset, Date.now()));
  useEffect(() => {
    const timer = setInterval(() => {
      const next = remainingMs(deadline, offset, Date.now());
      setMs(next);
      if (next === 0) {
        clearInterval(timer);
        onExpire?.();
      }
    }, 250);
    return () => clearInterval(timer);
  }, [deadline, offset, onExpire]);
  const urgent = ms > 0 && ms < 60_000;
  return (
    <span
      className={`font-mono font-bold tabular-nums ${size === 'lg' ? 'text-4xl' : 'text-lg'} ${
        urgent ? 'text-red-600 animate-pulse' : 'text-primary'
      }`}
      aria-live="polite"
    >
      {formatMmSs(ms)}
    </span>
  );
}
