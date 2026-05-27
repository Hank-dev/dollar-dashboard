import { NextResponse } from "next/server";
import { fetchNuclearMarketLive } from "@/lib/nuclearLive";

export const revalidate = 3600;

export async function GET() {
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
}
