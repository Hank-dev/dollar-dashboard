import type { ReactNode } from "react";

export function MetricCard({
  label,
  value,
  sub,
  tone = "neutral",
}: {
  label: string;
  value: ReactNode;
  sub?: ReactNode;
  tone?: "up" | "down" | "warn" | "neutral";
}) {
  const toneCls =
    tone === "up"
      ? "text-[var(--accent-green)]"
      : tone === "down"
        ? "text-[var(--accent-red)]"
        : tone === "warn"
          ? "text-[var(--accent-amber)]"
          : "text-[var(--text-primary)]";
  return (
    <div className="px-3 py-2 flex flex-col gap-0.5 border-b border-[var(--border)] last:border-b-0">
      <div className="mono text-[10px] tracking-widest uppercase text-[var(--text-tertiary)]">
        {label}
      </div>
      <div className={`mono text-xl tnum ${toneCls}`}>{value}</div>
      {sub ? (
        <div className="mono text-[11px] text-[var(--text-secondary)]">
          {sub}
        </div>
      ) : null}
    </div>
  );
}
