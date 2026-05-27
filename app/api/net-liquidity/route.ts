import { NextResponse } from "next/server";
import { fetchFredSeries } from "@/lib/btc/data/fred";
import { computeNetLiquidity } from "@/lib/btc/calc/netLiquidity";
import { SECONDS_PER_DAY } from "@/lib/btc/calc/util";
import type { NetLiquidityResponse } from "@/lib/btc/types";

export const revalidate = 21600;

export async function GET() {
  try {
    const [walcl, tga, rrp] = await Promise.all([
      fetchFredSeries("WALCL", "2015-01-01"),
      fetchFredSeries("WTREGEN", "2015-01-01"),
      fetchFredSeries("RRPONTSYD", "2015-01-01"),
    ]);

    const series = computeNetLiquidity(walcl, tga, rrp);
    if (series.length === 0) throw new Error("no net liquidity data");

    const latest = series[series.length - 1];
    let delta30d = 0;
    const back30 = latest.t - 30 * SECONDS_PER_DAY;
    for (let i = series.length - 1; i >= 0; i--) {
      if (series[i].t <= back30) {
        delta30d = latest.v - series[i].v;
        break;
      }
    }

    const body: NetLiquidityResponse = {
      series,
      latest: latest.v,
      delta30d,
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
