"use client";

import { STATUS_COLOR, STATUS_LABEL, type Metric } from "@/lib/metrics";

export default function MetricCard({
  metric,
  onClick,
}: {
  metric: Metric;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className="group relative flex flex-col gap-1.5 border border-[var(--border)] bg-[var(--bg-surface)] p-3 text-left transition-colors hover:bg-[var(--bg-surface-hover)] focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--text-secondary)]"
      style={{ borderRadius: 8 }}
      aria-label={`${metric.label}, ${metric.value}, status ${STATUS_LABEL[metric.status]}. Click to explain.`}
    >
      <div className="flex items-center justify-between gap-2">
        <div className="flex items-center gap-1.5 min-w-0">
          <span
            aria-hidden
            className="inline-block h-[7px] w-[7px] shrink-0 rounded-full"
            style={{ backgroundColor: STATUS_COLOR[metric.status] }}
          />
          <span className="truncate text-[12px] text-[var(--text-secondary)]">
            {metric.label}
          </span>
        </div>
        <span
          aria-hidden
          className="text-[10px] font-medium text-[var(--text-tertiary)] opacity-0 transition-opacity group-hover:opacity-100"
          title="Explain"
        >
          i
        </span>
      </div>
      <span className="text-[22px] font-medium tabular-nums text-[var(--text-primary)]">
        {metric.value}
      </span>
      <span className="text-[11.5px] leading-snug text-[var(--text-tertiary)]">
        {metric.context}
      </span>
      <span className="sr-only">Status: {STATUS_LABEL[metric.status]}</span>
    </button>
  );
}
