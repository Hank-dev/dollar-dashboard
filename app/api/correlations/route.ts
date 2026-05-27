import { NextResponse } from "next/server";
import { fetchBtcDailyHistory } from "@/lib/btc/data/btcPrice";
import { fetchFredSeries } from "@/lib/btc/data/fred";
import { fetchYahooDaily } from "@/lib/btc/data/yahoo";
import { fetchCoingeckoDaily } from "@/lib/btc/data/coingecko";
import {
  alignByDay,
  logReturns,
  rollingCorrelation,
} from "@/lib/btc/calc/correlation";
import { SECONDS_PER_DAY } from "@/lib/btc/calc/util";
import type {
  CorrelationAsset,
  CorrelationKey,
  CorrelationsResponse,
} from "@/lib/btc/types";

export const revalidate = 86_400; // daily

type Point = { t: number; v: number };

type AssetSpec = {
  key: CorrelationKey;
  label: string;
  source: string;
  fetch: () => Promise<Point[]>;
};

function buildSpecs(startIso: string): AssetSpec[] {
  return [
    {
      key: "sp500",
      label: "S&P 500",
      source: "FRED · SP500",
      fetch: () => fetchFredSeries("SP500", startIso),
    },
    {
      key: "ndx",
      label: "NASDAQ 100",
      source: "FRED · NASDAQ100",
      fetch: () => fetchFredSeries("NASDAQ100", startIso),
    },
    {
      key: "gold",
      label: "Gold (PAXG)",
      source: "CoinGecko · pax-gold",
      // PAX Gold tracks spot gold ~1:1; log returns match for correlation
      // purposes. Falls back to Yahoo GLD if CoinGecko fails.
      fetch: async () => {
        try {
          return await fetchCoingeckoDaily("pax-gold", 365);
        } catch (e) {
          const rows = await fetchYahooDaily("GLD", "1y");
          if (rows.length === 0) throw e;
          return rows.map((r) => ({ t: r.t, v: r.v }));
        }
      },
    },
    {
      key: "dxy",
      label: "USD Index",
      source: "FRED · DTWEXBGS",
      fetch: () => fetchFredSeries("DTWEXBGS", startIso),
    },
    {
      key: "vix",
      label: "VIX",
      source: "FRED · VIXCLS",
      fetch: () => fetchFredSeries("VIXCLS", startIso),
    },
    {
      key: "dfii10",
      label: "10Y Real Yield",
      source: "FRED · DFII10",
      fetch: () => fetchFredSeries("DFII10", startIso),
    },
  ];
}

function summariseAsset(
  spec: AssetSpec,
  series: Point[],
  btc: Point[],
): CorrelationAsset {
  if (series.length === 0) {
    return {
      key: spec.key,
      label: spec.label,
      source: spec.source,
      r30: null,
      r90: null,
      error: "no data",
    };
  }
  const aligned = alignByDay(btc, series);
  if (aligned.a.length < 91) {
    return {
      key: spec.key,
      label: spec.label,
      source: spec.source,
      r30: null,
      r90: null,
      error: "insufficient overlap",
    };
  }
  const btcRet = logReturns(aligned.a);
  const assetRet = logReturns(aligned.b);
  const r30 = rollingCorrelation(btcRet, assetRet, 30);
  const r90 = rollingCorrelation(btcRet, assetRet, 90);
  const last30 = r30[r30.length - 1];
  const last90 = r90[r90.length - 1];
  return {
    key: spec.key,
    label: spec.label,
    source: spec.source,
    r30: Number.isFinite(last30) ? last30 : null,
    r90: Number.isFinite(last90) ? last90 : null,
  };
}

export async function GET() {
  const startIso = new Date(Date.now() - 365 * 86_400_000)
    .toISOString()
    .slice(0, 10);

  const specs = buildSpecs(startIso);
  try {
    const btcAll = await fetchBtcDailyHistory();
    const startSec = Math.floor(new Date(startIso).getTime() / 1000);
    const btc = btcAll.filter((p) => p.t >= startSec - 30 * SECONDS_PER_DAY);

    const results = await Promise.all(
      specs.map((s) =>
        s
          .fetch()
          .then((rows) => ({ key: s.key, rows, error: null as string | null }))
          .catch((e) => ({
            key: s.key,
            rows: [] as Point[],
            error: e instanceof Error ? e.message : String(e),
          })),
      ),
    );

    const assets: CorrelationAsset[] = specs.map((s) => {
      const r = results.find((x) => x.key === s.key);
      const summary = summariseAsset(s, r?.rows ?? [], btc);
      if (r?.error && summary.r30 == null && summary.r90 == null) {
        return { ...summary, error: r.error };
      }
      return summary;
    });

    const body: CorrelationsResponse = {
      assets,
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
