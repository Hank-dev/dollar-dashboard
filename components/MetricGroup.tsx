"use client";

import type { Metric } from "@/lib/metrics";
import MetricCard from "./MetricCard";
import { GroupIcon } from "./Dashboard";

export default function MetricGroup({
  title,
  icon,
  metrics,
  onSelect,
}: {
  title: string;
  icon: string;
  metrics: Metric[];
  onSelect: (m: Metric) => void;
}) {
  return (
    <section>
      <h2 className="mb-3 flex items-center gap-2 text-[12px] font-semibold uppercase tracking-[0.1em] text-[var(--text-secondary)]">
        <GroupIcon name={icon} />
        {title}
      </h2>
      <div
        className="grid gap-2.5"
        style={{ gridTemplateColumns: "repeat(auto-fit, minmax(150px, 1fr))" }}
      >
        {metrics.map((m) => (
          <MetricCard key={m.id} metric={m} onClick={() => onSelect(m)} />
        ))}
      </div>
    </section>
  );
}
