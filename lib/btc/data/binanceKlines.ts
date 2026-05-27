// Spot OHLCV. Originally Binance, then Bybit — both return HTTP 451 / 403
// from US-hosted runtimes (Railway, Vercel, most AWS regions). Now uses
// Kraken, which is US-domiciled and has a fully open public market-data API.
// Pair "BTCUSDT" is mapped to Kraken's "XBTUSDT". Quote volume is derived
// as vwap * base volume since Kraken's OHLC doesn't return it directly.
// File/function names are kept for caller compatibility.

const BASE = "https://api.kraken.com";

export type Kline = {
  t: number; // unix seconds, candle open time
  open: number;
  high: number;
  low: number;
  close: number;
  volume: number; // base asset (BTC)
  quoteVolume: number; // quote asset (USDT, ~USD)
};

type KrakenInterval = 60 | 240 | 1440;

function toKrakenInterval(i: "1h" | "4h" | "1d"): KrakenInterval {
  if (i === "1h") return 60;
  if (i === "4h") return 240;
  return 1440;
}

// [time, open, high, low, close, vwap, volume, count]
type KrakenOhlcRow = [
  number,
  string,
  string,
  string,
  string,
  string,
  string,
  number,
];
type KrakenResponse = {
  error: string[];
  result: Record<string, KrakenOhlcRow[] | number>;
};

export async function fetchBinanceKlines(
  symbol: string,
  interval: "1h" | "4h" | "1d",
  limit = 500,
  revalidateSec = 3600,
): Promise<Kline[]> {
  const krakenPair = symbol === "BTCUSDT" ? "XBTUSDT" : symbol;
  const url =
    `${BASE}/0/public/OHLC?pair=${encodeURIComponent(krakenPair)}` +
    `&interval=${toKrakenInterval(interval)}`;
  const res = await fetch(url, {
    next: { revalidate: revalidateSec },
    headers: { Accept: "application/json" },
  });
  if (!res.ok) {
    throw new Error(`Kraken OHLC ${symbol} ${interval} ${res.status}`);
  }
  const j = (await res.json()) as KrakenResponse;
  if (j.error.length > 0) {
    throw new Error(
      `Kraken OHLC ${symbol} ${interval}: ${j.error.join(", ")}`,
    );
  }
  // Kraken occasionally normalizes pair keys (e.g. XBTUSDT -> XXBTZUSD for
  // some legacy pairs), so pick the first array-valued entry rather than
  // assuming the key matches the requested pair.
  let rows: KrakenOhlcRow[] | null = null;
  for (const v of Object.values(j.result)) {
    if (Array.isArray(v)) {
      rows = v as KrakenOhlcRow[];
      break;
    }
  }
  if (!rows) {
    throw new Error(`Kraken OHLC ${symbol}: no series in response`);
  }

  const out: Kline[] = [];
  for (const r of rows) {
    const t = r[0];
    const open = parseFloat(r[1]);
    const high = parseFloat(r[2]);
    const low = parseFloat(r[3]);
    const close = parseFloat(r[4]);
    const vwap = parseFloat(r[5]);
    const volume = parseFloat(r[6]);
    const quoteVolume =
      Number.isFinite(vwap) && Number.isFinite(volume) ? vwap * volume : NaN;
    if (
      !Number.isFinite(t) ||
      !Number.isFinite(close) ||
      !Number.isFinite(quoteVolume)
    )
      continue;
    out.push({ t, open, high, low, close, volume, quoteVolume });
  }
  // Kraken returns up to ~720 rows in chronological order; slice to caller's
  // requested window from the tail (most recent).
  return out.slice(-limit);
}

// Daily BTCUSDT, last ~720 days (Kraken's per-call cap, ~2 years). Plenty for
// MEI normalization windows. USDT quote volume ≈ USD.
export async function fetchBtcDailyVolume(): Promise<
  { t: number; v: number }[]
> {
  const klines = await fetchBinanceKlines("BTCUSDT", "1d", 1000, 21_600);
  return klines.map((k) => ({ t: k.t, v: k.quoteVolume }));
}
