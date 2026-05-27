import { NextResponse } from "next/server";
import { fetchBtcDailyHistory } from "@/lib/btc/data/btcPrice";
import { fetchBtcDailyVolume } from "@/lib/btc/data/binanceKlines";
import {
  DEFAULT_MEI_CONFIG,
  REGIMES,
  computeMEI,
  regimeDistribution,
  type MeiBar,
  type MeiInputRow,
  type RegimeState,
} from "@/lib/btc/calc/marketEmotion";
import type { DailyPoint, MeiPoint, MeiResponse } from "@/lib/btc/types";

export const revalidate = 3600;

function joinPriceVolume(
  closes: DailyPoint[],
  volume: DailyPoint[],
): MeiInputRow[] {
  // Closes are the spine (full history); volume is back-aligned by timestamp.
  const volByT = new Map<number, number>();
  for (const v of volume) volByT.set(v.t, v.v);
  return closes.map((c) => ({
    t: c.t,
    close: c.v,
    volume: volByT.get(c.t) ?? null,
  }));
}

export async function GET() {
  try {
    const [closes, volume] = await Promise.all([
      fetchBtcDailyHistory(),
      fetchBtcDailyVolume().catch(() => [] as DailyPoint[]),
    ]);

    const rows = joinPriceVolume(closes, volume);
    const bars = computeMEI(rows, DEFAULT_MEI_CONFIG);

    const series: MeiPoint[] = bars.map((b) => ({
      t: b.t,
      close: b.close,
      v: b.valence,
      a: b.arousal,
      s: b.state,
      i: b.intensity,
      w: b.isWarmup,
    }));

    const last = bars[bars.length - 1];
    const postWarmup = bars.filter((b: MeiBar) => !b.isWarmup);
    const dist = regimeDistribution(postWarmup);
    const total = postWarmup.length || 1;
    const distribution = REGIMES.map((r) => ({
      state: r as RegimeState,
      count: dist[r],
      pct: dist[r] / total,
    }));

    const volWithData = bars.filter((b: MeiBar, idx: number) => rows[idx].volume != null).length;

    const body: MeiResponse = {
      series,
      current: {
        t: last.t,
        state: last.state,
        valence: last.valence,
        arousal: last.arousal,
        intensity: last.intensity,
      },
      distribution,
      config: {
        normWindow: DEFAULT_MEI_CONFIG.normWindow,
        trendWindow: DEFAULT_MEI_CONFIG.trendWindow,
        momWindow: DEFAULT_MEI_CONFIG.momWindow,
        volWindow: DEFAULT_MEI_CONFIG.volWindow,
        smoothSpan: DEFAULT_MEI_CONFIG.smoothSpan,
        bands: {
          valenceBear: DEFAULT_MEI_CONFIG.valenceBear,
          valenceBull: DEFAULT_MEI_CONFIG.valenceBull,
          arousalLow: DEFAULT_MEI_CONFIG.arousalLow,
          arousalHigh: DEFAULT_MEI_CONFIG.arousalHigh,
        },
      },
      generatedAt: Math.floor(Date.now() / 1000),
      volumeCoverage: bars.length > 0 ? volWithData / bars.length : 0,
    };
    return NextResponse.json(body);
  } catch (err) {
    return NextResponse.json(
      { error: err instanceof Error ? err.message : String(err) },
      { status: 502 },
    );
  }
}
