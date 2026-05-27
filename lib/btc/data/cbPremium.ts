// Coinbase premium: spot BTC-USD on Coinbase vs BTCUSDT on Binance.
// Positive = Coinbase trades higher = US demand strong.
// Negative = Coinbase trades lower = US demand weak.
import { fetchBinanceKlines } from "@/lib/btc/data/binanceKlines";

export type PremiumPoint = {
  t: number; // unix seconds, hour-aligned
  cb: number;
  bn: number;
  premiumBps: number; // (cb - bn) / bn * 10_000
};

// Coinbase Exchange candles: array of [time, low, high, open, close, volume].
type CbCandle = [number, number, number, number, number, number];

async function fetchCoinbaseHourly(): Promise<{ t: number; close: number }[]> {
  const url =
    "https://api.exchange.coinbase.com/products/BTC-USD/candles?granularity=3600";
  const res = await fetch(url, {
    next: { revalidate: 600 },
    headers: {
      Accept: "application/json",
      "User-Agent": "btc-terminal/1.0 (single-user dashboard)",
    },
  });
  if (!res.ok) throw new Error(`Coinbase candles ${res.status}`);
  const j = (await res.json()) as CbCandle[];
  return j
    .map((c) => ({ t: c[0], close: c[4] }))
    .filter((c) => Number.isFinite(c.t) && Number.isFinite(c.close))
    .sort((a, b) => a.t - b.t);
}

async function fetchBinanceHourly(): Promise<{ t: number; close: number }[]> {
  const klines = await fetchBinanceKlines("BTCUSDT", "1h", 72, 600);
  return klines.map((k) => ({ t: k.t, close: k.close }));
}

export type PremiumResult = {
  series: PremiumPoint[]; // chronological, last ~48 hourly points
  current: PremiumPoint | null;
  avg24hBps: number | null;
};

export async function fetchCoinbasePremium(
  hours = 48,
): Promise<PremiumResult> {
  const [cb, bn] = await Promise.all([
    fetchCoinbaseHourly(),
    fetchBinanceHourly(),
  ]);
  const cbMap = new Map<number, number>();
  for (const c of cb) cbMap.set(c.t, c.close);

  const aligned: PremiumPoint[] = [];
  for (const k of bn) {
    const cbClose = cbMap.get(k.t);
    if (cbClose == null) continue;
    if (k.close <= 0) continue;
    aligned.push({
      t: k.t,
      cb: cbClose,
      bn: k.close,
      premiumBps: ((cbClose - k.close) / k.close) * 10_000,
    });
  }
  aligned.sort((a, b) => a.t - b.t);
  const series = aligned.slice(-hours);
  const current = series.length > 0 ? series[series.length - 1] : null;
  const last24 = series.slice(-24);
  const avg24hBps =
    last24.length > 0
      ? last24.reduce((s, p) => s + p.premiumBps, 0) / last24.length
      : null;

  return { series, current, avg24hBps };
}
