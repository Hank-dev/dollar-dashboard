import { fetchFredSeries } from "./btc/data/fred";

export interface NuclearIndexPoint {
  date: string;
  value: number;
}

export interface NuclearUraniumIndex {
  label: string;
  unit: string;
  source: string;
  sourceUrl: string;
  asOf: string | null;
  latest: number | null;
  change12mPercent: number | null;
  points: NuclearIndexPoint[];
  note: string;
}

export interface NuclearQuote {
  symbol: string;
  name: string;
  price: number | null;
  changePercent: number | null;
  marketCapUsd: number | null;
  marketCapSource: "live" | "snapshot" | "unavailable";
  currency: string;
  exchange: string | null;
  regularMarketTime: number | null;
  sourceUrl: string;
}

export interface NuclearMarketLiveResponse {
  checkedAt: string;
  refreshCadenceSeconds: number;
  source: string;
  note: string;
  publicEquities: NuclearQuote[];
  uraniumProxies: NuclearQuote[];
  totalPublicMarketCapUsd: number | null;
  uraniumIndex: NuclearUraniumIndex | null;
  errors: string[];
}

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
  quoteResponse?: {
    result?: YahooQuote[];
    error?: { description?: string } | null;
  };
};

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

const REFRESH_CADENCE_SECONDS = 3600;

const PUBLIC_SYMBOLS = [
  "CEG",
  "VST",
  "CCJ",
  "BWXT",
  "OKLO",
  "UEC",
  "SMR",
  "LEU",
];

const URANIUM_PROXY_SYMBOLS = ["URA", "URNM", "NLR"];

const MARKET_CAP_USD_BY_SYMBOL: Record<string, number> = {
  CEG: 108.9e9,
  VST: 55.5e9,
  CCJ: 46.6e9,
  BWXT: 18.7e9,
  OKLO: 12.0e9,
  UEC: 6.4e9,
  SMR: 4.5e9,
  LEU: 3.5e9,
};

const NAME_BY_SYMBOL: Record<string, string> = {
  CEG: "Constellation Energy",
  VST: "Vistra",
  CCJ: "Cameco",
  BWXT: "BWX Technologies",
  OKLO: "Oklo",
  UEC: "Uranium Energy",
  SMR: "NuScale Power",
  LEU: "Centrus Energy",
  URA: "Global X Uranium ETF",
  URNM: "Sprott Uranium Miners ETF",
  NLR: "VanEck Uranium and Nuclear ETF",
};

export async function fetchNuclearMarketLive(): Promise<NuclearMarketLiveResponse> {
  const checkedAt = new Date().toISOString();
  const symbols = [...PUBLIC_SYMBOLS, ...URANIUM_PROXY_SYMBOLS];
  const errors: string[] = [];
  let source = "Yahoo Finance quote API";
  let note =
    "Public equities use live/delayed exchange quotes. Market caps fall back to the dashboard snapshot if the quote source does not provide them. Uranium proxies are equity ETFs.";

  let quotes: NuclearQuote[] = [];
  let uraniumIndex: NuclearUraniumIndex | null = null;
  try {
    quotes = await fetchYahooQuotes(symbols);
  } catch (err) {
    errors.push(`Yahoo Finance: ${err instanceof Error ? err.message : String(err)}`);
    try {
      quotes = await fetchStooqQuotes(symbols);
      source = "Stooq quote CSV fallback";
      note =
        "Public equities use delayed Stooq quotes. Market caps use the dashboard snapshot because Stooq does not provide them; percentage moves are calculated from the daily open. Uranium proxies are equity ETFs.";
    } catch (fallbackErr) {
      errors.push(
        `Stooq: ${
          fallbackErr instanceof Error ? fallbackErr.message : String(fallbackErr)
        }`,
      );
    }
  }

  try {
    uraniumIndex = await fetchUraniumIndex();
  } catch (err) {
    errors.push(`FRED uranium index: ${err instanceof Error ? err.message : String(err)}`);
  }

  const bySymbol = new Map(quotes.map((quote) => [quote.symbol, quote]));
  const publicEquities = PUBLIC_SYMBOLS.map((symbol) => bySymbol.get(symbol)).filter(
    (quote): quote is NuclearQuote => Boolean(quote),
  );
  const uraniumProxies = URANIUM_PROXY_SYMBOLS.map((symbol) =>
    bySymbol.get(symbol),
  ).filter((quote): quote is NuclearQuote => Boolean(quote));
  const totalPublicMarketCapUsd = sumMarketCap(publicEquities);

  return {
    checkedAt,
    refreshCadenceSeconds: REFRESH_CADENCE_SECONDS,
    source,
    note,
    publicEquities,
    uraniumProxies,
    totalPublicMarketCapUsd,
    uraniumIndex,
    errors,
  };
}

async function fetchYahooQuotes(symbols: string[]): Promise<NuclearQuote[]> {
  let lastErr: unknown = null;
  for (const host of HOSTS) {
    try {
      return await fetchYahooQuotesFromHost(host, symbols);
    } catch (err) {
      lastErr = err;
    }
  }
  throw lastErr instanceof Error
    ? lastErr
    : new Error("Yahoo quote request failed.");
}

async function fetchYahooQuotesFromHost(
  host: string,
  symbols: string[],
): Promise<NuclearQuote[]> {
  const url = `${host}/v7/finance/quote?symbols=${symbols
    .map(encodeURIComponent)
    .join(",")}`;
  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), 3500);
  const res = await fetch(url, {
    next: { revalidate: REFRESH_CADENCE_SECONDS },
    headers: BROWSER_HEADERS,
    signal: controller.signal,
  }).finally(() => clearTimeout(timeout));
  if (!res.ok) throw new Error(`Yahoo quote API returned ${res.status}`);

  const body = (await res.json()) as YahooQuoteResponse;
  const error = body.quoteResponse?.error;
  if (error) throw new Error(error.description ?? "Yahoo quote API error");

  return (body.quoteResponse?.result ?? [])
    .map(normalizeQuote)
    .filter((quote): quote is NuclearQuote => Boolean(quote));
}

function normalizeQuote(raw: YahooQuote): NuclearQuote | null {
  if (!raw.symbol) return null;
  const symbol = raw.symbol.toUpperCase();
  const marketCap = finiteOrNull(raw.marketCap);
  const snapshotMarketCap = MARKET_CAP_USD_BY_SYMBOL[symbol] ?? null;
  return {
    symbol,
    name: raw.shortName ?? raw.longName ?? symbol,
    price: finiteOrNull(raw.regularMarketPrice),
    changePercent: finiteOrNull(raw.regularMarketChangePercent),
    marketCapUsd: marketCap ?? snapshotMarketCap,
    marketCapSource:
      marketCap != null ? "live" : snapshotMarketCap != null ? "snapshot" : "unavailable",
    currency: raw.currency ?? "USD",
    exchange: raw.fullExchangeName ?? raw.exchange ?? null,
    regularMarketTime: finiteOrNull(raw.regularMarketTime),
    sourceUrl: `https://finance.yahoo.com/quote/${encodeURIComponent(symbol)}`,
  };
}

async function fetchStooqQuotes(symbols: string[]): Promise<NuclearQuote[]> {
  const settled = await Promise.allSettled(
    symbols.map((symbol) => fetchStooqQuote(symbol)),
  );
  const quotes: NuclearQuote[] = [];
  const errors: string[] = [];

  settled.forEach((result, index) => {
    if (result.status === "fulfilled") {
      quotes.push(result.value);
    } else {
      errors.push(
        `${symbols[index]} ${
          result.reason instanceof Error ? result.reason.message : "failed"
        }`,
      );
    }
  });

  if (quotes.length === 0) {
    throw new Error(errors.join("; ") || "No Stooq quotes returned");
  }

  return quotes;
}

async function fetchStooqQuote(symbol: string): Promise<NuclearQuote> {
  const stooqSymbol = `${symbol.toLowerCase()}.us`;
  const url = `https://stooq.com/q/l/?s=${encodeURIComponent(
    stooqSymbol,
  )}&f=sd2t2ohlcv&h`;
  const res = await fetch(url, {
    next: { revalidate: REFRESH_CADENCE_SECONDS },
    headers: { Accept: "text/csv,*/*" },
  });
  if (!res.ok) throw new Error(`returned ${res.status}`);

  const csv = await res.text();
  const [, row] = csv.trim().split(/\r?\n/);
  if (!row) throw new Error("empty response");

  const [rawSymbol, date, time, openRaw, , , closeRaw] = row.split(",");
  if (!rawSymbol || date === "N/D" || closeRaw === "N/D") {
    throw new Error("no quote data");
  }

  const open = Number(openRaw);
  const close = Number(closeRaw);
  const changePercent =
    Number.isFinite(open) && open > 0 && Number.isFinite(close)
      ? ((close - open) / open) * 100
      : null;
  const marketTime = Date.parse(`${date}T${time || "00:00:00"}Z`);

  return {
    symbol,
    name: NAME_BY_SYMBOL[symbol] ?? symbol,
    price: Number.isFinite(close) ? close : null,
    changePercent,
    marketCapUsd: MARKET_CAP_USD_BY_SYMBOL[symbol] ?? null,
    marketCapSource:
      MARKET_CAP_USD_BY_SYMBOL[symbol] != null ? "snapshot" : "unavailable",
    currency: "USD",
    exchange: "Stooq",
    regularMarketTime: Number.isFinite(marketTime)
      ? Math.floor(marketTime / 1000)
      : null,
    sourceUrl: `https://stooq.com/q/?s=${encodeURIComponent(stooqSymbol)}`,
  };
}

async function fetchUraniumIndex(): Promise<NuclearUraniumIndex> {
  const startIso = new Date(Date.now() - 3 * 365 * 24 * 3600 * 1000)
    .toISOString()
    .slice(0, 10);
  const series = await fetchFredSeries("PURANUSDM", startIso);
  const points = series.map((point) => ({
    date: new Date(point.t * 1000).toISOString().slice(0, 10),
    value: point.v,
  }));
  const latest = points.at(-1) ?? null;
  const prior12m = points.length > 12 ? points[points.length - 13] : null;
  const change12mPercent =
    latest && prior12m && prior12m.value > 0
      ? ((latest.value - prior12m.value) / prior12m.value) * 100
      : null;

  return {
    label: "Uranium price index",
    unit: "USD/lb",
    source: "FRED · PURANUSDM",
    sourceUrl: "https://fred.stlouisfed.org/series/PURANUSDM",
    asOf: latest?.date ?? null,
    latest: latest?.value ?? null,
    change12mPercent,
    points,
    note:
      "Monthly global uranium price series from FRED. It is slower-moving than equity quotes and can lag the current month.",
  };
}

function finiteOrNull(value: unknown): number | null {
  return typeof value === "number" && Number.isFinite(value) ? value : null;
}

function sumMarketCap(quotes: NuclearQuote[]): number | null {
  const values = quotes
    .map((quote) => quote.marketCapUsd)
    .filter((value): value is number => value != null);
  if (values.length === 0) return null;
  return values.reduce((sum, value) => sum + value, 0);
}
