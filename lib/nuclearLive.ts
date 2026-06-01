import { fetchFredSeries } from "./btc/data/fred";
import { fetchEquityQuotes, type EquityQuote } from "./quotes";

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

  const { quotes: equityQuotes, source: quoteSource, errors: quoteErrors } =
    await fetchEquityQuotes(symbols, {
      snapshotMarketCapUsd: MARKET_CAP_USD_BY_SYMBOL,
      nameBySymbol: NAME_BY_SYMBOL,
      revalidateSeconds: REFRESH_CADENCE_SECONDS,
    });
  errors.push(...quoteErrors);

  let source = "Yahoo Finance quote API";
  let note =
    "Public equities use live/delayed exchange quotes. Market caps fall back to the dashboard snapshot if the quote source does not provide them. Uranium proxies are equity ETFs.";
  if (quoteSource === "stooq") {
    source = "Stooq quote CSV fallback";
    note =
      "Public equities use delayed Stooq quotes. Market caps use the dashboard snapshot because Stooq does not provide them; percentage moves are calculated from the daily open. Uranium proxies are equity ETFs.";
  }

  const quotes: NuclearQuote[] = equityQuotes.map(toNuclearQuote);

  let uraniumIndex: NuclearUraniumIndex | null = null;
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

function toNuclearQuote(q: EquityQuote): NuclearQuote {
  return {
    symbol: q.symbol,
    name: q.name ?? q.symbol,
    price: q.price,
    changePercent: q.changePercent,
    marketCapUsd: q.marketCapUsd,
    marketCapSource: q.marketCapSource,
    currency: q.currency,
    exchange: q.exchange,
    regularMarketTime: q.asOfUnix,
    sourceUrl: q.sourceUrl,
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

function sumMarketCap(quotes: NuclearQuote[]): number | null {
  const values = quotes
    .map((quote) => quote.marketCapUsd)
    .filter((value): value is number => value != null);
  if (values.length === 0) return null;
  return values.reduce((sum, value) => sum + value, 0);
}
