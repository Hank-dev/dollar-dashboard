import { NextResponse } from "next/server";
import { fetchBtcDailyHistory } from "@/lib/btc/data/btcPrice";
import { fetchBtcDailyVolume } from "@/lib/btc/data/binanceKlines";
import {
  fitPowerLaw,
  powerLawBand,
  powerLawValue,
} from "@/lib/btc/calc/powerLaw";
import { LAST_HALVING_UNIX, SECONDS_PER_DAY, sma } from "@/lib/btc/calc/util";
import type { DailyPoint, PriceHistoryResponse } from "@/lib/btc/types";

export const revalidate = 3600;

export async function GET() {
  try {
    const [raw, rawVolume] = await Promise.all([
      fetchBtcDailyHistory(),
      fetchBtcDailyVolume().catch(() => [] as DailyPoint[]),
    ]);

    const series: DailyPoint[] = raw.map((p) => ({ t: p.t, v: p.v }));
    const fit = fitPowerLaw(series);

    const fitSeries: DailyPoint[] = [];
    const upper1: DailyPoint[] = [];
    const lower1: DailyPoint[] = [];
    const upper2: DailyPoint[] = [];
    const lower2: DailyPoint[] = [];
    for (const p of series) {
      fitSeries.push({ t: p.t, v: powerLawValue(fit, p.t) });
      upper1.push({ t: p.t, v: powerLawBand(fit, p.t, 1) });
      lower1.push({ t: p.t, v: powerLawBand(fit, p.t, -1) });
      upper2.push({ t: p.t, v: powerLawBand(fit, p.t, 2) });
      lower2.push({ t: p.t, v: powerLawBand(fit, p.t, -2) });
    }

    const prices = series.map((p) => p.v);
    const smaArr = sma(prices, 200);
    const sma200: DailyPoint[] = series.map((p, i) => ({
      t: p.t,
      v: smaArr[i],
    }));

    let athIdx = 0;
    for (let i = 1; i < series.length; i++) {
      if (series[i].v > series[athIdx].v) athIdx = i;
    }
    const ath = { t: series[athIdx].t, v: series[athIdx].v };
    const nowSec = Math.floor(Date.now() / 1000);
    const daysSinceAth = Math.max(
      0,
      Math.floor((nowSec - ath.t) / SECONDS_PER_DAY),
    );
    const daysSinceLastHalving = Math.max(
      0,
      Math.floor((nowSec - LAST_HALVING_UNIX) / SECONDS_PER_DAY),
    );

    const body: PriceHistoryResponse = {
      series,
      fit: fitSeries,
      upper1,
      lower1,
      upper2,
      lower2,
      sma200,
      volume: rawVolume.map((p) => ({ t: p.t, v: p.v })),
      ath,
      daysSinceAth,
      daysSinceLastHalving,
      powerLaw: { a: fit.a, b: fit.b, sigma: fit.sigma },
      generatedAt: nowSec,
    };
    return NextResponse.json(body);
  } catch (err) {
    return NextResponse.json(
      { error: err instanceof Error ? err.message : String(err) },
      { status: 502 },
    );
  }
}
