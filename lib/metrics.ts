export type Observation = { date: string; value: number };

export type RawData = {
  ust30: Observation | null;
  ust10: Observation | null;
  ust2: Observation | null;
  fedTargetUpper: Observation | null;
  fedTargetLower: Observation | null;
  dxyBroad: Observation | null; // FRED DTWEXBGS: broad trade-weighted USD (~120), not ICE DXY (~100)
  usdjpy: Observation | null;
  vix: Observation | null;
  sp500: Observation | null;
  gold: Observation | null;
  brent: Observation | null;
  bitcoin: Observation | null;
  jgb10: Observation | null;
  bojRate: Observation | null;
};

export type Status = "calm" | "neutral" | "elevated" | "stressed";

export interface Metric {
  id: string;
  group: "rates" | "dollar" | "risk";
  label: string;
  value: string;
  unit?: string;
  status: Status;
  /** Short note plus the reading's date; says so when the value is the built-in fallback. */
  context: string;
  /** ISO date of the reading. */
  asOf: string;
  /** True when no live source answered and the built-in snapshot value is shown. */
  fallback: boolean;
}

export interface CurvePoint {
  m: string;
  y: number;
}

export interface DashboardData {
  metrics: Metric[];
  yieldCurve: CurvePoint[];
  /** Newest reading date (ISO yyyy-mm-dd). Individual metrics can be older. */
  snapshotDate: string;
  /** Oldest reading date on the page. */
  oldestDate: string;
  /** How many metrics show the built-in fallback instead of a live reading. */
  fallbackCount: number;
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
  v >= 4000 ? "elevated" : v >= 2500 ? "neutral" : "calm";

// Same thresholds as the desktop client.
const jgbStatus = (v: number): Status =>
  v >= 3.5 ? "stressed" : v >= 2 ? "elevated" : "neutral";

const bojStatus = (v: number): Status =>
  v >= 1 ? "stressed" : v >= 0.5 ? "elevated" : "neutral";

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
// Snapshot refreshed 2026-09-03 using the latest official Treasury close (Sep 2)
// and current September 3 live quotes. Verified against the U.S. Treasury, CBOE,
// CoinGecko, Gold API, exchangerate-api, FRED, and TradingView.
// Keeps the dashboard rendering even with no API key configured.

export const FALLBACK: RawData = {
  ust30: { date: "2026-09-02", value: 5.27 }, // U.S. Treasury: https://home.treasury.gov/resource-center/data-chart-center/interest-rates/pages/xml?data=daily_treasury_yield_curve&field_tdr_date_value=2026 (Sep 2 close, 5.27%)
  ust10: { date: "2026-09-02", value: 4.79 }, // U.S. Treasury: https://home.treasury.gov/resource-center/data-chart-center/interest-rates/pages/xml?data=daily_treasury_yield_curve&field_tdr_date_value=2026 (Sep 2 close, 4.79%)
  ust2: { date: "2026-09-02", value: 4.39 }, // U.S. Treasury: https://home.treasury.gov/resource-center/data-chart-center/interest-rates/pages/xml?data=daily_treasury_yield_curve&field_tdr_date_value=2026 (Sep 2 close, 4.39%)
  fedTargetUpper: { date: "2026-09-02", value: 3.75 }, // Federal Reserve: https://fred.stlouisfed.org/graph/fredgraph.csv?id=DFEDTARU (Sep 2, upper bound 3.75%; no FOMC since, unchanged)
  fedTargetLower: { date: "2026-09-02", value: 3.5 }, // Federal Reserve: https://fred.stlouisfed.org/graph/fredgraph.csv?id=DFEDTARL (Sep 2, lower bound 3.50%; unchanged)
  dxyBroad: { date: "2026-09-02", value: 118.5296 }, // FRED: https://fred.stlouisfed.org/graph/fredgraph.csv?id=DTWEXBGS (Sep 2, 118.5296). Must be the broad index, the same series as the live reading; ICE DXY is a different scale.
  usdjpy: { date: "2026-09-03", value: 159.088772 }, // exchangerate-api: https://open.er-api.com/v6/latest/USD (Sep 3 mid, 159.088772)
  vix: { date: "2026-09-02", value: 15.2 }, // CBOE: https://cdn.cboe.com/api/global/delayed_quotes/quotes/_VIX.json (Sep 2 close, 15.20)
  sp500: { date: "2026-09-02", value: 7666.6 }, // FRED: https://fred.stlouisfed.org/graph/fredgraph.csv?id=SP500 (Sep 2 close, 7,666.60)
  gold: { date: "2026-09-03", value: 4433.4 }, // Gold API: https://api.gold-api.com/price/XAU (Sep 3 spot, $4,433.40/oz)
  brent: { date: "2026-09-03", value: 94.26 }, // TradingView: https://scanner.tradingview.com/futures/scan (NYMEX:BZ1! front month, Sep 3 quote, $94.26/bbl)
  bitcoin: { date: "2026-09-03", value: 77751.0 }, // CoinGecko: https://api.coingecko.com/api/v3/simple/price?ids=bitcoin&vs_currencies=usd (Sep 3 spot, $77,751)
  jgb10: { date: "2026-09-03", value: 2.954 }, // TradingView: https://scanner.tradingview.com/global/scan (TVC:JP10Y, Sep 3, 2.954%)
  bojRate: { date: "2026-09-03", value: 1.0 }, // BOJ: https://www.boj.or.jp/en/mopo/mpmdeci/mpr_2026/k260731a.pdf (held at 1.0% on Jul 31 2026)
};

// --- Builder ---------------------------------------------------------------

const FMT_DATE = new Intl.DateTimeFormat("en-GB", {
  day: "numeric",
  month: "short",
  timeZone: "UTC",
});

function shortDate(date: string): string {
  return FMT_DATE.format(new Date(date + "T00:00:00Z"));
}

type Field = keyof RawData;

export function buildDashboard(raw: RawData): DashboardData {
  // A missing reading falls back to the built-in snapshot value so one failed
  // source doesn't blank the page, but the card says so and keeps its date.
  const used = (field: Field): Observation => raw[field] ?? FALLBACK[field]!;
  const isFallback = (field: Field): boolean => raw[field] == null;

  /** Optional note, then the reading's date; fallback values are named as such. */
  const describe = (fields: Field[], note = ""): Pick<Metric, "context" | "asOf" | "fallback"> => {
    const fallback = fields.some(isFallback);
    const asOf = fields.map((f) => used(f).date).sort()[0];
    const dated = fallback ? `fallback snapshot · ${shortDate(asOf)}` : `as of ${shortDate(asOf)}`;
    return { context: note ? `${note} · ${dated}` : dated, asOf, fallback };
  };

  const ust30v = used("ust30").value;
  const ust10v = used("ust10").value;
  const ust2v = used("ust2").value;
  const fedHi = used("fedTargetUpper").value;
  const fedLo = used("fedTargetLower").value;
  const dxyV = used("dxyBroad").value;
  const jpyV = used("usdjpy").value;
  const vixV = used("vix").value;
  const spV = used("sp500").value;
  const goldV = used("gold").value;
  const brentV = used("brent").value;
  const btcV = used("bitcoin").value;
  const jgbV = used("jgb10").value;
  const bojV = used("bojRate").value;

  const curveBps = Math.round((ust10v - ust2v) * 100);

  const metrics: Metric[] = [
    // US rates & fiscal stress
    {
      id: "ust30",
      group: "rates",
      label: "30Y Treasury yield",
      value: pct(ust30v),
      status: yieldStatus30(ust30v),
      ...describe(["ust30"]),
    },
    {
      id: "ust10",
      group: "rates",
      label: "10Y Treasury yield",
      value: pct(ust10v),
      status: yieldStatus10(ust10v),
      ...describe(["ust10"]),
    },
    {
      id: "ust2",
      group: "rates",
      label: "2Y Treasury yield",
      value: pct(ust2v),
      status: yieldStatus2(ust2v),
      ...describe(["ust2"]),
    },
    {
      id: "fedfunds",
      group: "rates",
      label: "Fed funds target",
      value: `${fedLo.toFixed(2)}–${fedHi.toFixed(2)}%`,
      status: "neutral",
      ...describe(["fedTargetUpper", "fedTargetLower"]),
    },
    {
      id: "curve",
      group: "rates",
      label: "2s/10s curve",
      value: `${curveBps >= 0 ? "+" : ""}${curveBps} bps`,
      status: curveStatusBps(curveBps),
      ...describe(["ust10", "ust2"], curveBps >= 50 ? "bear-steepening risk" : "flat / mid range"),
    },
    // Dollar & the yen carry trade
    {
      id: "dxy",
      group: "dollar",
      label: "USD index (broad)",
      value: num1(dxyV),
      status: "neutral",
      ...describe(["dxyBroad"], "trade-weighted, not ICE DXY"),
    },
    {
      id: "usdjpy",
      group: "dollar",
      label: "USD / JPY",
      value: num1(jpyV),
      status: usdjpyStatus(jpyV),
      ...describe(["usdjpy"], jpyV >= 155 ? "near intervention zone (~160)" : ""),
    },
    {
      id: "jgb10",
      group: "dollar",
      label: "Japan 10Y JGB",
      value: pct(jgbV),
      status: jgbStatus(jgbV),
      ...describe(["jgb10"]),
    },
    {
      id: "bojrate",
      group: "dollar",
      label: "BOJ policy rate",
      value: pct(bojV),
      status: bojStatus(bojV),
      ...describe(["bojRate"], bojV >= 1 ? "squeezes the yen carry trade" : ""),
    },
    // Risk appetite & havens
    {
      id: "vix",
      group: "risk",
      label: "VIX (volatility)",
      value: num1(vixV),
      status: vixStatus(vixV),
      ...describe(["vix"], vixV < 18 ? "calm — watch divergence vs bonds" : ""),
    },
    {
      id: "sp500",
      group: "risk",
      label: "S&P 500",
      value: intCommas(spV),
      status: "neutral",
      ...describe(["sp500"]),
    },
    {
      id: "gold",
      group: "risk",
      label: "Gold",
      value: dollars0(goldV),
      status: goldStatus(goldV),
      ...describe(["gold"], goldV >= 4000 ? "haven / debasement bid" : ""),
    },
    {
      id: "bitcoin",
      group: "risk",
      label: "Bitcoin",
      value: `$${(btcV / 1000).toFixed(1)}k`,
      status: "neutral",
      ...describe(["bitcoin"], "CoinGecko spot"),
    },
    {
      id: "brent",
      group: "risk",
      label: "Brent crude",
      value: `$${brentV.toFixed(1)}`,
      status: brentStatus(brentV),
      ...describe(["brent"]),
    },
  ];

  const yieldCurve: CurvePoint[] = [
    { m: "2Y", y: ust2v },
    { m: "10Y", y: ust10v },
    { m: "30Y", y: ust30v },
  ];

  const dates = metrics.map((m) => m.asOf).sort();
  return {
    metrics,
    yieldCurve,
    snapshotDate: dates[dates.length - 1],
    oldestDate: dates[0],
    fallbackCount: metrics.filter((m) => m.fallback).length,
  };
}
