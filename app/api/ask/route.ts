import { NextRequest } from "next/server";
import { anthropic } from "@/lib/anthropic";
import { getDashboardData } from "@/lib/fetchers";
import { askSystemPrompt } from "@/lib/prompts";

export async function POST(req: NextRequest) {
  const { question } = await req.json();
  if (!question || typeof question !== "string") {
    return new Response("Missing question", { status: 400 });
  }

  const data = await getDashboardData();

  const stream = anthropic.messages.stream({
    model: process.env.ASK_MODEL ?? "claude-sonnet-4-6",
    max_tokens: 800,
    system: askSystemPrompt(data),
    messages: [{ role: "user", content: question }],
  });

  const encoder = new TextEncoder();
  const body = new ReadableStream<Uint8Array>({
    start(controller) {
      stream.on("text", (t) => controller.enqueue(encoder.encode(t)));
      stream.on("end", () => controller.close());
      stream.on("error", (e) => {
        console.error("ask stream error:", e);
        controller.error(e);
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
