/** Shown when the SSE stream drops. The claim is accurate: every page that
 *  subscribes also polls (refetchInterval), so data stays fresh — just slower. */
export function LivePill() {
  return (
    <span className="inline-flex items-center rounded-full bg-amber-100 px-2 py-0.5 text-xs font-normal text-amber-800">
      Atualização ao vivo indisponível — atualizando periodicamente
    </span>
  );
}
