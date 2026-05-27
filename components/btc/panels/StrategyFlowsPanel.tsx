"use client";

import useSWR from "swr";
import { Panel } from "@/components/btc/layout/Panel";
import { StrcWeeklyChart } from "@/components/btc/charts/StrcWeeklyChart";
import type { StrategyFlowsResponse } from "@/lib/btc/types";

const fetcher = (u: string) =>
  fetch(u).then((r) => {
    if (!r.ok) {
      return r.json().then((j) => {
        throw new Error(j?.error ?? `${r.status}`);
      });
    }
    return r.json();
  });

function fmtUsdM(v: number | null | undefined, digits = 1): string {
  if (v == null || !Number.isFinite(v)) return "—";
  return `$${v.toFixed(digits)}M`;
}

function fmtUsdB(v: number | null | undefined, digits = 1): string {
  if (v == null || !Number.isFinite(v)) return "—";
  return `$${v.toFixed(digits)}B`;
}

function fmtPrice(v: number | null | undefined): string {
  if (v == null || !Number.isFinite(v)) return "—";
  return `$${v.toFixed(2)}`;
}

function fmtBtc(v: number | null | undefined): string {
  if (v == null || !Number.isFinite(v)) return "—";
  if (v >= 1000) return `${v.toLocaleString("en-US", { maximumFractionDigits: 0 })} ₿`;
  return `${v.toFixed(1)} ₿`;
}

function strcPriceTone(p: number | null): string {
  if (p == null) return "var(--text-primary)";
  if (p >= 99) return "var(--accent-green)";
  if (p >= 95) return "var(--accent-amber)";
  return "var(--accent-red)";
}

function flywheelStatus(p: number | null): { text: string; color: string } {
  if (p == null) return { text: "—", color: "var(--text-tertiary)" };
  if (p >= 100) return { text: "flywheel ACTIVE", color: "var(--accent-green)" };
  if (p >= 95)
    return { text: "flywheel slowing", color: "var(--accent-amber)" };
  return { text: "below par — risk", color: "var(--accent-red)" };
}

function ageLabel(filedAt: number | null): string {
  if (filedAt == null) return "—";
  const days = Math.floor((Date.now() / 1000 - filedAt) / 86400);
  if (days < 1) return "today";
  if (days === 1) return "1d ago";
  return `${days}d ago`;
}

export function StrategyFlowsPanel() {
  const { data, error } = useSWR<StrategyFlowsResponse>(
    "/api/strategy",
    fetcher,
    { refreshInterval: 6 * 60 * 60 * 1000 },
  );

  const right = data?.latestFiledAt
    ? `8-K · ${ageLabel(data.latestFiledAt)}`
    : "";

  return (
    <Panel title="STRATEGY FLOWS (STRC)" right={right} className="h-full">
      {error ? (
        <div className="p-3 mono text-[11px] text-[var(--accent-red)] whitespace-pre-wrap">
          strategy fetch failed: {String(error.message ?? error)}
        </div>
      ) : !data ? (
        <div className="p-3 mono text-[12px] text-[var(--text-tertiary)]">
          loading…
        </div>
      ) : (
        <div className="flex flex-col h-full">
          <div className="grid grid-cols-2 sm:grid-cols-3 divide-x divide-y divide-[var(--border)] flex-shrink-0">
            <Card
              label="STRC price"
              value={fmtPrice(data.strcPrice)}
              valueColor={strcPriceTone(data.strcPrice)}
              sub={
                <span style={{ color: flywheelStatus(data.strcPrice).color }}>
                  {flywheelStatus(data.strcPrice).text}
                </span>
              }
            />
            <Card
              label="Last week"
              value={fmtUsdM(data.strcLastWeekM)}
              sub="STRC ATM net proceeds"
            />
            <Card
              label="BTC bought (impl.)"
              value={fmtBtc(data.strcLastWeekImpliedBtc)}
              sub="proceeds ÷ wk avg BTC"
            />
            <Card
              label="STRC YTD"
              value={fmtUsdM(data.strcYtdM, 0)}
              sub="cumulative net proceeds"
            />
            <Card
              label="STRC capacity"
              value={fmtUsdM(data.strcRemainingCapacityM, 0)}
              sub="remaining ATM (STRC only)"
            />
            <Card
              label="Total ATM cap"
              value={fmtUsdB(data.totalAtmCapacityB)}
              sub="all preferreds + MSTR"
            />
          </div>
          <div className="flex items-center justify-between px-3 pt-2 pb-1 flex-shrink-0">
            <div className="mono text-[10px] tracking-widest uppercase text-[var(--text-tertiary)]">
              12-week ATM capital raised ($M)
            </div>
            <div className="mono text-[10px] text-[var(--text-tertiary)]">
              {data.weekly.length} filings
            </div>
          </div>
          <div className="flex-1 min-h-[80px]">
            <StrcWeeklyChart data={data.weekly} />
          </div>
        </div>
      )}
    </Panel>
  );
}

function Card({
  label,
  value,
  sub,
  valueColor = "var(--text-primary)",
}: {
  label: string;
  value: string;
  sub?: React.ReactNode;
  valueColor?: string;
}) {
  return (
    <div className="px-3 py-2 flex flex-col gap-0.5">
      <div className="mono text-[10px] tracking-widest uppercase text-[var(--text-tertiary)]">
        {label}
      </div>
      <div className="mono text-lg tnum" style={{ color: valueColor }}>
        {value}
      </div>
      {sub ? (
        <div className="mono text-[10px] text-[var(--text-secondary)] leading-tight">
          {sub}
        </div>
      ) : null}
    </div>
  );
}
