import "server-only";
import { buildDashboard, FALLBACK, type DashboardData, type Observation, type RawData } from "./metrics";
import { cached } from "./routeCache";

export type { Observation, RawData } from "./metrics";

const FRED_URL = "https://api.stlouisfed.org/fred/series/observations";
const TIMEOUT_MS = 6000;
const CLIENT_NAME = "market-monitor/0.1";

const today = () => new Date().toISOString().slice(0, 10);

/** A source that fails, times out or returns nonsense is "no reading", never a throw. */
async function reading(
  label: string,
  load: () => Promise<Observation | null>,
): Promise<Observation | null> {
  try {
    const obs = await load();
    return obs && Number.isFinite(obs.value) ? obs : null;
  } catch (err) {
    console.error(`dollar dashboard: ${label} unavailable:`, err instanceof Error ? err.message : err);
    return null;
  }
}

async function getJson<T>(url: string, init: RequestInit = {}): Promise<T> {
  const res = await fetch(url, {
    cache: "no-store",
    ...init,
    signal: AbortSignal.timeout(TIMEOUT_MS),
  });
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  return (await res.json()) as T;
}

async function fetchFredLatest(seriesId: string): Promise<Observation | null> {
  const key = process.env.FRED_API_KEY;
  if (!key) return null;
  const json = await getJson<{ observations?: { date: string; value: string }[] }>(
    `${FRED_URL}?series_id=${seriesId}&api_key=${key}&file_type=json&sort_order=desc&limit=10`,
  );
  // FRED returns "." for holidays / missing values. Pick the newest real one.
  const obs = json.observations?.find((o) => o.value && o.value !== ".");
  return obs ? { date: obs.date, value: Number(obs.value) } : null;
}

async function fetchBitcoinUsd(): Promise<Observation | null> {
  const json = await getJson<{ bitcoin?: { usd?: number } }>(
    "https://api.coingecko.com/api/v3/simple/price?ids=bitcoin&vs_currencies=usd",
  );
  const v = json.bitcoin?.usd;
  return typeof v === "number" ? { date: today(), value: v } : null;
}

// FRED dropped the LBMA gold series (GOLDPMGBD228NLBM) in 2022; the request
// now 404s. Gold API is the keyless spot source the desktop client uses.
async function fetchGoldSpot(): Promise<Observation | null> {
  const json = await getJson<{ price?: number; updatedAt?: string }>(
    "https://api.gold-api.com/price/XAU",
  );
  if (typeof json.price !== "number") return null;
  return { date: json.updatedAt?.slice(0, 10) || today(), value: json.price };
}

/** Japan 10Y yield and the BOJ policy rate. The scanner reports no trade time, so the date is the receipt date. */
async function fetchJapanRates(): Promise<{ jgb10: Observation | null; bojRate: Observation | null }> {
  const json = await getJson<{ data?: { s: string; d: (number | null)[] }[] }>(
    "https://scanner.tradingview.com/global/scan",
    {
      method: "POST",
      headers: { "Content-Type": "application/json", "User-Agent": CLIENT_NAME },
      body: JSON.stringify({
        symbols: { tickers: ["TVC:JP10Y", "ECONOMICS:JPINTR"] },
        columns: ["close"],
      }),
    },
  );
  const pick = (ticker: string): Observation | null => {
    const v = json.data?.find((row) => row.s === ticker)?.d?.[0];
    return typeof v === "number" && Number.isFinite(v) ? { date: today(), value: v } : null;
  };
  return { jgb10: pick("TVC:JP10Y"), bojRate: pick("ECONOMICS:JPINTR") };
}

export async function fetchRawData(): Promise<RawData> {
  const fred = (id: string) => reading(`FRED ${id}`, () => fetchFredLatest(id));
  const japan = fetchJapanRates().catch((err) => {
    console.error("dollar dashboard: Japan rates unavailable:", err instanceof Error ? err.message : err);
    return { jgb10: null, bojRate: null };
  });
  const [
    ust30,
    ust10,
    ust2,
    fedTargetUpper,
    fedTargetLower,
    dxyBroad,
    usdjpy,
    vix,
    sp500,
    gold,
    brent,
    bitcoin,
    japanRates,
  ] = await Promise.all([
    fred("DGS30"),
    fred("DGS10"),
    fred("DGS2"),
    fred("DFEDTARU"),
    fred("DFEDTARL"),
    fred("DTWEXBGS"),
    fred("DEXJPUS"),
    fred("VIXCLS"),
    fred("SP500"),
    reading("gold", fetchGoldSpot),
    fred("DCOILBRENTEU"),
    reading("bitcoin", fetchBitcoinUsd),
    japan,
  ]);
  return {
    ust30,
    ust10,
    ust2,
    fedTargetUpper,
    fedTargetLower,
    dxyBroad,
    usdjpy,
    vix,
    sp500,
    gold,
    brent,
    bitcoin,
    jgb10: japanRates.jgb10,
    bojRate: japanRates.bojRate,
  };
}

// Fifteen minutes when every source answered; two when any reading fell back,
// so a recovered source shows up quickly.
export async function getDashboardData(): Promise<DashboardData> {
  try {
    const hit = await cached<DashboardData>(
      "dollar-dashboard",
      { ttlSeconds: 900, degraded: (d) => d.fallbackCount > 0, degradedTtlSeconds: 120 },
      async () => buildDashboard(await fetchRawData()),
    );
    return hit.value;
  } catch (err) {
    console.error("dashboard fetch failed, using fallback snapshot:", err);
    return buildDashboard(FALLBACK);
  }
}
