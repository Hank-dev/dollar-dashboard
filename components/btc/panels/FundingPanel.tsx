"use client";

import useSWR from "swr";
import { Panel } from "@/components/btc/layout/Panel";
import type { FundingResponse } from "@/lib/btc/types";

const fetcher = (u: string) =>
  fetch(u).then((r) => {
    if (!r.ok) {
      return r.json().then((j) => {
        throw new Error(j?.error ?? `${r.status}`);
      });
    }
    return r.json();
  });

function fmtRate8h(v: number | null): string {
  if (v == null || !Number.isFinite(v)) return "—";
  const pct = v * 100;
  const sign = pct > 0 ? "+" : "";
  return `${sign}${pct.toFixed(4)}%`;
}

function fmtAnnualized(v: number | null): string {
  if (v == null || !Number.isFinite(v)) return "—";
  const pct = v * 100;
  const sign = pct > 0 ? "+" : "";
  return `${sign}${pct.toFixed(1)}%`;
}

function toneFor(v: number | null): string {
  if (v == null) return "text-[var(--text-primary)]";
  if (v > 0.0002) return "text-[var(--accent-red)]"; // hot
  if (v > 0.00005) return "text-[var(--accent-amber)]";
  if (v < -0.00005) return "text-[var(--accent-green)]"; // capitulation
  return "text-[var(--text-primary)]";
}

const LABELS: Record<string, string> = {
  binance: "Binance",
  bybit: "Bybit",
  okx: "OKX",
};

export function FundingPanel() {
  const { data, error } = useSWR<FundingResponse>("/api/funding", fetcher, {
    refreshInterval: 10 * 60 * 1000,
  });

  const annu = data?.combinedAnnualized7d ?? null;
  const avg7d = data?.combinedAvg7d ?? null;
  const tone = toneFor(avg7d);

  return (
    <Panel title="PERP FUNDING (7D AVG)" right="Binance · Bybit · OKX" className="h-full">
      {error ? (
        <div className="p-3 mono text-[11px] text-[var(--accent-red)] whitespace-pre-wrap">
          {String(error.message ?? error)}
        </div>
      ) : !data ? (
        <div className="p-3 mono text-[12px] text-[var(--text-tertiary)]">
          loading…
        </div>
      ) : (
        <div className="flex flex-col h-full">
          <div className="px-3 py-3 border-b border-[var(--border)]">
            <div className="mono text-[10px] tracking-widest uppercase text-[var(--text-tertiary)]">
              7d avg · annualized
            </div>
            <div className={`mono text-3xl tnum ${tone}`}>
              {fmtAnnualized(annu)}
            </div>
            <div className="mono text-[11px] text-[var(--text-secondary)] mt-1">
              {fmtRate8h(avg7d)} per 8h ·{" "}
              {avg7d == null
                ? ""
                : avg7d > 0
                  ? "longs paying"
                  : "shorts paying"}
            </div>
          </div>
          <div className="flex-1">
            {data.exchanges.map((e) => (
              <div
                key={e.exchange}
                className="px-3 py-1.5 flex items-center justify-between border-b border-[var(--border)] last:border-b-0"
              >
                <div className="mono text-[11px] tracking-wider uppercase text-[var(--text-secondary)]">
                  {LABELS[e.exchange] ?? e.exchange}
                </div>
                {e.error ? (
                  <div className="mono text-[10px] text-[var(--accent-red)]">
                    err
                  </div>
                ) : (
                  <div className="flex gap-3 items-baseline">
                    <span
                      className={`mono text-[11px] tnum ${toneFor(e.avg7d)}`}
                    >
                      7d {fmtRate8h(e.avg7d)}
                    </span>
                    <span
                      className={`mono text-[11px] tnum ${toneFor(e.current)}`}
                    >
                      now {fmtRate8h(e.current)}
                    </span>
                  </div>
                )}
              </div>
            ))}
          </div>
        </div>
      )}
    </Panel>
  );
}
