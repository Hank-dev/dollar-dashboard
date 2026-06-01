// Generic equity quote fetcher (Yahoo → Stooq fallback). NOT server-only:
// imported by API routes, the /ai server page, and scripts. Extracted from
// lib/nuclearLive.ts so multiple dashboards share one quote path.
// Note: the Stooq fallback assumes US-listed symbols (it appends ".us").

export interface EquityQuote {
  symbol: string;
  name: string | null;
  price: number | null;
  changePercent: number | null;
  marketCapUsd: number | null;
  marketCapSource: "live" | "snapshot" | "unavailable";
  currency: string;
  exchange: string | null;
  asOfUnix: number | null;
  sourceUrl: string;
}

export interface FetchEquityQuotesOptions {
  snapshotMarketCapUsd?: Record<string, number>;
  nameBySymbol?: Record<string, string>;
  revalidateSeconds?: number;
}

export interface EquityQuotesResult {
  quotes: EquityQuote[];
  source: "yahoo" | "stooq" | "none";
  errors: string[];
}

const DEFAULT_REVALIDATE = 3600;
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

type YahooQuote = {
  symbol?: string;
  shortName?: string;
  longName?: string;
  regularMarketPrice?: number;
  regularMarketChangePercent?: number;
  marketCap?: number;
  currency?: string;
  fullExchangeName?: string;
  exchange?: string;
  regularMarketTime?: number;
};

type YahooQuoteResponse = {
  quoteResponse?: { result?: YahooQuote[]; error?: { description?: string } | null };
};

export async function fetchEquityQuotes(
  symbols: string[],
  opts: FetchEquityQuotesOptions = {},
): Promise<EquityQuotesResult> {
  const errors: string[] = [];
  try {
    return { quotes: await fetchYahooQuotes(symbols, opts), source: "yahoo", errors };
  } catch (err) {
    errors.push(`Yahoo: ${err instanceof Error ? err.message : String(err)}`);
  }
  try {
    return { quotes: await fetchStooqQuotes(symbols, opts), source: "stooq", errors };
  } catch (err) {
    errors.push(`Stooq: ${err instanceof Error ? err.message : String(err)}`);
  }
  return { quotes: [], source: "none", errors };
}

export function formatMarketCapUsd(value: number | null): string | null {
  if (value == null || !Number.isFinite(value) || value <= 0) return null;
  if (value >= 1e12) return `$${(value / 1e12).toFixed(1)}T`;
  if (value >= 1e9) return `$${Math.round(value / 1e9)}B`;
  if (value >= 1e6) return `$${Math.round(value / 1e6)}M`;
  return `$${Math.round(value)}`;
}

async function fetchYahooQuotes(
  symbols: string[],
  opts: FetchEquityQuotesOptions,
): Promise<EquityQuote[]> {
  let lastErr: unknown = null;
  for (const host of HOSTS) {
    try {
      return await fetchYahooQuotesFromHost(host, symbols, opts);
    } catch (err) {
      lastErr = err;
    }
  }
  throw lastErr instanceof Error ? lastErr : new Error("Yahoo quote request failed.");
}

async function fetchYahooQuotesFromHost(
  host: string,
  symbols: string[],
  opts: FetchEquityQuotesOptions,
): Promise<EquityQuote[]> {
  const revalidate = opts.revalidateSeconds ?? DEFAULT_REVALIDATE;
  const url = `${host}/v7/finance/quote?symbols=${symbols.map(encodeURIComponent).join(",")}`;
  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), 3500);
  const res = await fetch(url, {
    next: { revalidate },
    headers: BROWSER_HEADERS,
    signal: controller.signal,
  }).finally(() => clearTimeout(timeout));
  if (!res.ok) throw new Error(`Yahoo quote API returned ${res.status}`);

  const body = (await res.json()) as YahooQuoteResponse;
  const error = body.quoteResponse?.error;
  if (error) throw new Error(error.description ?? "Yahoo quote API error");

  return (body.quoteResponse?.result ?? [])
    .map((raw) => normalizeYahoo(raw, opts))
    .filter((quote): quote is EquityQuote => Boolean(quote));
}

function normalizeYahoo(raw: YahooQuote, opts: FetchEquityQuotesOptions): EquityQuote | null {
  if (!raw.symbol) return null;
  const symbol = raw.symbol.toUpperCase();
  const liveCap = finiteOrNull(raw.marketCap);
  const snapshotCap = opts.snapshotMarketCapUsd?.[symbol] ?? null;
  return {
    symbol,
    name: raw.shortName ?? raw.longName ?? opts.nameBySymbol?.[symbol] ?? symbol,
    price: finiteOrNull(raw.regularMarketPrice),
    changePercent: finiteOrNull(raw.regularMarketChangePercent),
    marketCapUsd: liveCap ?? snapshotCap,
    marketCapSource: liveCap != null ? "live" : snapshotCap != null ? "snapshot" : "unavailable",
    currency: raw.currency ?? "USD",
    exchange: raw.fullExchangeName ?? raw.exchange ?? null,
    asOfUnix: finiteOrNull(raw.regularMarketTime),
    sourceUrl: `https://finance.yahoo.com/quote/${encodeURIComponent(symbol)}`,
  };
}

async function fetchStooqQuotes(
  symbols: string[],
  opts: FetchEquityQuotesOptions,
): Promise<EquityQuote[]> {
  const settled = await Promise.allSettled(symbols.map((s) => fetchStooqQuote(s, opts)));
  const quotes: EquityQuote[] = [];
  const errors: string[] = [];
  settled.forEach((result, index) => {
    if (result.status === "fulfilled") quotes.push(result.value);
    else errors.push(`${symbols[index]} ${result.reason instanceof Error ? result.reason.message : "failed"}`);
  });
  if (quotes.length === 0) throw new Error(errors.join("; ") || "No Stooq quotes returned");
  return quotes;
}

async function fetchStooqQuote(symbol: string, opts: FetchEquityQuotesOptions): Promise<EquityQuote> {
  const revalidate = opts.revalidateSeconds ?? DEFAULT_REVALIDATE;
  const stooqSymbol = `${symbol.toLowerCase()}.us`;
  const url = `https://stooq.com/q/l/?s=${encodeURIComponent(stooqSymbol)}&f=sd2t2ohlcv&h`;
  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), 4500);
  const res = await fetch(url, {
    next: { revalidate },
    headers: { Accept: "text/csv,*/*" },
    signal: controller.signal,
  }).finally(() => clearTimeout(timeout));
  if (!res.ok) throw new Error(`returned ${res.status}`);

  const csv = await res.text();
  const [, row] = csv.trim().split(/\r?\n/);
  if (!row) throw new Error("empty response");

  const [rawSymbol, date, time, openRaw, , , closeRaw] = row.split(",");
  if (!rawSymbol || date === "N/D" || closeRaw === "N/D") throw new Error("no quote data");

  const open = Number(openRaw);
  const close = Number(closeRaw);
  const changePercent =
    Number.isFinite(open) && open > 0 && Number.isFinite(close)
      ? ((close - open) / open) * 100
      : null;
  const marketTime = Date.parse(`${date}T${time || "00:00:00"}Z`);
  const snapshotCap = opts.snapshotMarketCapUsd?.[symbol.toUpperCase()] ?? null;

  return {
    symbol: symbol.toUpperCase(),
    name: opts.nameBySymbol?.[symbol.toUpperCase()] ?? symbol.toUpperCase(),
    price: Number.isFinite(close) ? close : null,
    changePercent,
    marketCapUsd: snapshotCap,
    marketCapSource: snapshotCap != null ? "snapshot" : "unavailable",
    currency: "USD",
    exchange: "Stooq",
    asOfUnix: Number.isFinite(marketTime) ? Math.floor(marketTime / 1000) : null,
    sourceUrl: `https://stooq.com/q/?s=${encodeURIComponent(stooqSymbol)}`,
  };
}

function finiteOrNull(value: unknown): number | null {
  return typeof value === "number" && Number.isFinite(value) ? value : null;
}
