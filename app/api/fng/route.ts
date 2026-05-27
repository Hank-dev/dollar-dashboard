import { NextResponse } from "next/server";
import { fetchFng } from "@/lib/btc/data/alternativeme";
import type { FngResponse } from "@/lib/btc/types";

export const revalidate = 3600;

export async function GET() {
  try {
    const history = await fetchFng(365);
    if (history.length === 0) throw new Error("no F&G data");
    const current = history[history.length - 1];
    const body: FngResponse = {
      current,
      history,
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
