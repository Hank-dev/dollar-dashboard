"use client";

import useSWR from "swr";
import { Panel } from "@/components/btc/layout/Panel";
import { REGIMES, REGIME_COLORS, type RegimeState } from "@/lib/btc/calc/marketEmotion";
import type { MeiResponse } from "@/lib/btc/types";

const fetcher = (u: string) =>
  fetch(u).then((r) => {
    if (!r.ok) throw new Error(`${r.status}`);
    return r.json();
  });

// 3x3 grid laid out top-left = (Bear, High), bottom-right = (Bull, Low).
const GRID_ROWS: { arousalBand: "high" | "mid" | "low"; cells: RegimeState[] }[] = [
  { arousalBand: "high", cells: ["Panic", "Volatile", "Euphoria"] },
  { arousalBand: "mid", cells: ["Anxiety", "Neutral", "Optimism"] },
  { arousalBand: "low", cells: ["Despondency", "Apathy", "Complacency"] },
];

export function MarketEmotionPanel() {
  const { data, error } = useSWR<MeiResponse>("/api/mei", fetcher, {
    refreshInterval: 60 * 60 * 1000,
  });

  const asOf = data?.current.t
    ? new Date(data.current.t * 1000).toISOString().slice(0, 10)
    : "loading…";

  return (
    <Panel title="MARKET EMOTION" right={`as of ${asOf}`} className="h-full">
      {error ? (
        <div className="p-3 mono text-[12px] text-[var(--accent-red)]">
          MEI fetch failed: {String(error)}
        </div>
      ) : data ? (
        <Content data={data} />
      ) : (
        <div className="p-3 mono text-[12px] text-[var(--text-tertiary)]">
          loading…
        </div>
      )}
    </Panel>
  );
}

function Content({ data }: { data: MeiResponse }) {
  const { current } = data;
  const color = REGIME_COLORS[current.state] ?? REGIME_COLORS.warmup;

  return (
    <div className="flex flex-col h-full gap-3 p-3">
      <Headline state={current.state} color={color} />
      <div className="flex-1 min-h-0 grid grid-cols-1 md:grid-cols-[minmax(180px,1fr)_minmax(280px,2fr)] gap-3">
        <Scores
          valence={current.valence}
          arousal={current.arousal}
          intensity={current.intensity}
        />
        <Grid current={current} />
      </div>
      <Footer distribution={data.distribution} volumeCoverage={data.volumeCoverage} />
    </div>
  );
}

function Headline({ state, color }: { state: RegimeState; color: string }) {
  return (
    <div
      className="rounded-sm px-4 py-3 flex items-center gap-3"
      style={{
        background: `linear-gradient(90deg, ${color}33 0%, ${color}11 100%)`,
        borderLeft: `3px solid ${color}`,
      }}
    >
      <span
        className="inline-block w-3 h-3 rounded-full shrink-0"
        style={{ background: color }}
      />
      <span className="mono text-[20px] tracking-wider uppercase text-[var(--text-primary)]">
        {state}
      </span>
    </div>
  );
}

function Scores({
  valence,
  arousal,
  intensity,
}: {
  valence: number | null;
  arousal: number | null;
  intensity: number | null;
}) {
  const fmt = (v: number | null, d = 0) =>
    v == null ? "—" : v.toFixed(d);
  return (
    <div className="grid grid-cols-3 md:grid-cols-1 gap-px bg-[var(--border)] content-start">
      <ScoreCell label="VALENCE" value={fmt(valence)} hint="0 bear · 100 bull" />
      <ScoreCell label="AROUSAL" value={fmt(arousal)} hint="0 calm · 100 active" />
      <ScoreCell label="INTENSITY" value={fmt(intensity, 2)} hint="how far into corner" />
    </div>
  );
}

function ScoreCell({
  label,
  value,
  hint,
}: {
  label: string;
  value: string;
  hint: string;
}) {
  return (
    <div className="bg-[var(--bg-panel)] px-3 py-2 flex flex-col gap-0.5">
      <span className="mono text-[9px] tracking-widest uppercase text-[var(--text-tertiary)]">
        {label}
      </span>
      <span className="mono text-[22px] text-[var(--text-primary)] tabular-nums">
        {value}
      </span>
      <span className="mono text-[9px] text-[var(--text-tertiary)]">{hint}</span>
    </div>
  );
}

function Grid({
  current,
}: {
  current: MeiResponse["current"];
}) {
  // Map (valence 0-100, arousal 0-100) -> position in a 300x300 viewBox where
  // the inner grid spans (40..280, 40..280). Bear→Bull is left→right; high
  // arousal is TOP (low y), low arousal is BOTTOM (high y).
  const inset = 40;
  const innerSize = 240;
  const vx = current.valence;
  const ay = current.arousal;
  const dotX = vx == null ? null : inset + (vx / 100) * innerSize;
  const dotY = ay == null ? null : inset + ((100 - ay) / 100) * innerSize;

  return (
    <div className="flex-1 min-h-0 flex">
      <svg
        viewBox="0 0 320 320"
        preserveAspectRatio="xMidYMid meet"
        className="w-full h-full max-h-[320px]"
      >
        {/* Cell backgrounds */}
        {GRID_ROWS.map((row, rIdx) =>
          row.cells.map((regime, cIdx) => {
            const x = inset + (cIdx * innerSize) / 3;
            const y = inset + (rIdx * innerSize) / 3;
            const w = innerSize / 3;
            const isCurrent = current.state === regime;
            return (
              <g key={regime}>
                <rect
                  x={x}
                  y={y}
                  width={w}
                  height={w}
                  fill={REGIME_COLORS[regime]}
                  fillOpacity={isCurrent ? 0.45 : 0.18}
                  stroke="#1a1d23"
                  strokeWidth={1}
                />
                <text
                  x={x + w / 2}
                  y={y + w / 2 + 4}
                  textAnchor="middle"
                  fontFamily="var(--font-jetbrains-mono), monospace"
                  fontSize={11}
                  fill={isCurrent ? "#ffffff" : "#cbd1d7"}
                  fontWeight={isCurrent ? 700 : 400}
                >
                  {regime}
                </text>
              </g>
            );
          }),
        )}

        {/* Threshold guides (40/60 valence, 35/65 arousal) — these are the band edges */}
        <g stroke="#2a2f38" strokeWidth={1} strokeDasharray="2 3">
          <line x1={inset + (40 / 100) * innerSize} y1={inset} x2={inset + (40 / 100) * innerSize} y2={inset + innerSize} />
          <line x1={inset + (60 / 100) * innerSize} y1={inset} x2={inset + (60 / 100) * innerSize} y2={inset + innerSize} />
          <line x1={inset} y1={inset + ((100 - 35) / 100) * innerSize} x2={inset + innerSize} y2={inset + ((100 - 35) / 100) * innerSize} />
          <line x1={inset} y1={inset + ((100 - 65) / 100) * innerSize} x2={inset + innerSize} y2={inset + ((100 - 65) / 100) * innerSize} />
        </g>

        {/* Axis labels */}
        <g fontFamily="var(--font-jetbrains-mono), monospace" fontSize={9} fill="#8b929c" letterSpacing={1}>
          <text x={inset} y={inset - 18} textAnchor="start">AROUSAL ↑</text>
          <text x={inset - 4} y={inset + 4} textAnchor="end">HIGH</text>
          <text x={inset - 4} y={inset + innerSize / 2 + 4} textAnchor="end">MID</text>
          <text x={inset - 4} y={inset + innerSize - 4} textAnchor="end">LOW</text>
          <text x={inset + innerSize / 2} y={inset + innerSize + 18} textAnchor="middle">VALENCE →</text>
          <text x={inset} y={inset + innerSize + 18} textAnchor="middle">BEAR</text>
          <text x={inset + innerSize / 2} y={inset + innerSize + 30} textAnchor="middle">NEUTRAL</text>
          <text x={inset + innerSize} y={inset + innerSize + 18} textAnchor="middle">BULL</text>
        </g>

        {/* Current point dot */}
        {dotX != null && dotY != null ? (
          <g>
            <circle cx={dotX} cy={dotY} r={10} fill="none" stroke="#ffffff" strokeWidth={1} opacity={0.4} />
            <circle cx={dotX} cy={dotY} r={5} fill="#ffffff" stroke="#0a0c0f" strokeWidth={1.5} />
          </g>
        ) : null}
      </svg>
    </div>
  );
}

function Footer({
  distribution,
  volumeCoverage,
}: {
  distribution: MeiResponse["distribution"];
  volumeCoverage: number;
}) {
  const total = distribution.reduce((s, r) => s + r.count, 0);
  const top3 = [...distribution]
    .filter((r) => REGIMES.includes(r.state as (typeof REGIMES)[number]))
    .sort((a, b) => b.count - a.count)
    .slice(0, 3);
  return (
    <div className="border-t border-[var(--border)] pt-2 mono text-[10px] text-[var(--text-tertiary)] flex flex-wrap gap-x-4 gap-y-0.5">
      <span>
        history ({total.toLocaleString()} bars) · most-visited:{" "}
        {top3.map((r, i) => (
          <span key={r.state}>
            <span className="text-[var(--text-secondary)]">{r.state}</span>{" "}
            {(r.pct * 100).toFixed(0)}%{i < top3.length - 1 ? " · " : ""}
          </span>
        ))}
      </span>
      <span>
        volume coverage {(volumeCoverage * 100).toFixed(0)}%
      </span>
    </div>
  );
}
