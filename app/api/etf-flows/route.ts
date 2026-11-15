import { NextResponse } from "next/server";
import { cachedRoute } from "@/lib/routeCache";
import { fetchFarsideFlows, summariseFlows } from "@/lib/btc/data/farside";
import type { EtfFlowsResponse } from "@/lib/btc/types";

export const dynamic = "force-dynamic";

export const GET = cachedRoute("etf-flows", 21600, async () => {
  try {
    const days = await fetchFarsideFlows();
    if (days.length === 0) throw new Error("no ETF flow rows");
    const s = summariseFlows(days);
    const body: EtfFlowsResponse = {
      days,
      latestDate: s.latestDate,
      cumulativeTotal: s.cumulativeTotal,
      latestTotal: s.latestTotal,
      flow7dAvg: s.flow7dAvg,
      flow30dAvg: s.flow30dAvg,
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
