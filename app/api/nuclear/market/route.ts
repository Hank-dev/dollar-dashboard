import { NextResponse } from "next/server";
import { cachedRoute } from "@/lib/routeCache";
import { fetchNuclearMarketLive } from "@/lib/nuclearLive";

export const dynamic = "force-dynamic";

export const GET = cachedRoute("nuclear/market", 3600, async () => {
  try {
    const data = await fetchNuclearMarketLive();
    return NextResponse.json(data, {
      headers: {
        "Cache-Control": "public, max-age=120, s-maxage=3600, stale-while-revalidate=900",
      },
    });
  } catch (err) {
    return NextResponse.json(
      { error: err instanceof Error ? err.message : String(err) },
      { status: 502 },
    );
  }
});
