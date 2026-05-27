"use client";

import useSWR from "swr";
import { Panel } from "@/components/btc/layout/Panel";
import type { CorrelationAsset, CorrelationsResponse } from "@/lib/btc/types";

const fetcher = (u: string) =>
  fetch(u).then((r) => {
    if (!r.ok) {
      return r.json().then((j) => {
        throw new Error(j?.error ?? `${r.status}`);
      });
    }
    return r.json();
  });

function fmtCorr(v: number | null): string {
  if (v == null || !Number.isFinite(v)) return "—";
  const sign = v > 0 ? "+" : v < 0 ? "" : "";
  return `${sign}${v.toFixed(2)}`;
}

function corrColor(v: number | null): string {
  if (v == null || !Number.isFinite(v)) return "transparent";
  const x = Math.max(-1, Math.min(1, v));
  if (x >= 0) {
    const alpha = 0.12 + 0.7 * x;
    return `rgba(34, 197, 94, ${alpha.toFixed(3)})`;
  }
  const alpha = 0.12 + 0.7 * Math.abs(x);
  return `rgba(239, 68, 68, ${alpha.toFixed(3)})`;
}

function cellTextColor(v: number | null): string {
  if (v == null) return "var(--text-tertiary)";
  return Math.abs(v) > 0.4 ? "#fff" : "var(--text-primary)";
}

export function CorrelationHeatmapPanel() {
  const { data, error } = useSWR<CorrelationsResponse>(
    "/api/correlations",
    fetcher,
    { refreshInterval: 24 * 60 * 60 * 1000 },
  );

  return (
    <Panel
      title="CORRELATIONS · BTC vs."
      right="rolling Pearson · 30d / 90d"
      className="h-full"
    >
      {error ? (
        <div className="p-3 mono text-[11px] text-[var(--accent-red)] whitespace-pre-wrap">
          correlations fetch failed: {String(error.message ?? error)}
        </div>
      ) : !data ? (
        <div className="p-3 mono text-[12px] text-[var(--text-tertiary)]">
          loading…
        </div>
      ) : (
        <div className="flex flex-col h-full">
          <div className="grid grid-cols-[1fr_56px_56px] gap-px bg-[var(--border)] border-b border-[var(--border)] flex-shrink-0">
            <div className="bg-[var(--bg-panel)] px-3 py-1.5 mono text-[10px] tracking-widest uppercase text-[var(--text-tertiary)]">
              Asset
            </div>
            <div className="bg-[var(--bg-panel)] px-2 py-1.5 mono text-[10px] tracking-widest uppercase text-[var(--text-tertiary)] text-center">
              30d
            </div>
            <div className="bg-[var(--bg-panel)] px-2 py-1.5 mono text-[10px] tracking-widest uppercase text-[var(--text-tertiary)] text-center">
              90d
            </div>
          </div>
          <div className="flex-1 flex flex-col">
            {data.assets.map((a) => (
              <Row key={a.key} asset={a} />
            ))}
          </div>
        </div>
      )}
    </Panel>
  );
}

function Row({ asset }: { asset: CorrelationAsset }) {
  return (
    <div className="grid grid-cols-[1fr_56px_56px] gap-px bg-[var(--border)] border-b border-[var(--border)] last:border-b-0 flex-1">
      <div className="bg-[var(--bg-panel)] px-3 py-2 mono text-[11px] flex flex-col justify-center">
        <div className="text-[var(--text-primary)]">{asset.label}</div>
        <div className="text-[var(--text-tertiary)] text-[10px]">
          {asset.source}
        </div>
      </div>
      <Cell value={asset.r30} error={asset.error} />
      <Cell value={asset.r90} error={asset.error} />
    </div>
  );
}

function Cell({ value, error }: { value: number | null; error?: string }) {
  return (
    <div
      className="px-2 py-2 mono text-[12px] tnum flex items-center justify-center"
      style={{
        background: corrColor(value),
        color: cellTextColor(value),
      }}
      title={error ?? undefined}
    >
      {error && value == null ? "err" : fmtCorr(value)}
    </div>
  );
}
