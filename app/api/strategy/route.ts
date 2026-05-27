import { NextResponse } from "next/server";
import { fetchMstrAtmFilings } from "@/lib/btc/data/edgar";
import { fetchYahooDaily } from "@/lib/btc/data/yahoo";
import { fetchBtcDailyHistory } from "@/lib/btc/data/btcPrice";
import { SECONDS_PER_DAY } from "@/lib/btc/calc/util";
import type {
  StrategyFiling,
  StrategyFlowsResponse,
  StrcWeekPoint,
} from "@/lib/btc/types";

export const revalidate = 86400; // 24h — EDGAR filings are weekly

function avgBtcPriceInWindow(
  prices: { t: number; v: number }[],
  endSec: number,
  windowDays = 7,
): number | null {
  if (prices.length === 0) return null;
  const startSec = endSec - windowDays * SECONDS_PER_DAY;
  let sum = 0;
  let n = 0;
  for (const p of prices) {
    if (p.t > startSec && p.t <= endSec) {
      sum += p.v;
      n++;
    }
  }
  return n > 0 ? sum / n : null;
}

function buildWeekly(
  filings: StrategyFiling[],
  prices: { t: number; v: number }[],
): StrcWeekPoint[] {
  const sorted = [...filings].sort((a, b) => a.filedAt - b.filedAt);
  const out: StrcWeekPoint[] = [];
  for (const f of sorted) {
    const strc = f.rows.find((r) => r.ticker === "STRC");
    const refDate = f.periodEnd ?? f.filedAt;
    const avg = avgBtcPriceInWindow(prices, refDate, 7);
    const strcProceedsM = strc?.netProceedsM ?? 0;
    const totalProceedsM = f.rows.reduce((s, r) => s + r.netProceedsM, 0);
    const impliedBtc =
      strcProceedsM > 0 && avg && avg > 0
        ? (strcProceedsM * 1_000_000) / avg
        : null;
    out.push({
      t: f.filedAt,
      netProceedsM: strcProceedsM,
      totalProceedsM,
      impliedBtc,
    });
  }
  return out.slice(-12);
}

function ytdSum(filings: StrategyFiling[]): number {
  const yearStart =
    Math.floor(Date.UTC(new Date().getUTCFullYear(), 0, 1) / 1000);
  let sum = 0;
  for (const f of filings) {
    if (f.filedAt < yearStart) continue;
    const strc = f.rows.find((r) => r.ticker === "STRC");
    if (strc) sum += strc.netProceedsM;
  }
  return sum;
}

export async function GET() {
  try {
    const [filings, strcQuote, btcPrices] = await Promise.all([
      fetchMstrAtmFilings(16).catch(() => [] as StrategyFiling[]),
      fetchYahooDaily("STRC", "1mo").catch(() => []),
      fetchBtcDailyHistory().catch(() => [] as { t: number; v: number }[]),
    ]);

    const sortedFilings = filings.sort((a, b) => b.filedAt - a.filedAt);
    const latest = sortedFilings[0] ?? null;
    const latestStrc =
      latest?.rows.find((r) => r.ticker === "STRC") ?? null;

    const strcPrice =
      strcQuote.length > 0 ? strcQuote[strcQuote.length - 1].v : null;
    const strcPrev =
      strcQuote.length > 1 ? strcQuote[strcQuote.length - 2].v : null;
    const strcChange24h =
      strcPrice != null && strcPrev != null && strcPrev > 0
        ? ((strcPrice - strcPrev) / strcPrev) * 100
        : null;

    const strcLastWeekM = latestStrc?.netProceedsM ?? null;
    const refDate = latest ? (latest.periodEnd ?? latest.filedAt) : null;
    const avgBtc = refDate ? avgBtcPriceInWindow(btcPrices, refDate, 7) : null;
    const strcLastWeekImpliedBtc =
      strcLastWeekM != null && avgBtc && avgBtc > 0
        ? (strcLastWeekM * 1_000_000) / avgBtc
        : null;

    const strcRemainingCapacityM = latestStrc?.availableM ?? null;
    const totalAtmCapacityB = latest
      ? latest.rows.reduce((s, r) => s + r.availableM, 0) / 1000
      : null;

    const body: StrategyFlowsResponse = {
      strcPrice,
      strcChange24h,
      latestFiledAt: latest?.filedAt ?? null,
      strcLastWeekM,
      strcLastWeekImpliedBtc,
      strcRemainingCapacityM,
      totalAtmCapacityB,
      strcYtdM: ytdSum(sortedFilings),
      weekly: buildWeekly(sortedFilings, btcPrices),
      generatedAt: Math.floor(Date.now() / 1000),
    };
    return NextResponse.json(body);
  } catch (err) {
    return NextResponse.json(
      { error: err instanceof Error ? err.message : String(err) },
      { status: 502 },
    );
  }
}
