import { NextResponse } from "next/server";
import { fetchBtcDailyHistory } from "@/lib/btc/data/btcPrice";
import {
  fetchMvrvZ,
  fetchNupl,
  fetchRealizedPrice,
} from "@/lib/btc/data/bitcoinData";
import { mayerMultiple } from "@/lib/btc/calc/indicators";
import { fitPowerLaw, powerLawZ } from "@/lib/btc/calc/powerLaw";
import { sma } from "@/lib/btc/calc/util";
import type { RegimeResponse } from "@/lib/btc/types";

export const revalidate = 1800;

const tail = <T extends { v: number }>(arr: T[]): number | null =>
  arr.length > 0 ? arr[arr.length - 1].v : null;

export async function GET() {
  try {
    const [prices, mvrvZ, nupl, realized] = await Promise.all([
      fetchBtcDailyHistory(),
      fetchMvrvZ(),
      fetchNupl(),
      fetchRealizedPrice(),
    ]);

    if (prices.length === 0) throw new Error("no price data");
    const last = prices[prices.length - 1];

    const fit = fitPowerLaw(prices);
    const pwZ = powerLawZ(fit, last.t, last.v);

    const priceArr = prices.map((p) => p.v);
    const smaArr = sma(priceArr, 200);
    const sma200 = smaArr[smaArr.length - 1];
    const mayer = mayerMultiple(last.v, sma200);

    const body: RegimeResponse = {
      spot: last.v,
      mvrvZ: tail(mvrvZ),
      nupl: tail(nupl),
      realizedPrice: tail(realized),
      mayer,
      powerLawZ: Number.isFinite(pwZ) ? pwZ : null,
      asOf: Math.floor(Date.now() / 1000),
    };
    return NextResponse.json(body);
  } catch (err) {
    return NextResponse.json(
      { error: err instanceof Error ? err.message : String(err) },
      { status: 502 },
    );
  }
}
