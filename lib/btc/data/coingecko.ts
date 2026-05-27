// CoinGecko public API. Free tier, no key.
// Daily historical data is capped at the last 365 days for free users.
const BASE = "https://api.coingecko.com/api/v3";

export type CgPoint = { t: number; v: number };

type MarketChartResp = {
  prices: [number, number][]; // [timestamp_ms, price_usd]
};

export async function fetchCoingeckoDaily(
  coinId: string,
  days = 365,
  revalidateSec = 6 * 3600,
): Promise<CgPoint[]> {
  const url =
    `${BASE}/coins/${encodeURIComponent(coinId)}/market_chart` +
    `?vs_currency=usd&days=${days}&interval=daily`;
  const res = await fetch(url, {
    next: { revalidate: revalidateSec },
    headers: { Accept: "application/json" },
  });
  if (!res.ok) {
    throw new Error(`CoinGecko ${coinId} ${res.status}: ${await res.text()}`);
  }
  const j = (await res.json()) as MarketChartResp;
  return j.prices
    .map((p) => ({ t: Math.floor(p[0] / 1000), v: p[1] }))
    .filter((p) => Number.isFinite(p.t) && Number.isFinite(p.v) && p.v > 0);
}
