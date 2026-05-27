"use client";

import { useEffect, useState } from "react";
import { UtcClock } from "@/components/UtcClock";

interface MarketSummary {
  btcPrice: number | null;
  btcChange: number | null;
  dxy: number | null;
  vix: number | null;
  fng: number | null;
}

const DAYS = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];
const MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

function getSessionStatus(): { label: string; status: string; cls: string }[] {
  const now = new Date();
  const utcH = now.getUTCHours();
  const utcM = now.getUTCMinutes();
  const t = utcH * 60 + utcM;

  const tokyoOpen = t >= 0 && t < 6 * 60;
  const londonOpen = t >= 8 * 60 && t < 16 * 60 + 30;
  const nyOpen = t >= 13 * 60 + 30 && t < 20 * 60;

  return [
    { label: "Tokyo", status: tokyoOpen ? "open" : "closed", cls: tokyoOpen ? "text-[var(--accent-green)]" : "text-[var(--text-tertiary)]" },
    { label: "London", status: londonOpen ? "open" : "closed", cls: londonOpen ? "text-[var(--accent-green)]" : "text-[var(--text-tertiary)]" },
    { label: "New York", status: nyOpen ? "open" : "closed", cls: nyOpen ? "text-[var(--accent-green)]" : "text-[var(--text-tertiary)]" },
  ];
}

function riskRegime(m: MarketSummary): "risk-on" | "risk-off" | "mixed" {
  let bullish = 0;
  let bearish = 0;

  if (m.btcChange != null) {
    if (m.btcChange > 0.005) bullish++;
    else if (m.btcChange < -0.005) bearish++;
  }
  if (m.vix != null) {
    if (m.vix < 16) bullish++;
    else if (m.vix > 25) bearish++;
  }
  if (m.fng != null) {
    if (m.fng >= 55) bullish++;
    else if (m.fng <= 30) bearish++;
  }
  if (m.dxy != null) {
    if (m.dxy < 100) bullish++;
    else if (m.dxy > 106) bearish++;
  }

  if (bullish > bearish + 1) return "risk-on";
  if (bearish > bullish + 1) return "risk-off";
  return "mixed";
}

function buildHeadline(m: MarketSummary): { text: string; accent: "up" | "down" | "warn" } {
  const hasData = m.btcPrice != null || m.dxy != null;
  if (!hasData) return { text: "Loading market data...", accent: "warn" };

  const regime = riskRegime(m);

  const dollarPart =
    m.dxy != null
      ? m.dxy > 105 ? "dollar bid" : m.dxy < 100 ? "dollar fading" : "dollar steady"
      : null;

  const volPart =
    m.vix != null
      ? m.vix > 30 ? "vol spiking" : m.vix > 20 ? "vol rising" : m.vix < 14 ? "vol crushed" : null
      : null;

  const cryptoPart =
    m.btcChange != null
      ? Math.abs(m.btcChange) < 0.005 ? "crypto flat" :
        m.btcChange > 0.03 ? "crypto rallying" :
        m.btcChange > 0 ? "crypto bid" :
        m.btcChange < -0.03 ? "crypto selling off" :
        "crypto soft"
      : null;

  const parts: string[] = [];

  if (regime === "risk-on") {
    parts.push("Markets are risk-on");
  } else if (regime === "risk-off") {
    parts.push("Markets are risk-off");
  } else {
    parts.push("Mixed signals across markets");
  }

  const details = [dollarPart, volPart, cryptoPart].filter(Boolean);
  if (details.length > 0) {
    parts[0] += " — " + details.join(", ");
  }

  return {
    text: parts[0] + ".",
    accent: regime === "risk-on" ? "up" : regime === "risk-off" ? "down" : "warn",
  };
}

const ACCENT_COLOR = {
  up: "var(--accent-green)",
  down: "var(--accent-red)",
  warn: "var(--accent-amber)",
};

export function HomeHeader() {
  const [summary, setSummary] = useState<MarketSummary>({
    btcPrice: null, btcChange: null, dxy: null, vix: null, fng: null,
  });
  const [sessions, setSessions] = useState(getSessionStatus());

  useEffect(() => {
    const id = setInterval(() => setSessions(getSessionStatus()), 60_000);
    return () => clearInterval(id);
  }, []);

  useEffect(() => {
    Promise.allSettled([
      fetch("/api/price-history").then((r) => r.json()),
      fetch("/api/macro").then((r) => r.json()),
      fetch("/api/fng").then((r) => r.json()),
    ]).then(([btcResult, macroResult, fngResult]) => {
      const s: MarketSummary = { btcPrice: null, btcChange: null, dxy: null, vix: null, fng: null };

      if (btcResult.status === "fulfilled") {
        const d = btcResult.value;
        const series = d.series as { v: number }[];
        const last = series.at(-1)?.v ?? null;
        const prev = series.at(-2)?.v ?? null;
        s.btcPrice = last;
        s.btcChange = last && prev ? (last - prev) / prev : null;
      }
      if (macroResult.status === "fulfilled") {
        const indicators = macroResult.value.indicators as { key: string; current: number | null }[];
        s.dxy = indicators.find((i) => i.key === "dxy")?.current ?? null;
        s.vix = indicators.find((i) => i.key === "vix")?.current ?? null;
      }
      if (fngResult.status === "fulfilled") {
        s.fng = fngResult.value.current?.v ?? null;
      }
      setSummary(s);
    });
  }, []);

  const d = new Date();
  const { text: headline, accent } = buildHeadline(summary);

  return (
    <header className="grid grid-cols-[1fr_auto_auto] gap-6 items-end px-6 pt-7 pb-5 border-b border-[var(--border)]">
      <div>
        <div className="mono text-[11px] tracking-[0.18em] uppercase text-[var(--text-tertiary)] mb-2.5">
          {DAYS[d.getUTCDay()]} &middot; {MONTHS[d.getUTCMonth()]} {d.getUTCDate()} {d.getUTCFullYear()}
        </div>
        <h1 className="text-[22px] sm:text-[26px] font-semibold tracking-tight m-0" style={{ color: ACCENT_COLOR[accent] }}>
          {headline}
        </h1>
        <div className="mono text-[12px] text-[var(--text-secondary)] mt-1.5 flex flex-wrap gap-x-3">
          {summary.btcPrice != null && (
            <span>BTC <span className="text-[var(--text-primary)]">${summary.btcPrice.toLocaleString("en-US", { maximumFractionDigits: 0 })}</span></span>
          )}
          {summary.dxy != null && (
            <span>DXY <span className="text-[var(--text-primary)]">{summary.dxy.toFixed(2)}</span></span>
          )}
          {summary.vix != null && (
            <span>VIX <span className="text-[var(--text-primary)]">{summary.vix.toFixed(2)}</span></span>
          )}
          {summary.fng != null && (
            <span>F&G <span className={summary.fng >= 60 ? "text-[var(--accent-green)]" : summary.fng <= 25 ? "text-[var(--accent-red)]" : "text-[var(--text-primary)]"}>{summary.fng}</span></span>
          )}
        </div>
      </div>
      <div className="flex gap-3.5 items-center">
        {sessions.map((s) => (
          <div key={s.label} className="flex flex-col items-start gap-0.5 pr-3 border-r border-[var(--border)] last:border-r-0 last:pr-0">
            <span className="mono text-[10px] tracking-[0.14em] uppercase text-[var(--text-tertiary)]">{s.label}</span>
            <span className={`mono text-[12.5px] tabular-nums ${s.cls}`}>
              {s.status === "open" ? "● open" : "closed"}
            </span>
          </div>
        ))}
      </div>
      <div>
        <div className="mono text-[10px] tracking-[0.18em] uppercase text-[var(--text-tertiary)] text-right mb-1.5">UTC</div>
        <UtcClock />
      </div>
    </header>
  );
}
