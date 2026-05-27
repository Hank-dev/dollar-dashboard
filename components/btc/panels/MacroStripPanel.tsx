"use client";

import useSWR from "swr";
import { Panel } from "@/components/btc/layout/Panel";
import type { MacroIndicator, MacroKey, MacroResponse } from "@/lib/btc/types";

// One-line plain-English read of each indicator. Rising = risk-off for every
// one of these (USD up, real yields up, credit spreads wider, vol higher),
// which is why the arrow convention treats ▲ as red across the strip.
const BLURBS: Record<MacroKey, string> = {
  dxy: "broad USD strength · ↑ tightens global $ liquidity",
  dfii10: "real cost of capital · ↑ pressures long-duration risk",
  hyspread: "junk-bond risk premium · ↑ = credit stress",
  vix: "S&P 500 implied vol · ↑ = equity fear",
  move: "US Treasury implied vol · ↑ = rates regime stress",
};

const fetcher = (u: string) =>
  fetch(u).then((r) => {
    if (!r.ok) {
      return r.json().then((j) => {
        throw new Error(j?.error ?? `${r.status}`);
      });
    }
    return r.json();
  });

function fmtVal(i: MacroIndicator): string {
  if (i.current == null) return "—";
  // pick precision based on magnitude
  const abs = Math.abs(i.current);
  const digits = abs >= 50 ? 1 : 2;
  return `${i.current.toFixed(digits)}${i.unit}`;
}

function fmtDelta(i: MacroIndicator): string {
  if (i.delta30d == null) return "";
  const abs = Math.abs(i.delta30d);
  const digits = abs >= 5 ? 1 : 2;
  const sign = i.delta30d > 0 ? "+" : "";
  return `${sign}${i.delta30d.toFixed(digits)}${i.unit}`;
}

// Convention: a *rising* DXY / real yield / HY spread / VIX / MOVE is
// risk-off for crypto. So "up" arrow renders red, "down" renders green.
function toneFor(delta: number | null): {
  arrow: string;
  cls: string;
} {
  if (delta == null) return { arrow: "·", cls: "text-[var(--text-tertiary)]" };
  if (delta > 0)
    return { arrow: "▲", cls: "text-[var(--accent-red)]" };
  if (delta < 0)
    return { arrow: "▼", cls: "text-[var(--accent-green)]" };
  return { arrow: "·", cls: "text-[var(--text-tertiary)]" };
}

export function MacroStripPanel() {
  const { data, error } = useSWR<MacroResponse>("/api/macro", fetcher, {
    refreshInterval: 6 * 60 * 60 * 1000,
  });

  return (
    <Panel
      title="MACRO STRIP · 30D"
      right={
        <span>
          <span className="text-[var(--accent-red)]">▲</span> risk-off ·{" "}
          <span className="text-[var(--accent-green)]">▼</span> risk-on
        </span>
      }
      className="h-full"
    >
      {error ? (
        <div className="p-3 mono text-[11px] text-[var(--accent-red)] whitespace-pre-wrap">
          macro fetch failed: {String(error.message ?? error)}
        </div>
      ) : !data ? (
        <div className="p-3 mono text-[12px] text-[var(--text-tertiary)]">
          loading…
        </div>
      ) : (
        <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-5 divide-x divide-[var(--border)] h-full">
          {data.indicators.map((i) => {
            const tone = toneFor(i.delta30d);
            return (
              <div
                key={i.key}
                className="px-3 py-2 flex flex-col gap-1 min-h-[104px]"
              >
                <div className="mono text-[10px] tracking-widest uppercase text-[var(--text-tertiary)]">
                  {i.label}
                </div>
                {i.error ? (
                  <div className="mono text-[11px] text-[var(--accent-red)]">
                    err
                  </div>
                ) : (
                  <>
                    <div className="mono text-xl tnum text-[var(--text-primary)]">
                      {fmtVal(i)}
                    </div>
                    <div className={`mono text-[11px] tnum ${tone.cls}`}>
                      {tone.arrow} {fmtDelta(i)}
                    </div>
                  </>
                )}
                <div className="mono text-[10px] text-[var(--text-tertiary)] leading-tight mt-auto">
                  {BLURBS[i.key]}
                </div>
              </div>
            );
          })}
        </div>
      )}
    </Panel>
  );
}
