import { NextResponse } from "next/server";
import { cachedRoute } from "@/lib/routeCache";
import { fetchBtcDailyHistory } from "@/lib/btc/data/btcPrice";
import { fetchDxyDaily } from "@/lib/btc/data/cnbc";
import { fetchFng } from "@/lib/btc/data/alternativeme";

export const dynamic = "force-dynamic";

export const GET = cachedRoute("home-sparklines", 3600, async () => {
  const [btcResult, dxyResult, fngResult] = await Promise.allSettled([
    fetchBtcDailyHistory().then((series) => series.slice(-90).map((p) => p.v)),
    fetchDxyDaily().then((series) => series.slice(-90).map((p) => p.v)),
    fetchFng(90).then((data) => data.map((p) => p.v)),
  ]);

  return NextResponse.json({
    btc: btcResult.status === "fulfilled" ? btcResult.value : [],
    dxy: dxyResult.status === "fulfilled" ? dxyResult.value : [],
    fng: fngResult.status === "fulfilled" ? fngResult.value : [],
  });
}, { degraded: (b) => Object.values(b as Record<string, unknown[]>).some((s) => s.length === 0) });
