"use client";

import useSWR from "swr";
import { Panel } from "@/components/btc/layout/Panel";
import { RegimeScoreSparkline } from "@/components/btc/charts/RegimeScoreSparkline";
import type { RegimeScoreInputDetail, RegimeScoreResponse } from "@/lib/btc/types";

const fetcher = (u: string) =>
  fetch(u).then((r) => {
    if (!r.ok) {
      return r.json().then((j) => {
        throw new Error(j?.error ?? `${r.status}`);
      });
    }
    return r.json();
  });

// Sign-keyed colors used in the headline number, the per-input pills, and
// the sparkline shading — kept consistent so the eye reads them as one
// system.
function scoreColor(score: number): string {
  if (score >= 3) return "var(--accent-green)";
  if (score >= 1) return "rgba(34, 197, 94, 0.7)";
  if (score === 0) return "var(--text-primary)";
  if (score >= -1) return "rgba(239, 68, 68, 0.7)";
  return "var(--accent-red)";
}

function signLabel(score: number): string {
  if (score > 0) return "+";
  return "";
}

function pillColor(s: -1 | 0 | 1): string {
  if (s > 0) return "var(--accent-green)";
  if (s < 0) return "var(--accent-red)";
  return "var(--text-tertiary)";
}

function pillSign(s: -1 | 0 | 1): string {
  if (s > 0) return "+1";
  if (s < 0) return "-1";
  return "0";
}

function regimeLabel(score: number): string {
  if (score >= 3) return "full long";
  if (score === 2) return "75% long";
  if (score === 1) return "50% long";
  if (score === 0) return "25% long (neutral)";
  if (score === -1) return "10% long";
  return "out (cash)";
}

export function RegimeScorePanel() {
  const { data, error } = useSWR<RegimeScoreResponse>(
    "/api/regime-score",
    fetcher,
    { refreshInterval: 24 * 60 * 60 * 1000 },
  );

  return (
    <Panel
      title="MACRO REGIME SCORE"
      right={
        <span>
          5-factor composite ·{" "}
          <span className="text-[var(--text-tertiary)]">
            FRED + bitcoin-data.com
          </span>
        </span>
      }
      className="h-full"
    >
      {error ? (
        <div className="p-3 mono text-[11px] text-[var(--accent-red)] whitespace-pre-wrap">
          regime score fetch failed: {String(error.message ?? error)}
        </div>
      ) : !data ? (
        <div className="p-3 mono text-[12px] text-[var(--text-tertiary)]">
          loading…
        </div>
      ) : (
        <div className="flex flex-col h-full">
          {/* Headline: composite score + allocation */}
          <div className="flex items-center gap-4 px-3 py-3 border-b border-[var(--border)] flex-shrink-0">
            <div>
              <div className="mono text-[10px] tracking-widest uppercase text-[var(--text-tertiary)]">
                composite
              </div>
              <div
                className="mono text-4xl tnum"
                style={{ color: scoreColor(data.currentScore) }}
              >
                {signLabel(data.currentScore)}
                {data.currentScore}
              </div>
              <div className="mono text-[10px] text-[var(--text-tertiary)]">
                range −5 to +5
              </div>
            </div>
            <div className="flex-1 border-l border-[var(--border)] pl-4">
              <div className="mono text-[10px] tracking-widest uppercase text-[var(--text-tertiary)]">
                target BTC alloc
              </div>
              <div
                className="mono text-2xl tnum"
                style={{ color: scoreColor(data.currentScore) }}
              >
                {data.targetAllocPct.toFixed(0)}%
              </div>
              <div
                className="mono text-[11px]"
                style={{ color: scoreColor(data.currentScore) }}
              >
                {regimeLabel(data.currentScore)}
              </div>
            </div>
          </div>

          {/* 5 sub-input rows */}
          <div className="flex-shrink-0 border-b border-[var(--border)]">
            {data.inputs.map((i) => (
              <InputRow key={i.key} input={i} />
            ))}
          </div>

          {/* History sparkline */}
          <div className="flex-shrink-0 px-3 pt-1 mono text-[10px] tracking-widest uppercase text-[var(--text-tertiary)]">
            2y weekly history
          </div>
          <div className="flex-1 min-h-[60px]">
            <RegimeScoreSparkline data={data.history} />
          </div>
        </div>
      )}
    </Panel>
  );
}

function InputRow({ input }: { input: RegimeScoreInputDetail }) {
  return (
    <div className="px-3 py-1.5 flex items-center justify-between border-b border-[var(--border)] last:border-b-0">
      <div className="flex flex-col min-w-0 flex-1 pr-2">
        <div className="mono text-[11px] text-[var(--text-primary)]">
          {input.label}
        </div>
        <div className="mono text-[10px] text-[var(--text-tertiary)] truncate">
          {input.rationale}
        </div>
      </div>
      <div
        className="mono text-[11px] tnum tabular-nums font-bold w-8 text-right flex-shrink-0"
        style={{ color: pillColor(input.score) }}
      >
        {pillSign(input.score)}
      </div>
    </div>
  );
}
