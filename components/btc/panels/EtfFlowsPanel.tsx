"use client";

import useSWR from "swr";
import { Panel } from "@/components/btc/layout/Panel";
import { EtfFlowsChart } from "@/components/btc/charts/EtfFlowsChart";
import type { EtfFlowsResponse } from "@/lib/btc/types";

const fetcher = (u: string) =>
  fetch(u).then((r) => {
    if (!r.ok) {
      return r.json().then((j) => {
        throw new Error(j?.error ?? `${r.status}`);
      });
    }
    return r.json();
  });

function fmtM(v: number | null | undefined, digits = 0): string {
  if (v == null || !Number.isFinite(v)) return "—";
  const sign = v > 0 ? "+" : "";
  return `${sign}${v.toLocaleString("en-US", {
    minimumFractionDigits: digits,
    maximumFractionDigits: digits,
  })}M`;
}

function fmtB(v: number | null | undefined, digits = 1): string {
  if (v == null || !Number.isFinite(v)) return "—";
  const sign = v > 0 ? "+" : "";
  return `${sign}${(v / 1000).toFixed(digits)}B`;
}

export function EtfFlowsPanel() {
  const { data, error } = useSWR<EtfFlowsResponse>("/api/etf-flows", fetcher, {
    refreshInterval: 6 * 60 * 60 * 1000,
  });

  const right = data
    ? `cum ${fmtB(data.cumulativeTotal)} · 7d avg ${fmtM(data.flow7dAvg)}/d`
    : "";

  return (
    <Panel title="SPOT BTC ETF NET FLOWS ($M)" right={right} className="h-full">
      {error ? (
        <div className="p-3 mono text-[11px] text-[var(--accent-red)] whitespace-pre-wrap">
          ETF flow fetch failed: {String(error.message ?? error)}
        </div>
      ) : !data ? (
        <div className="p-3 mono text-[12px] text-[var(--text-tertiary)]">
          loading…
        </div>
      ) : (
        <div className="flex flex-col h-full">
          <div className="flex border-b border-[var(--border)] flex-shrink-0">
            <Stat label="latest day" value={fmtM(data.latestTotal)} />
            <Stat label="7d avg" value={`${fmtM(data.flow7dAvg)}/d`} />
            <Stat label="30d avg" value={`${fmtM(data.flow30dAvg)}/d`} />
            <Stat label="cumulative" value={fmtB(data.cumulativeTotal)} />
          </div>
          <div className="flex-1 min-h-0">
            <EtfFlowsChart data={data} />
          </div>
        </div>
      )}
    </Panel>
  );
}

function Stat({ label, value }: { label: string; value: string }) {
  const color = value.startsWith("+")
    ? "var(--accent-green)"
    : value.startsWith("-")
      ? "var(--accent-red)"
      : "var(--text-primary)";
  return (
    <div className="px-3 py-2 flex-1 border-r border-[var(--border)] last:border-r-0">
      <div className="mono text-[10px] tracking-widest uppercase text-[var(--text-tertiary)]">
        {label}
      </div>
      <div className="mono text-base tnum" style={{ color }}>
        {value}
      </div>
    </div>
  );
}
