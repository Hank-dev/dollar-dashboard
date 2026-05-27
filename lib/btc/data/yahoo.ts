// Yahoo Finance v8 chart API. No key required.
// Yahoo rate-limits aggressively and reacts to UA strings, so we send a real
// Chrome UA and fall back to the query2 host if query1 fails.
const HOSTS = [
  "https://query1.finance.yahoo.com",
  "https://query2.finance.yahoo.com",
];

const BROWSER_HEADERS: Record<string, string> = {
  "User-Agent":
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/121.0.0.0 Safari/537.36",
  Accept: "application/json, text/plain, */*",
  "Accept-Language": "en-US,en;q=0.9",
};

export type YahooPoint = { t: number; v: number };

type ChartResp = {
  chart: {
    result?: {
      timestamp?: number[];
      indicators: { quote: { close: (number | null)[] }[] };
    }[];
    error?: { description?: string } | null;
  };
};

async function fetchOne(
  host: string,
  symbol: string,
  range: string,
): Promise<YahooPoint[]> {
  const url = `${host}/v8/finance/chart/${encodeURIComponent(
    symbol,
  )}?range=${range}&interval=1d`;
  const res = await fetch(url, {
    next: { revalidate: 6 * 3600 },
    headers: BROWSER_HEADERS,
  });
  if (!res.ok) throw new Error(`Yahoo ${symbol} ${res.status}`);
  const j = (await res.json()) as ChartResp;
  if (j.chart.error) {
    throw new Error(`Yahoo ${symbol}: ${j.chart.error.description ?? "error"}`);
  }
  const r = j.chart.result?.[0];
  if (!r || !r.timestamp || !r.indicators.quote[0]?.close) {
    throw new Error(`Yahoo ${symbol}: empty response`);
  }
  const ts = r.timestamp;
  const closes = r.indicators.quote[0].close;
  const out: YahooPoint[] = [];
  for (let i = 0; i < ts.length; i++) {
    const v = closes[i];
    if (v == null || !Number.isFinite(v)) continue;
    out.push({ t: ts[i], v });
  }
  return out;
}

export async function fetchYahooDaily(
  symbol: string,
  range: "1mo" | "3mo" | "6mo" | "1y" | "2y" | "5y" = "3mo",
): Promise<YahooPoint[]> {
  let lastErr: unknown = null;
  for (const host of HOSTS) {
    try {
      return await fetchOne(host, symbol, range);
    } catch (e) {
      lastErr = e;
    }
  }
  throw lastErr instanceof Error
    ? lastErr
    : new Error(`Yahoo ${symbol}: all hosts failed`);
}
