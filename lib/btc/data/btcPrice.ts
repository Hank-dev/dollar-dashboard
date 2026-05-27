// Daily BTC price history.
// CoinGecko free tier caps public users at 365 days, which is useless for a
// power-law fit. Coin Metrics community API gives full daily history (no key,
// rate-limited at 10 req/sec) so we use PriceUSD as the canonical daily close.
import { fetchBtcMetrics } from "./coinmetrics";

export type RawDaily = { t: number; v: number };

export async function fetchBtcDailyHistory(): Promise<RawDaily[]> {
  const out = await fetchBtcMetrics(["PriceUSD"]);
  return out.PriceUSD;
}
