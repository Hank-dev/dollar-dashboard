import { NextRequest } from "next/server";
import { anthropic } from "@/lib/anthropic";
import { fetchNuclearMarketLive } from "@/lib/nuclearLive";
import { getNuclearDashboardData } from "@/lib/nuclearMetrics";
import { nuclearAskSystemPrompt } from "@/lib/nuclearPrompts";

export async function POST(req: NextRequest) {
  const { question } = await req.json();
  if (!question || typeof question !== "string") {
    return new Response("Missing question", { status: 400 });
  }

  const data = getNuclearDashboardData();
  const live = await fetchNuclearMarketLive().catch(() => null);
  const stream = anthropic.messages.stream({
    model: process.env.ASK_MODEL ?? "claude-sonnet-4-6",
    max_tokens: 850,
    system: nuclearAskSystemPrompt(data, live),
    messages: [{ role: "user", content: question }],
  });

  const encoder = new TextEncoder();
  const body = new ReadableStream<Uint8Array>({
    start(controller) {
      stream.on("text", (text) => controller.enqueue(encoder.encode(text)));
      stream.on("end", () => controller.close());
      stream.on("error", (err) => {
        console.error("nuclear ask stream error:", err);
        controller.error(err);
      });
    },
  });

  return new Response(body, {
    headers: {
      "Content-Type": "text/plain; charset=utf-8",
      "Cache-Control": "no-store",
    },
  });
}
