import { NextResponse } from "next/server";
import { fetchAllFunding } from "@/lib/btc/data/funding";
import type { FundingResponse } from "@/lib/btc/types";

export const revalidate = 600;

export async function GET() {
  try {
    const exchanges = await fetchAllFunding();

    const valid = exchanges.filter((e) => e.history.length > 0);
    const combinedCurrent =
      valid.length > 0
        ? valid.reduce((s, e) => s + e.current, 0) / valid.length
        : null;
    const combinedAvg7d =
      valid.length > 0
        ? valid.reduce((s, e) => s + e.avg7d, 0) / valid.length
        : null;
    const combinedAnnualized7d =
      combinedAvg7d != null ? combinedAvg7d * 3 * 365 : null;

    const body: FundingResponse = {
      exchanges,
      combinedCurrent,
      combinedAvg7d,
      combinedAnnualized7d,
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
