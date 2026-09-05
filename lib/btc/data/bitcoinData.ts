// bitcoin-data.com — free, no key, daily history.
// Provides pre-computed on-chain indicators like MVRV Z-Score, Realized Price,
// NUPL. Used because Coin Metrics community API no longer exposes CapRealUSD
// on its free tier (returns 403 forbidden).
//
// Response shape per metric is: [{ d: "YYYY-MM-DD", unixTs: 1234, <key>: number }, ...]
const BASE = "https://bitcoin-data.com/api/v1";
const CACHE_TTL_MS = 3_600_000; // 1 hour

export type BdPoint = { t: number; v: number };

const seriesCache = new Map<
  string,
  { points: BdPoint[]; at: number }
>();

export async function fetchBitcoinDataSeries(
  path: string,
  valueKey: string,
): Promise<BdPoint[]> {
  const cacheKey = `${path}:${valueKey}`;
  const cached = seriesCache.get(cacheKey);
  if (cached && Date.now() - cached.at < CACHE_TTL_MS) {
    return cached.points;
  }

  const res = await fetch(`${BASE}/${path}`, {
    headers: { Accept: "application/json" },
  });
  if (!res.ok) {
    // Return stale cache on rate-limit if available
    if (res.status === 429 && cached) {
      return cached.points;
    }
    // On any failure without cache, return empty — caller handles nulls gracefully
    console.warn(`bitcoin-data.com ${path} ${res.status}`);
    return [];
  }
  const rows = (await res.json()) as Record<string, number | string>[];
  const out: BdPoint[] = [];
  for (const row of rows) {
    const ts = row.unixTs;
    const v = row[valueKey];
    if (typeof ts !== "number") continue;
    const num = typeof v === "number" ? v : parseFloat(String(v));
    if (!Number.isFinite(num)) continue;
    out.push({ t: ts, v: num });
  }
  out.sort((a, b) => a.t - b.t);
  seriesCache.set(cacheKey, { points: out, at: Date.now() });
  return out;
}

export const fetchMvrvZ = () =>
  fetchBitcoinDataSeries("mvrv-zscore", "mvrvZscore");
export const fetchRealizedPrice = () =>
  fetchBitcoinDataSeries("realized-price", "realizedPrice");
export const fetchNupl = () => fetchBitcoinDataSeries("nupl", "nupl");
