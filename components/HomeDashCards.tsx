"use client";

import { useEffect, useState } from "react";
import Link from "next/link";

interface BtcData {
  price: number | null;
  ath: number | null;
  daysSinceAth: number | null;
  daysSinceLastHalving: number | null;
  powerLawZ: number | null;
  change24h: number | null;
}

interface DollarData {
  dxy: number | null;
  dxyDelta: number | null;
  vix: number | null;
  hySpread: number | null;
  move: number | null;
}

interface FngData {
  value: number | null;
  label: string | null;
}

interface AiData {
  topModel: string | null;
  topIndex: number | null;
  cheapestModel: string | null;
  cheapestCost: number | null;
  modelCount: number;
}

interface NuclearData {
  totalPublicMarketCapUsd: number | null;
  weightedChangePercent: number | null;
  proxySymbol: string | null;
  proxyChangePercent: number | null;
  quoteCount: number;
}

interface NuclearQuoteLite {
  symbol: string;
  changePercent: number | null;
  marketCapUsd: number | null;
}

function fmtNum(v: number | null, decimals = 0): string {
  if (v == null) return "—";
  return v.toLocaleString("en-US", { minimumFractionDigits: decimals, maximumFractionDigits: decimals });
}

function fmtPct(v: number | null): string {
  if (v == null) return "—";
  const sign = v >= 0 ? "+" : "";
  return `${sign}${(v * 100).toFixed(1)}%`;
}

function fmtPctPoints(v: number | null): string {
  if (v == null) return "—";
  const sign = v >= 0 ? "+" : "";
  return `${sign}${v.toFixed(1)}%`;
}

function fmtUsdCompact(v: number | null): string {
  if (v == null) return "—";
  return new Intl.NumberFormat("en-US", {
    style: "currency",
    currency: "USD",
    notation: "compact",
    maximumFractionDigits: 1,
  }).format(v);
}

function weightedChangePercent(quotes: NuclearQuoteLite[]): number | null {
  let weighted = 0;
  let totalCap = 0;
  const fallback: number[] = [];

  for (const quote of quotes) {
    if (quote.changePercent == null) continue;
    fallback.push(quote.changePercent);
    if (quote.marketCapUsd == null) continue;
    weighted += quote.changePercent * quote.marketCapUsd;
    totalCap += quote.marketCapUsd;
  }

  if (totalCap > 0) return weighted / totalCap;
  if (fallback.length === 0) return null;
  return fallback.reduce((sum, value) => sum + value, 0) / fallback.length;
}

function valuesToPath(values: number[], w: number, h: number, pad = 2): { line: string; area: string } {
  if (values.length < 2) return { line: "", area: "" };
  const min = Math.min(...values);
  const max = Math.max(...values);
  const range = max - min || 1;
  const points = values.map((v, i) => {
    const x = (i / (values.length - 1)) * w;
    const y = pad + (1 - (v - min) / range) * (h - pad * 2);
    return `${x.toFixed(1)},${y.toFixed(1)}`;
  });
  const line = `M${points.join(" L")}`;
  const area = `${line} L${w},${h} L0,${h} Z`;
  return { line, area };
}

export function HomeDashCards() {
  const [btc, setBtc] = useState<BtcData>({ price: null, ath: null, daysSinceAth: null, daysSinceLastHalving: null, powerLawZ: null, change24h: null });
  const [dollar, setDollar] = useState<DollarData>({ dxy: null, dxyDelta: null, vix: null, hySpread: null, move: null });
  const [fng, setFng] = useState<FngData>({ value: null, label: null });
  const [ai, setAi] = useState<AiData>({ topModel: null, topIndex: null, cheapestModel: null, cheapestCost: null, modelCount: 0 });
  const [nuclear, setNuclear] = useState<NuclearData>({
    totalPublicMarketCapUsd: null,
    weightedChangePercent: null,
    proxySymbol: null,
    proxyChangePercent: null,
    quoteCount: 0,
  });
  const [livePrice, setLivePrice] = useState<number | null>(null);
  const [sparks, setSparks] = useState<{ btc: number[]; dxy: number[]; fng: number[] }>({ btc: [], dxy: [], fng: [] });

  useEffect(() => {
    fetch("/api/price-history")
      .then((r) => r.json())
      .then((d) => {
        const series = d.series as { v: number }[];
        const last = series.at(-1)?.v ?? null;
        const prev = series.at(-2)?.v ?? null;
        setBtc({
          price: last,
          ath: d.ath?.v ?? null,
          daysSinceAth: d.daysSinceAth ?? null,
          daysSinceLastHalving: d.daysSinceLastHalving ?? null,
          powerLawZ: d.powerLaw?.sigma ?? null,
          change24h: last && prev ? (last - prev) / prev : null,
        });
      })
      .catch(() => {});

    fetch("/api/regime")
      .then((r) => r.json())
      .then((d) => {
        setBtc((prev) => ({ ...prev, powerLawZ: d.powerLawZ ?? prev.powerLawZ }));
      })
      .catch(() => {});

    fetch("/api/macro")
      .then((r) => r.json())
      .then((d) => {
        const indicators = d.indicators as { key: string; current: number | null; delta30d: number | null }[];
        const dxyI = indicators.find((i) => i.key === "dxy");
        const vixI = indicators.find((i) => i.key === "vix");
        const hyI = indicators.find((i) => i.key === "hyspread");
        const moveI = indicators.find((i) => i.key === "move");
        setDollar({
          dxy: dxyI?.current ?? null,
          dxyDelta: dxyI?.delta30d ?? null,
          vix: vixI?.current ?? null,
          hySpread: hyI?.current ?? null,
          move: moveI?.current ?? null,
        });
      })
      .catch(() => {});

    fetch("/api/fng")
      .then((r) => r.json())
      .then((d) => {
        setFng({ value: d.current?.v ?? null, label: d.current?.label ?? null });
      })
      .catch(() => {});

    fetch("/api/ai/frontier-models")
      .then((r) => r.json())
      .then((d) => {
        const points = d.points as { label: string; intelligenceIndex: number; blendedUsdPerMillion: number }[];
        if (points.length === 0) return;
        const sorted = [...points].sort((a, b) => b.intelligenceIndex - a.intelligenceIndex);
        const cheapest = [...points].sort((a, b) => a.blendedUsdPerMillion - b.blendedUsdPerMillion);
        setAi({
          topModel: sorted[0].label,
          topIndex: sorted[0].intelligenceIndex,
          cheapestModel: cheapest[0].label,
          cheapestCost: cheapest[0].blendedUsdPerMillion,
          modelCount: points.length,
        });
      })
      .catch(() => {});

    fetch("/api/home-sparklines")
      .then((r) => r.json())
      .then((d) => setSparks({ btc: d.btc ?? [], dxy: d.dxy ?? [], fng: d.fng ?? [] }))
      .catch(() => {});

    fetch("/api/nuclear/market")
      .then((r) => r.json())
      .then((d) => {
        const publicEquities = (d.publicEquities ?? []) as NuclearQuoteLite[];
        const uraniumProxies = (d.uraniumProxies ?? []) as NuclearQuoteLite[];
        const proxy = uraniumProxies[0];
        setNuclear({
          totalPublicMarketCapUsd: d.totalPublicMarketCapUsd ?? null,
          weightedChangePercent: weightedChangePercent(publicEquities),
          proxySymbol: proxy?.symbol ?? null,
          proxyChangePercent: proxy?.changePercent ?? null,
          quoteCount: publicEquities.length,
        });
      })
      .catch(() => {});
  }, []);

  useEffect(() => {
    const ws = new WebSocket("wss://stream.binance.com:9443/ws/btcusdt@trade");
    ws.onmessage = (e) => {
      try {
        const msg = JSON.parse(e.data) as { p: string };
        const px = parseFloat(msg.p);
        if (Number.isFinite(px)) setLivePrice(px);
      } catch {}
    };
    return () => ws.close();
  }, []);

  const displayPrice = livePrice ?? btc.price;
  const athPct = displayPrice && btc.ath ? (displayPrice - btc.ath) / btc.ath : null;

  const dollarStress =
    dollar.vix != null
      ? dollar.vix > 25 ? "stressed" : dollar.vix > 18 ? "elevated" : dollar.vix < 14 ? "calm" : "neutral"
      : "—";

  const nuclearSpark = [42, 43, 44, 47, 51, 55, 58, 62, 67, 71, 78];

  return (
    <div className="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-4 gap-4 p-5">
      <DashCard
        href="/btc"
        idx="01"
        name="Bitcoin"
        nameAccent="Terminal"
        route="/btc"
        accentVar="var(--dash-btc)"
        sparkColor="oklch(0.82 0.14 80)"
        sparkData={sparks.btc}
        sparkLabel="BTC · 90d"
        heroValue={displayPrice ? `$${fmtNum(displayPrice)}` : "$—"}
        heroMeta={btc.change24h != null ? fmtPct(btc.change24h) + " · 24h" : "· 24h"}
        heroMetaDir={btc.change24h != null ? (btc.change24h >= 0 ? "up" : "down") : undefined}
        stats={[
          { label: "vs ATH", value: athPct != null ? fmtPct(athPct) : "—", dir: athPct != null ? (athPct >= 0 ? "up" : "down") : undefined },
          { label: "cycle day", value: btc.daysSinceLastHalving != null ? String(btc.daysSinceLastHalving) : "—" },
          { label: "power-law Z", value: btc.powerLawZ != null ? (btc.powerLawZ >= 0 ? "+" : "") + btc.powerLawZ.toFixed(2) : "—", dir: btc.powerLawZ != null ? (Math.abs(btc.powerLawZ) > 1.5 ? "warn" : undefined) : undefined },
          { label: "F&G", value: fng.value != null ? `${fng.value} · ${fng.label}` : "—", dir: fng.value != null ? (fng.value >= 60 ? "up" : fng.value <= 30 ? "down" : undefined) : undefined },
        ]}
        summary="Power law, regime detection, ETF flows, funding rates, market emotion, and AI interpretation."
      />

      <DashCard
        href="/dollar"
        idx="02"
        name="Dollar &"
        nameAccent="Macro"
        route="/dollar"
        accentVar="var(--dash-dollar)"
        sparkColor="oklch(0.73 0.14 245)"
        sparkData={sparks.dxy}
        sparkLabel="DXY · 90d"
        heroValue={dollar.dxy != null ? fmtNum(dollar.dxy, 2) : "—"}
        heroMeta={dollar.dxyDelta != null ? `${dollar.dxyDelta > 0 ? "+" : ""}${dollar.dxyDelta.toFixed(2)} · DXY 30d` : "· DXY"}
        heroMetaDir={dollar.dxyDelta != null ? (dollar.dxyDelta > 0 ? "up" : "down") : undefined}
        stats={[
          { label: "VIX", value: dollar.vix != null ? fmtNum(dollar.vix, 1) : "—", dir: dollar.vix != null ? (dollar.vix > 25 ? "down" : dollar.vix < 15 ? "up" : undefined) : undefined },
          { label: "HY spread", value: dollar.hySpread != null ? fmtNum(dollar.hySpread, 2) : "—" },
          { label: "MOVE", value: dollar.move != null ? fmtNum(dollar.move, 1) : "—" },
          { label: "stress", value: dollarStress, dir: dollarStress === "calm" ? "up" : dollarStress === "stressed" ? "down" : dollarStress === "elevated" ? "warn" : undefined },
        ]}
        summary="US Treasury yields, dollar, VIX, carry trades, and risk-haven metrics with Claude-powered explanations."
      />

      <DashCard
        href="/ai"
        idx="03"
        name="AI &"
        nameAccent="Agents"
        route="/ai"
        accentVar="var(--dash-ai)"
        sparkColor="oklch(0.74 0.16 295)"
        sparkData={sparks.fng}
        sparkLabel="F&G · 90d"
        heroValue={ai.topModel ?? "—"}
        heroMeta={ai.topIndex != null ? `index ${ai.topIndex.toFixed(1)} · #1 frontier` : "· frontier models"}
        stats={[
          { label: "top score", value: ai.topIndex != null ? ai.topIndex.toFixed(1) : "—" },
          { label: "cheapest", value: ai.cheapestModel ? `${ai.cheapestModel}` : "—" },
          { label: "cheapest $/1M", value: ai.cheapestCost != null ? `$${ai.cheapestCost.toFixed(2)}` : "—" },
          { label: "live models", value: ai.modelCount > 0 ? String(ai.modelCount) : "—" },
        ]}
        summary="Frontier model intelligence vs cost, public market leaders, private lab valuations, and technology radar."
      />

      <DashCard
        href="/nuclear"
        idx="04"
        name="Nuclear"
        nameAccent="Energy"
        route="/nuclear"
        accentVar="var(--dash-nuclear)"
        sparkColor="oklch(0.82 0.11 200)"
        sparkData={nuclearSpark}
        sparkLabel="buildout · signal"
        heroValue={
          nuclear.totalPublicMarketCapUsd != null
            ? fmtUsdCompact(nuclear.totalPublicMarketCapUsd)
            : "78 GW"
        }
        heroMeta={
          nuclear.totalPublicMarketCapUsd != null
            ? "tracked public cap"
            : "under construction"
        }
        stats={[
          {
            label: "public move",
            value: fmtPctPoints(nuclear.weightedChangePercent),
            dir:
              nuclear.weightedChangePercent == null
                ? undefined
                : nuclear.weightedChangePercent >= 0
                ? "up"
                : "down",
          },
          {
            label: nuclear.proxySymbol ?? "uranium proxy",
            value: fmtPctPoints(nuclear.proxyChangePercent),
            dir:
              nuclear.proxyChangePercent == null
                ? undefined
                : nuclear.proxyChangePercent >= 0
                ? "up"
                : "down",
          },
          { label: "live tickers", value: nuclear.quoteCount ? String(nuclear.quoteCount) : "—" },
          { label: "HALEU", value: "constrained", dir: "warn" },
        ]}
        summary="Nuclear fleets, uranium, HALEU, SMRs, data-center PPAs, restarts, and advanced reactor readiness."
      />
    </div>
  );
}

function DashCard({
  href, idx, name, nameAccent, route, accentVar, sparkColor,
  sparkData, sparkLabel, heroValue, heroMeta, heroMetaDir, stats, summary,
}: {
  href: string;
  idx: string;
  name: string;
  nameAccent: string;
  route: string;
  accentVar: string;
  sparkColor: string;
  sparkData: number[];
  sparkLabel: string;
  heroValue: string;
  heroMeta: string;
  heroMetaDir?: "up" | "down";
  stats: { label: string; value: string; dir?: "up" | "down" | "warn" }[];
  summary: string;
}) {
  const W = 140, H = 36;
  const { line, area } = valuesToPath(sparkData, W, H);
  const hasSparkData = sparkData.length >= 2;

  return (
    <Link
      href={href}
      className="border border-[var(--border)] bg-[var(--bg-surface)] flex flex-col relative no-underline text-inherit transition-all hover:border-[var(--border-strong)] hover:bg-[var(--bg-surface-hover)] overflow-hidden group"
    >
      <span className="absolute left-0 top-0 bottom-0 w-[3px]" style={{ background: accentVar }} />

      <div className="flex items-center gap-2.5 px-[18px] py-3 border-b border-[var(--border)]">
        <span className="mono text-[11px] text-[var(--text-tertiary)] tracking-[0.06em]">{idx} &middot;</span>
        <span className="mono text-[12px] tracking-[0.14em] uppercase text-[var(--text-primary)]">
          {name} <span style={{ color: accentVar }}>{nameAccent}</span>
        </span>
        <span className="mono text-[10.5px] text-[var(--text-tertiary)] tracking-[0.04em] ml-auto">{route}</span>
        <span className="w-[22px] h-[22px] border border-[var(--border)] flex items-center justify-center text-[var(--text-secondary)] text-xs transition-all group-hover:text-[var(--text-primary)] group-hover:border-[var(--border-strong)]">
          ↗
        </span>
      </div>

      <div className="flex gap-4 items-start px-[18px] pt-4 pb-2.5">
        <div className="flex-1 min-w-0">
          <div className="mono text-[28px] font-medium tracking-tight text-[var(--text-primary)] tabular-nums leading-none">
            {heroValue}
          </div>
          <div className="flex items-baseline gap-2 mt-1.5 mono text-[11.5px] tracking-[0.04em] uppercase">
            {heroMetaDir ? (
              <span className={heroMetaDir === "up" ? "text-[var(--accent-green)]" : "text-[var(--accent-red)]"}>{heroMeta}</span>
            ) : (
              <span className="text-[var(--text-tertiary)]">{heroMeta}</span>
            )}
          </div>
        </div>
        <div className="w-[140px]">
          <div className="mono text-[9.5px] tracking-[0.14em] uppercase text-[var(--text-tertiary)] text-right mb-1">
            {sparkLabel}
          </div>
          <svg className="block w-full h-9" viewBox={`0 0 ${W} ${H}`} preserveAspectRatio="none">
            {hasSparkData ? (
              <>
                <defs>
                  <linearGradient id={`grad-${idx}`} x1="0" x2="0" y1="0" y2="1">
                    <stop offset="0%" stopColor={sparkColor} stopOpacity={0.35} />
                    <stop offset="100%" stopColor={sparkColor} stopOpacity={0} />
                  </linearGradient>
                </defs>
                <path d={area} fill={`url(#grad-${idx})`} />
                <path d={line} fill="none" stroke={sparkColor} strokeWidth={1.2} />
              </>
            ) : (
              <text x="70" y="20" textAnchor="middle" fill="var(--text-tertiary)" fontSize="9" fontFamily="var(--font-mono)">loading</text>
            )}
          </svg>
        </div>
      </div>

      <div className="grid grid-cols-2 border-t border-[var(--border)] mt-auto">
        {stats.map((s, i) => (
          <div key={s.label} className={`px-3.5 py-2.5 ${i % 2 === 0 ? "border-r border-[var(--border)]" : ""}`}>
            <div className="mono text-[9.5px] tracking-[0.14em] uppercase text-[var(--text-tertiary)] mb-1">{s.label}</div>
            <div className={`mono text-[13px] tabular-nums ${
              s.dir === "up" ? "text-[var(--accent-green)]" : s.dir === "down" ? "text-[var(--accent-red)]" : s.dir === "warn" ? "text-[var(--accent-amber)]" : "text-[var(--text-primary)]"
            }`}>
              {s.value}
            </div>
          </div>
        ))}
      </div>

      <div className="px-[18px] py-3 text-[12.5px] leading-relaxed text-[var(--text-secondary)] border-t border-[var(--border)]">
        {summary}
      </div>
    </Link>
  );
}
