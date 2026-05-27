// Perpetual swap funding rates for BTC from Binance, Bybit, OKX.
// All three settle every 8h. 21 events = ~7 days.
import type { FundingExchangeData, FundingPoint } from "@/lib/btc/types";

const COMMON_HEADERS = { Accept: "application/json" } as const;
const FETCH_OPTS = {
  next: { revalidate: 600 }, // 10 min
  headers: COMMON_HEADERS,
} satisfies RequestInit & { next: { revalidate: number } };

type BinanceRow = { fundingTime: number; fundingRate: string };
type BybitResp = {
  result?: {
    list?: { fundingRate: string; fundingRateTimestamp: string }[];
  };
};
type OkxResp = {
  data?: { fundingRate: string; fundingTime: string }[];
};

async function fetchBinance(): Promise<FundingPoint[]> {
  const url =
    "https://fapi.binance.com/fapi/v1/fundingRate?symbol=BTCUSDT&limit=21";
  const res = await fetch(url, FETCH_OPTS);
  if (!res.ok) throw new Error(`Binance ${res.status}`);
  const rows = (await res.json()) as BinanceRow[];
  return rows
    .map((r) => ({
      t: Math.floor(r.fundingTime / 1000),
      rate: parseFloat(r.fundingRate),
    }))
    .filter((p) => Number.isFinite(p.rate))
    .sort((a, b) => a.t - b.t);
}

async function fetchBybit(): Promise<FundingPoint[]> {
  const url =
    "https://api.bybit.com/v5/market/funding/history?category=linear&symbol=BTCUSDT&limit=21";
  const res = await fetch(url, FETCH_OPTS);
  if (!res.ok) throw new Error(`Bybit ${res.status}`);
  const j = (await res.json()) as BybitResp;
  const list = j.result?.list ?? [];
  return list
    .map((r) => ({
      t: Math.floor(parseInt(r.fundingRateTimestamp, 10) / 1000),
      rate: parseFloat(r.fundingRate),
    }))
    .filter((p) => Number.isFinite(p.rate) && Number.isFinite(p.t))
    .sort((a, b) => a.t - b.t);
}

async function fetchOkx(): Promise<FundingPoint[]> {
  const url =
    "https://www.okx.com/api/v5/public/funding-rate-history?instId=BTC-USDT-SWAP&limit=21";
  const res = await fetch(url, FETCH_OPTS);
  if (!res.ok) throw new Error(`OKX ${res.status}`);
  const j = (await res.json()) as OkxResp;
  const list = j.data ?? [];
  return list
    .map((r) => ({
      t: Math.floor(parseInt(r.fundingTime, 10) / 1000),
      rate: parseFloat(r.fundingRate),
    }))
    .filter((p) => Number.isFinite(p.rate) && Number.isFinite(p.t))
    .sort((a, b) => a.t - b.t);
}

function summarise(
  exchange: FundingExchangeData["exchange"],
  history: FundingPoint[],
): FundingExchangeData {
  if (history.length === 0) {
    return {
      exchange,
      current: NaN,
      avg7d: NaN,
      history,
      error: "no rows",
    };
  }
  const current = history[history.length - 1].rate;
  const sum = history.reduce((a, b) => a + b.rate, 0);
  const avg7d = sum / history.length;
  return { exchange, current, avg7d, history };
}

export async function fetchAllFunding(): Promise<FundingExchangeData[]> {
  const tasks = [
    fetchBinance()
      .then((h) => summarise("binance", h))
      .catch((e) => ({
        exchange: "binance" as const,
        current: NaN,
        avg7d: NaN,
        history: [] as FundingPoint[],
        error: e instanceof Error ? e.message : String(e),
      })),
    fetchBybit()
      .then((h) => summarise("bybit", h))
      .catch((e) => ({
        exchange: "bybit" as const,
        current: NaN,
        avg7d: NaN,
        history: [] as FundingPoint[],
        error: e instanceof Error ? e.message : String(e),
      })),
    fetchOkx()
      .then((h) => summarise("okx", h))
      .catch((e) => ({
        exchange: "okx" as const,
        current: NaN,
        avg7d: NaN,
        history: [] as FundingPoint[],
        error: e instanceof Error ? e.message : String(e),
      })),
  ];
  return Promise.all(tasks);
}
