import { NextResponse } from "next/server";
import { anthropic, describeAnthropicError, messageText, MODELS } from "@/lib/anthropic";
import { getDashboardData } from "@/lib/fetchers";
import { hasAnthropicKey } from "@/lib/llmGuard";
import { cached } from "@/lib/routeCache";

export const dynamic = "force-dynamic";

// One verdict per day; concurrent requests share the call.
export async function GET() {
  if (!hasAnthropicKey()) {
    return NextResponse.json(
      { error: "AI verdict is not configured on this server (ANTHROPIC_API_KEY)." },
      { status: 503 },
    );
  }
  try {
    const hit = await cached("verdict", { ttlSeconds: 86_400 }, async () => {
      const data = await getDashboardData();
      const metricsSummary = data.metrics
        .map((m) => `${m.label}: ${m.value} (${m.status}; ${m.asOf})`)
        .join("\n");
      const msg = await anthropic.messages.create({
        model: MODELS.briefing,
        max_tokens: 300,
        messages: [
          {
            role: "user",
            content: `You are a macro analyst writing a daily verdict. Based on the following US dollar & global financial system metrics, write a single-paragraph verdict (2-4 sentences) that captures the most important tension or signal. Be direct, use specific numbers. Each metric carries its own as-of date; do not present an older reading as today's. No greetings, no disclaimers, no markdown.

Metrics:
${metricsSummary}`,
          },
        ],
      });
      const verdict = messageText(msg, "");
      if (!verdict) throw new Error("empty verdict");
      return verdict;
    });
    return NextResponse.json({
      verdict: hit.value,
      generatedAt: new Date(hit.storedAt).toISOString(),
      cached: hit.status !== "fresh",
    });
  } catch (err) {
    console.error("verdict error:", err);
    const { status, message } = describeAnthropicError(err);
    return NextResponse.json({ error: message }, { status });
  }
}
