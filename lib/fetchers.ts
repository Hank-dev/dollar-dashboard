import "server-only";
import { buildDashboard, FALLBACK, type DashboardData } from "./metrics";

const FRED_URL = "https://api.stlouisfed.org/fred/series/observations";

export type Observation = { date: string; value: number };

async function fetchFredLatest(seriesId: string): Promise<Observation | null> {
  const key = process.env.FRED_API_KEY;
  if (!key) return null;
  const url =
    `${FRED_URL}?series_id=${seriesId}&api_key=${key}` +
    `&file_type=json&sort_order=desc&limit=10`;
  const res = await fetch(url, {
    next: { revalidate: 900, tags: ["fred", `fred:${seriesId}`] },
  });
  if (!res.ok) return null;
  const json = (await res.json()) as {
    observations?: { date: string; value: string }[];
  };
  // FRED returns "." for holidays / missing values. Pick the newest real one.
  const obs = json.observations?.find((o) => o.value && o.value !== ".");
  if (!obs) return null;
  const value = Number(obs.value);
  if (!Number.isFinite(value)) return null;
  return { date: obs.date, value };
}

async function fetchBitcoinUsd(): Promise<number | null> {
  const res = await fetch(
    "https://api.coingecko.com/api/v3/simple/price?ids=bitcoin&vs_currencies=usd",
    { next: { revalidate: 900, tags: ["coingecko"] } },
  );
  if (!res.ok) return null;
  const json = (await res.json()) as { bitcoin?: { usd?: number } };
  const v = json.bitcoin?.usd;
  return typeof v === "number" && Number.isFinite(v) ? v : null;
}

export type RawData = {
  ust30: Observation | null;
  ust10: Observation | null;
  ust2: Observation | null;
  fedTargetUpper: Observation | null;
  fedTargetLower: Observation | null;
  dxyBroad: Observation | null; // FRED DTWEXBGS — broad trade-weighted USD, proxy for ICE DXY
  usdjpy: Observation | null;
  vix: Observation | null;
  sp500: Observation | null;
  gold: Observation | null;
  brent: Observation | null;
  bitcoin: number | null;
};

export async function fetchRawData(): Promise<RawData> {
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
  ] = await Promise.all([
    fetchFredLatest("DGS30"),
    fetchFredLatest("DGS10"),
    fetchFredLatest("DGS2"),
    fetchFredLatest("DFEDTARU"),
    fetchFredLatest("DFEDTARL"),
    fetchFredLatest("DTWEXBGS"),
    fetchFredLatest("DEXJPUS"),
    fetchFredLatest("VIXCLS"),
    fetchFredLatest("SP500"),
    fetchFredLatest("GOLDPMGBD228NLBM"),
    fetchFredLatest("DCOILBRENTEU"),
    fetchBitcoinUsd(),
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
  };
}

export async function getDashboardData(): Promise<DashboardData> {
  try {
    return buildDashboard(await fetchRawData());
  } catch (err) {
    console.error("dashboard fetch failed, using fallback snapshot:", err);
    return buildDashboard(FALLBACK);
  }
}
