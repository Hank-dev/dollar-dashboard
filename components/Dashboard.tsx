"use client";

import { useState } from "react";
import {
  GROUPS,
  STATUS_COLOR,
  STATUS_LABEL,
  type DashboardData,
  type Metric,
  type Status,
} from "@/lib/metrics";
import MetricGroup from "./MetricGroup";
import YieldCurveChart from "./YieldCurveChart";
import ExplainDrawer from "./ExplainDrawer";
import AskBox from "./AskBox";

const STATUS_ORDER: Status[] = ["calm", "neutral", "elevated", "stressed"];

const DATE_FMT = new Intl.DateTimeFormat("en-GB", {
  day: "numeric",
  month: "short",
  year: "numeric",
  timeZone: "UTC",
});

export default function Dashboard({ data }: { data: DashboardData }) {
  const [openMetric, setOpenMetric] = useState<Metric | null>(null);
  const displayDate = DATE_FMT.format(
    new Date(data.snapshotDate + "T00:00:00Z"),
  );

  return (
    <main className="mx-auto max-w-[720px] px-5 py-10 sm:py-12">
      {/* Header */}
      <header className="flex items-baseline justify-between gap-4 border-b border-[var(--border)] pb-5">
        <h1 className="text-[17px] font-semibold tracking-tight text-[var(--text-primary)]">
          Dollar &amp; global financial system monitor
        </h1>
        <p className="shrink-0 text-[11.5px] font-medium uppercase tracking-wide text-[var(--text-tertiary)]">
          Close · {displayDate}
        </p>
      </header>

      {/* Verdict banner */}
      <section
        aria-label="Verdict"
        className="mt-6 border border-[var(--border)] bg-[var(--bg-surface)] p-4 sm:p-5"
        style={{ borderRadius: 8, borderLeft: "3px solid var(--accent-amber)" }}
      >
        <p className="text-[11px] font-semibold uppercase tracking-[0.12em] text-[var(--accent-amber)]">
          Verdict — two markets, two answers
        </p>
        <p className="mt-2 text-[13.5px] leading-relaxed text-[var(--text-primary)]">
          Equities and volatility say calm: the S&amp;P sits near records, the VIX is subdued.
          Rates and the dollar&apos;s funding plumbing say stress: the 30-year yield is at a
          19-year high while Japan, the world&apos;s marginal lender, withdraws. The open
          question is which side converges to the other.
        </p>
      </section>

      {/* Legend */}
      <div className="mt-6 flex flex-wrap items-center gap-x-5 gap-y-2 text-[11.5px] text-[var(--text-secondary)]">
        {STATUS_ORDER.map((s) => (
          <span key={s} className="inline-flex items-center gap-2">
            <span
              aria-hidden
              className="inline-block h-[7px] w-[7px] rounded-full"
              style={{ backgroundColor: STATUS_COLOR[s] }}
            />
            {STATUS_LABEL[s]}
          </span>
        ))}
      </div>

      {/* Metric groups */}
      <div className="mt-7 space-y-7">
        {GROUPS.map((g) => (
          <MetricGroup
            key={g.id}
            title={g.title}
            icon={g.icon}
            metrics={data.metrics.filter((m) => m.group === g.id)}
            onSelect={setOpenMetric}
          />
        ))}
      </div>

      {/* Yield curve */}
      <section className="mt-9">
        <h2 className="mb-3 flex items-center gap-2 text-[12px] font-semibold uppercase tracking-[0.1em] text-[var(--text-secondary)]">
          <GroupIcon name="trending-up" />
          US Treasury yield curve
        </h2>
        <div className="border border-[var(--border)] bg-[var(--bg-surface)] p-4" style={{ borderRadius: 8 }}>
          <YieldCurveChart points={data.yieldCurve} />
        </div>
      </section>

      {/* AskBox */}
      <section className="mt-9">
        <h2 className="mb-3 flex items-center gap-2 text-[12px] font-semibold uppercase tracking-[0.1em] text-[var(--text-secondary)]">
          <GroupIcon name="message" />
          Ask the dashboard
        </h2>
        <AskBox />
      </section>

      {/* Footer */}
      <footer className="mt-10 border-t border-[var(--border)] pt-5 text-[11.5px] leading-relaxed text-[var(--text-tertiary)]">
        <p>
          Sources: US Treasury &amp; FRED (yields, Fed funds), ICE (DXY), Reuters / Nikkei
          (USD/JPY, JGB, BOJ), CBOE (VIX), S&amp;P Global (S&amp;P 500), LBMA (gold), ICE
          Brent.
        </p>
        <p className="mt-2">
          Caveat: the Japan 10Y JGB figure (~2.8%) rests on a single recent source — moderate
          confidence. All other figures are corroborated. This dashboard is informational
          market context, not financial advice.
        </p>
      </footer>

      <ExplainDrawer
        metric={openMetric}
        onClose={() => setOpenMetric(null)}
      />
    </main>
  );
}

// Tiny inline icons (lucide-style strokes) so we don't add a runtime dep.
export function GroupIcon({ name }: { name: string }) {
  const common = {
    width: 14,
    height: 14,
    viewBox: "0 0 24 24",
    fill: "none",
    stroke: "currentColor",
    strokeWidth: 1.75,
    strokeLinecap: "round" as const,
    strokeLinejoin: "round" as const,
    "aria-hidden": true,
  };
  switch (name) {
    case "building-bank":
      return (
        <svg {...common}>
          <path d="M3 21h18" />
          <path d="M3 10h18" />
          <path d="M12 3 2 8h20Z" />
          <path d="M5 10v8" />
          <path d="M10 10v8" />
          <path d="M14 10v8" />
          <path d="M19 10v8" />
        </svg>
      );
    case "currency-dollar":
      return (
        <svg {...common}>
          <line x1="12" y1="2" x2="12" y2="22" />
          <path d="M17 5H9.5a3.5 3.5 0 0 0 0 7h5a3.5 3.5 0 0 1 0 7H6" />
        </svg>
      );
    case "activity":
      return (
        <svg {...common}>
          <path d="M22 12h-4l-3 9L9 3l-3 9H2" />
        </svg>
      );
    case "trending-up":
      return (
        <svg {...common}>
          <polyline points="22 7 13.5 15.5 8.5 10.5 2 17" />
          <polyline points="16 7 22 7 22 13" />
        </svg>
      );
    case "message":
      return (
        <svg {...common}>
          <path d="M21 11.5a8.38 8.38 0 0 1-.9 3.8 8.5 8.5 0 0 1-7.6 4.7 8.38 8.38 0 0 1-3.8-.9L3 21l1.9-5.7a8.38 8.38 0 0 1-.9-3.8 8.5 8.5 0 0 1 4.7-7.6 8.38 8.38 0 0 1 3.8-.9h.5a8.48 8.48 0 0 1 8 8v.5z" />
        </svg>
      );
    default:
      return null;
  }
}
