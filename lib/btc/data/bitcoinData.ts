// bitcoin-data.com — free, no key, daily history.
// Provides pre-computed on-chain indicators like MVRV Z-Score, Realized Price,
// NUPL. Used because Coin Metrics community API no longer exposes CapRealUSD
// on its free tier (returns 403 forbidden).
//
// Response shape per metric is: [{ d: "YYYY-MM-DD", unixTs: 1234, <key>: number }, ...]
const BASE = "https://bitcoin-data.com/api/v1";

export type BdPoint = { t: number; v: number };

export async function fetchBitcoinDataSeries(
  path: string,
  valueKey: string,
): Promise<BdPoint[]> {
  const res = await fetch(`${BASE}/${path}`, {
    next: { revalidate: 21_600 }, // 6h
    headers: { Accept: "application/json" },
  });
  if (!res.ok) {
    throw new Error(
      `bitcoin-data.com ${path} ${res.status}: ${await res.text()}`,
    );
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
  return out;
}

export const fetchMvrvZ = () =>
  fetchBitcoinDataSeries("mvrv-zscore", "mvrvZscore");
export const fetchRealizedPrice = () =>
  fetchBitcoinDataSeries("realized-price", "realizedPrice");
export const fetchNupl = () => fetchBitcoinDataSeries("nupl", "nupl");
