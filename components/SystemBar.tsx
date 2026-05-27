export function SystemBar() {
  return (
    <div className="flex items-center gap-6 px-6 py-2 border-t border-[var(--border)] bg-[var(--bg-surface)] mono text-[10.5px] tracking-[0.06em] text-[var(--text-tertiary)]">
      <span className="flex items-center gap-1.5">
        <span className="dot live" />
        <span className="text-[var(--accent-green)]">all systems nominal</span>
      </span>
      <span className="text-[var(--text-disabled)]">&middot;</span>
      <span>binance ws <span className="text-[var(--accent-green)]">connected</span></span>
      <span className="text-[var(--text-disabled)]">&middot;</span>
      <span>fred <span className="text-[var(--accent-green)]">fresh</span></span>
      <span className="ml-auto">market monitor</span>
    </div>
  );
}
