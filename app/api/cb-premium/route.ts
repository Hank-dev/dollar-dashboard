import { NextResponse } from "next/server";
import { cachedRoute } from "@/lib/routeCache";
import { fetchCoinbasePremium } from "@/lib/btc/data/cbPremium";
import type { CbPremiumResponse } from "@/lib/btc/types";

export const dynamic = "force-dynamic";

export const GET = cachedRoute("cb-premium", 600, async () => {
  try {
    const r = await fetchCoinbasePremium(48);
    const body: CbPremiumResponse = {
      series: r.series,
      current: r.current,
      avg24hBps: r.avg24hBps,
      generatedAt: Math.floor(Date.now() / 1000),
    };
    return NextResponse.json(body);
  } catch (err) {
    return NextResponse.json(
      { error: err instanceof Error ? err.message : String(err) },
      { status: 502 },
    );
  }
});
