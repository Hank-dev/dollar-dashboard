import type { RawData } from "./fetchers";

export type Status = "calm" | "neutral" | "elevated" | "stressed";

export interface Metric {
  id: string;
  group: "rates" | "dollar" | "risk";
  label: string;
  value: string;
  unit?: string;
  status: Status;
  context: string;
}

export interface CurvePoint {
  m: string;
  y: number;
}

export interface DashboardData {
  metrics: Metric[];
  yieldCurve: CurvePoint[];
  snapshotDate: string; // ISO yyyy-mm-dd
}

export const STATUS_COLOR: Record<Status, string> = {
  calm: "oklch(0.78 0.16 145)",
  neutral: "#828a98",
  elevated: "oklch(0.82 0.14 80)",
  stressed: "oklch(0.70 0.20 25)",
};

export const STATUS_LABEL: Record<Status, string> = {
  calm: "Calm",
  neutral: "Neutral",
  elevated: "Elevated",
  stressed: "Stressed",
};

export const GROUPS = [
  { id: "rates", title: "US rates & fiscal stress", icon: "building-bank" },
  { id: "dollar", title: "Dollar & the yen carry trade", icon: "currency-dollar" },
  { id: "risk", title: "Risk appetite & havens", icon: "activity" },
] as const;

// --- Status thresholds (rules of thumb, transparent) -----------------------
// Each takes the numeric value and returns a Status. Kept simple on purpose;
// these are not trading signals.

const yieldStatus30 = (v: number): Status =>
  v >= 5 ? "stressed" : v >= 4.5 ? "elevated" : v >= 4 ? "neutral" : "calm";

const yieldStatus10 = (v: number): Status =>
  v >= 4.75 ? "stressed" : v >= 4.25 ? "elevated" : v >= 3.75 ? "neutral" : "calm";

const yieldStatus2 = (v: number): Status =>
  v >= 5 ? "stressed" : v >= 4.25 ? "elevated" : v >= 3.5 ? "neutral" : "calm";

const curveStatusBps = (bps: number): Status =>
  bps < -25 ? "stressed" : bps >= 100 ? "elevated" : "neutral";

const usdjpyStatus = (v: number): Status =>
  v >= 158 ? "stressed" : v >= 152 ? "elevated" : v >= 145 ? "neutral" : "calm";

const vixStatus = (v: number): Status =>
  v < 15 ? "calm" : v < 22 ? "neutral" : v < 30 ? "elevated" : "stressed";

const goldStatus = (v: number): Status =>
  v >= 3500 ? "elevated" : v >= 2500 ? "neutral" : "calm";

const brentStatus = (v: number): Status =>
  v >= 95 ? "elevated" : v >= 80 ? "neutral" : v >= 60 ? "neutral" : "calm";

// --- Formatters ------------------------------------------------------------

const pct = (v: number, dp = 2) => `${v.toFixed(dp)}%`;
const num1 = (v: number) => v.toFixed(1);
const dollars0 = (v: number) =>
  `$${v.toLocaleString("en-US", { maximumFractionDigits: 0 })}`;
const intCommas = (v: number) =>
  v.toLocaleString("en-US", { maximumFractionDigits: 0 });

// --- Fallback values (used when FRED key missing or all fetches fail) ------
// Snapshot from 2026-05-22 close. Keeps the dashboard rendering even with
// no API key configured.

export const FALLBACK: RawData = {
  ust30: { date: "2026-05-22", value: 5.06 },
  ust10: { date: "2026-05-22", value: 4.56 },
  ust2: { date: "2026-05-22", value: 4.13 },
  fedTargetUpper: { date: "2026-05-22", value: 3.75 },
  fedTargetLower: { date: "2026-05-22", value: 3.5 },
  dxyBroad: { date: "2026-05-22", value: 121.0 },
  usdjpy: { date: "2026-05-22", value: 159.2 },
  vix: { date: "2026-05-22", value: 16.7 },
  sp500: { date: "2026-05-22", value: 7473 },
  gold: { date: "2026-05-22", value: 4523 },
  brent: { date: "2026-05-22", value: 100 },
  bitcoin: 74600,
};

// --- Builder ---------------------------------------------------------------

const FMT_DATE = new Intl.DateTimeFormat("en-GB", {
  day: "numeric",
  month: "short",
  timeZone: "UTC",
});

function asOf(date: string | undefined): string {
  if (!date) return "—";
  const d = new Date(date + "T00:00:00Z");
  return `as of ${FMT_DATE.format(d)}`;
}

function latestDate(raw: RawData): string {
  const dates = [
    raw.ust30?.date,
    raw.ust10?.date,
    raw.ust2?.date,
    raw.vix?.date,
    raw.sp500?.date,
    raw.usdjpy?.date,
    raw.gold?.date,
    raw.brent?.date,
    raw.dxyBroad?.date,
  ].filter((d): d is string => Boolean(d));
  if (dates.length === 0) return FALLBACK.ust30!.date;
  return dates.sort().at(-1)!;
}

export function buildDashboard(raw: RawData): DashboardData {
  // Merge any missing fields with FALLBACK so a single failed series doesn't
  // wipe out the whole dashboard.
  const r: RawData = {
    ust30: raw.ust30 ?? FALLBACK.ust30,
    ust10: raw.ust10 ?? FALLBACK.ust10,
    ust2: raw.ust2 ?? FALLBACK.ust2,
    fedTargetUpper: raw.fedTargetUpper ?? FALLBACK.fedTargetUpper,
    fedTargetLower: raw.fedTargetLower ?? FALLBACK.fedTargetLower,
    dxyBroad: raw.dxyBroad ?? FALLBACK.dxyBroad,
    usdjpy: raw.usdjpy ?? FALLBACK.usdjpy,
    vix: raw.vix ?? FALLBACK.vix,
    sp500: raw.sp500 ?? FALLBACK.sp500,
    gold: raw.gold ?? FALLBACK.gold,
    brent: raw.brent ?? FALLBACK.brent,
    bitcoin: raw.bitcoin ?? FALLBACK.bitcoin,
  };

  const ust30v = r.ust30!.value;
  const ust10v = r.ust10!.value;
  const ust2v = r.ust2!.value;
  const fedHi = r.fedTargetUpper!.value;
  const fedLo = r.fedTargetLower!.value;
  const dxyV = r.dxyBroad!.value;
  const jpyV = r.usdjpy!.value;
  const vixV = r.vix!.value;
  const spV = r.sp500!.value;
  const goldV = r.gold!.value;
  const brentV = r.brent!.value;
  const btcV = r.bitcoin!;

  const curveBps = Math.round((ust10v - ust2v) * 100);

  const metrics: Metric[] = [
    // US rates & fiscal stress
    {
      id: "ust30",
      group: "rates",
      label: "30Y Treasury yield",
      value: pct(ust30v),
      status: yieldStatus30(ust30v),
      context: asOf(r.ust30?.date),
    },
    {
      id: "ust10",
      group: "rates",
      label: "10Y Treasury yield",
      value: pct(ust10v),
      status: yieldStatus10(ust10v),
      context: asOf(r.ust10?.date),
    },
    {
      id: "ust2",
      group: "rates",
      label: "2Y Treasury yield",
      value: pct(ust2v),
      status: yieldStatus2(ust2v),
      context: asOf(r.ust2?.date),
    },
    {
      id: "fedfunds",
      group: "rates",
      label: "Fed funds target",
      value: `${fedLo.toFixed(2)}–${fedHi.toFixed(2)}%`,
      status: "neutral",
      context: asOf(r.fedTargetUpper?.date),
    },
    {
      id: "curve",
      group: "rates",
      label: "2s/10s curve",
      value: `${curveBps >= 0 ? "+" : ""}${curveBps} bps`,
      status: curveStatusBps(curveBps),
      context: curveBps >= 50 ? "bear-steepening risk" : "flat / mid range",
    },
    // Dollar & the yen carry trade
    {
      id: "dxy",
      group: "dollar",
      label: "USD index (broad)",
      value: num1(dxyV),
      status: "neutral",
      context: "Fed broad TWI · proxy for DXY",
    },
    {
      id: "usdjpy",
      group: "dollar",
      label: "USD / JPY",
      value: num1(jpyV),
      status: usdjpyStatus(jpyV),
      context: jpyV >= 155 ? "near intervention zone (~160)" : asOf(r.usdjpy?.date),
    },
    {
      id: "jgb10",
      group: "dollar",
      label: "Japan 10Y JGB",
      value: "~2.8%",
      status: "elevated",
      context: "highest since ~1997 · static",
    },
    {
      id: "bojrate",
      group: "dollar",
      label: "BOJ policy rate",
      value: "0.75%",
      status: "elevated",
      context: "→ 1.0% expected June · static",
    },
    // Risk appetite & havens
    {
      id: "vix",
      group: "risk",
      label: "VIX (volatility)",
      value: num1(vixV),
      status: vixStatus(vixV),
      context: vixV < 18 ? "calm — watch divergence vs bonds" : asOf(r.vix?.date),
    },
    {
      id: "sp500",
      group: "risk",
      label: "S&P 500",
      value: intCommas(spV),
      status: "neutral",
      context: asOf(r.sp500?.date),
    },
    {
      id: "gold",
      group: "risk",
      label: "Gold",
      value: dollars0(goldV),
      status: goldStatus(goldV),
      context: goldV >= 3500 ? "haven / debasement bid" : asOf(r.gold?.date),
    },
    {
      id: "bitcoin",
      group: "risk",
      label: "Bitcoin",
      value: `$${(btcV / 1000).toFixed(1)}k`,
      status: "neutral",
      context: "CoinGecko spot",
    },
    {
      id: "brent",
      group: "risk",
      label: "Brent crude",
      value: `$${brentV.toFixed(1)}`,
      status: brentStatus(brentV),
      context: asOf(r.brent?.date),
    },
  ];

  const yieldCurve: CurvePoint[] = [
    { m: "2Y", y: ust2v },
    { m: "10Y", y: ust10v },
    { m: "30Y", y: ust30v },
  ];

  return { metrics, yieldCurve, snapshotDate: latestDate(r) };
}
