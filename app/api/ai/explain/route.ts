import { NextRequest, NextResponse } from "next/server";
import { anthropic } from "@/lib/anthropic";
import { getAiDashboardData, type AiExplainable } from "@/lib/aiMetrics";
import { aiExplainSystemPrompt } from "@/lib/aiPrompts";

type ItemType = AiExplainable["type"];

export async function POST(req: NextRequest) {
  try {
    const { itemId, itemType } = (await req.json()) as {
      itemId?: string;
      itemType?: ItemType;
    };
    const data = getAiDashboardData();
    const selection = findSelection(data, itemType, itemId);

    if (!selection) {
      return NextResponse.json({ error: "Unknown AI dashboard item" }, { status: 400 });
    }

    const msg = await anthropic.messages.create({
      model: process.env.EXPLAIN_MODEL ?? "claude-sonnet-4-6",
      max_tokens: 420,
      system: aiExplainSystemPrompt(data),
      messages: [
        {
          role: "user",
          content: `Explain this ${selection.type}: ${JSON.stringify(selection.item)}`,
        },
      ],
    });

    const explanation = msg.content
      .filter((block) => block.type === "text")
      .map((block) => (block as { text: string }).text)
      .join("\n")
      .trim();

    return NextResponse.json({ explanation });
  } catch (err) {
    console.error("ai explain error:", err);
    return NextResponse.json(
      { error: "Could not generate explanation." },
      { status: 500 },
    );
  }
}

function findSelection(
  data: ReturnType<typeof getAiDashboardData>,
  itemType: ItemType | undefined,
  itemId: string | undefined,
): AiExplainable | null {
  if (!itemType || !itemId) return null;
  if (itemType === "player") {
    const item = data.players.find((player) => player.id === itemId);
    return item ? { type: "player", item } : null;
  }
  if (itemType === "metric") {
    const item = data.marketMetrics.find((metric) => metric.id === itemId);
    return item ? { type: "metric", item } : null;
  }
  const item = data.techSignals.find((signal) => signal.id === itemId);
  return item ? { type: "signal", item } : null;
}
