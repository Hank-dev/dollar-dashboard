import { NextResponse } from "next/server";
import { fetchBtcDailyHistory } from "@/lib/btc/data/btcPrice";
import { fetchFredSeries } from "@/lib/btc/data/fred";
import { fetchFng } from "@/lib/btc/data/alternativeme";

export const revalidate = 3600;

export async function GET() {
  const start90d = new Date(Date.now() - 90 * 24 * 3600 * 1000)
    .toISOString()
    .slice(0, 10);

  const [btcResult, dxyResult, fngResult] = await Promise.allSettled([
    fetchBtcDailyHistory().then((series) => series.slice(-90).map((p) => p.v)),
    fetchFredSeries("DTWEXBGS", start90d).then((series) => series.map((p) => p.v)),
    fetchFng(90).then((data) => data.map((p) => p.v)),
  ]);

  return NextResponse.json({
    btc: btcResult.status === "fulfilled" ? btcResult.value : [],
    dxy: dxyResult.status === "fulfilled" ? dxyResult.value : [],
    fng: fngResult.status === "fulfilled" ? fngResult.value : [],
  });
}
