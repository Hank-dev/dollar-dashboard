import { NextRequest, NextResponse } from "next/server";
import { anthropic } from "@/lib/anthropic";
import { METRICS } from "@/lib/metrics";
import { explainSystemPrompt } from "@/lib/prompts";

export async function POST(req: NextRequest) {
  try {
    const { metricId } = await req.json();
    const metric = METRICS.find((m) => m.id === metricId);
    if (!metric) {
      return NextResponse.json({ error: "Unknown metric" }, { status: 400 });
    }

    const msg = await anthropic.messages.create({
      model: process.env.EXPLAIN_MODEL ?? "claude-sonnet-4-6",
      max_tokens: 400,
      system: explainSystemPrompt(),
      messages: [
        {
          role: "user",
          content: `Explain the metric "${metric.label}", currently ${metric.value} (${metric.context}).`,
        },
      ],
    });

    const explanation = msg.content
      .filter((b) => b.type === "text")
      .map((b) => (b as { text: string }).text)
      .join("\n")
      .trim();

    return NextResponse.json({ explanation });
  } catch (err) {
    console.error("explain error:", err);
    return NextResponse.json(
      { error: "Could not generate explanation." },
      { status: 500 },
    );
  }
}
