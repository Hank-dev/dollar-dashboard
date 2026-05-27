"use client";

import useSWR from "swr";
import { Panel } from "@/components/btc/layout/Panel";
import { CbPremiumSparkline } from "@/components/btc/charts/CbPremiumSparkline";
import type { CbPremiumResponse } from "@/lib/btc/types";

const fetcher = (u: string) =>
  fetch(u).then((r) => {
    if (!r.ok) {
      return r.json().then((j) => {
        throw new Error(j?.error ?? `${r.status}`);
      });
    }
    return r.json();
  });

function fmtBps(v: number | null | undefined): string {
  if (v == null || !Number.isFinite(v)) return "—";
  const sign = v > 0 ? "+" : "";
  return `${sign}${v.toFixed(1)} bps`;
}

function toneColor(v: number | null | undefined): string {
  if (v == null) return "var(--text-primary)";
  if (v > 5) return "var(--accent-green)";
  if (v > 0) return "rgba(34, 197, 94, 0.7)";
  if (v < -5) return "var(--accent-red)";
  if (v < 0) return "rgba(239, 68, 68, 0.7)";
  return "var(--text-primary)";
}

function label(v: number | null | undefined): string {
  if (v == null) return "—";
  if (v > 5) return "US bid";
  if (v > 0) return "US slight bid";
  if (v < -5) return "US offer";
  if (v < 0) return "US slight offer";
  return "flat";
}

export function CoinbasePremiumPanel() {
  const { data, error } = useSWR<CbPremiumResponse>(
    "/api/cb-premium",
    fetcher,
    { refreshInterval: 10 * 60 * 1000 },
  );

  const cur = data?.current?.premiumBps ?? null;
  const avg = data?.avg24hBps ?? null;

  return (
    <Panel
      title="COINBASE PREMIUM"
      right="BTC-USD vs BTCUSDT"
      className="h-full"
    >
      {error ? (
        <div className="p-3 mono text-[11px] text-[var(--accent-red)] whitespace-pre-wrap">
          premium fetch failed: {String(error.message ?? error)}
        </div>
      ) : !data ? (
        <div className="p-3 mono text-[12px] text-[var(--text-tertiary)]">
          loading…
        </div>
      ) : (
        <div className="flex flex-col h-full">
          <div className="px-3 py-3 border-b border-[var(--border)] flex-shrink-0">
            <div className="mono text-[10px] tracking-widest uppercase text-[var(--text-tertiary)]">
              current · last hour
            </div>
            <div
              className="mono text-3xl tnum"
              style={{ color: toneColor(cur) }}
            >
              {fmtBps(cur)}
            </div>
            <div
              className="mono text-[11px] mt-1"
              style={{ color: toneColor(cur) }}
            >
              {label(cur)}
            </div>
          </div>
          <div className="px-3 py-1.5 flex items-center justify-between border-b border-[var(--border)] flex-shrink-0">
            <span className="mono text-[10px] tracking-widest uppercase text-[var(--text-tertiary)]">
              24h avg
            </span>
            <span
              className="mono text-[12px] tnum"
              style={{ color: toneColor(avg) }}
            >
              {fmtBps(avg)}
            </span>
          </div>
          <div className="px-3 pt-1 mono text-[10px] tracking-widest uppercase text-[var(--text-tertiary)] flex-shrink-0">
            48h
          </div>
          <div className="flex-1 min-h-[60px]">
            <CbPremiumSparkline data={data.series} />
          </div>
        </div>
      )}
    </Panel>
  );
}
