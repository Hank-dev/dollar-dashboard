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
  const errors: string[] = [];
  let prices: { t: number; v: number }[] = [];
  let mvrvZ: number | null = null;
  let nupl: number | null = null;
  let realizedPrice: number | null = null;

  try {
    prices = await fetchBtcDailyHistory();
  } catch (err) {
    errors.push(`prices: ${err instanceof Error ? err.message : String(err)}`);
  }

  if (prices.length === 0) {
    return NextResponse.json(
      { error: "no BTC price data available", errors },
      { status: 502 },
    );
  }

  // On-chain metrics are nice-to-have — fail individually
  const [mvrvZResult, nuplResult, realizedResult] =
    await Promise.allSettled([
      fetchMvrvZ(),
      fetchNupl(),
      fetchRealizedPrice(),
    ]);

  if (mvrvZResult.status === "fulfilled") {
    mvrvZ = tail(mvrvZResult.value);
  } else {
    errors.push(`mvrv-zscore: ${mvrvZResult.reason?.message ?? String(mvrvZResult.reason)}`);
  }
  if (nuplResult.status === "fulfilled") {
    nupl = tail(nuplResult.value);
  } else {
    errors.push(`nupl: ${nuplResult.reason?.message ?? String(nuplResult.reason)}`);
  }
  if (realizedResult.status === "fulfilled") {
    realizedPrice = tail(realizedResult.value);
  } else {
    errors.push(`realized-price: ${realizedResult.reason?.message ?? String(realizedResult.reason)}`);
  }

  const last = prices[prices.length - 1];
  const fit = fitPowerLaw(prices);
  const pwZ = powerLawZ(fit, last.t, last.v);

  const priceArr = prices.map((p) => p.v);
  const smaArr = sma(priceArr, 200);
  const sma200 = smaArr[smaArr.length - 1];
  const mayer = mayerMultiple(last.v, sma200);

  const body: RegimeResponse = {
    spot: last.v,
    mvrvZ,
    nupl,
    realizedPrice,
    mayer,
    powerLawZ: Number.isFinite(pwZ) ? pwZ : null,
    asOf: Math.floor(Date.now() / 1000),
  };
  return NextResponse.json(body);
}
