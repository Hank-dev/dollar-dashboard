import { NextResponse } from "next/server";
import { anthropic } from "@/lib/anthropic";
import { getDashboardData } from "@/lib/fetchers";

const MS_PER_DAY = 24 * 60 * 60 * 1000;

let cachedVerdict: string | null = null;
let cachedAt: number | null = null;

export const dynamic = "force-dynamic";

export async function GET() {
  const now = Date.now();

  // Return cached verdict if still fresh (< 24h)
  if (cachedVerdict && cachedAt && now - cachedAt < MS_PER_DAY) {
    return NextResponse.json({
      verdict: cachedVerdict,
      generatedAt: new Date(cachedAt).toISOString(),
      cached: true,
    });
  }

  const data = await getDashboardData();

  const metricsSummary = data.metrics
    .map((m) => `${m.label}: ${m.value} (${m.status})`)
    .join("\n");

  const msg = await anthropic.messages.create({
    model: "claude-sonnet-4-6",
    max_tokens: 300,
    messages: [
      {
        role: "user",
        content: `You are a macro analyst writing a daily verdict. Based on the following US dollar & global financial system metrics, write a single-paragraph verdict (2-4 sentences) that captures the most important tension or signal. Be direct, use specific numbers. No greetings, no disclaimers, no markdown.

Metrics:
${metricsSummary}`,
      },
    ],
  });

  const verdict = msg.content
    .filter((b) => b.type === "text")
    .map((b) => b.text)
    .join("");

  cachedVerdict = verdict;
  cachedAt = now;

  return NextResponse.json({
    verdict,
    generatedAt: new Date(now).toISOString(),
    cached: false,
  });
}
