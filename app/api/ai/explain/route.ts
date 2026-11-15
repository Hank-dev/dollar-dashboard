import { readJson } from "@/lib/llmGuard";
import { explainItem } from "@/lib/llmRoutes";
import { getAiDashboardData, type AiExplainable } from "@/lib/aiMetrics";
import { aiExplainSystemPrompt } from "@/lib/aiPrompts";

type ItemType = AiExplainable["type"];

export async function POST(req: Request) {
  const body = await readJson(req);
  const itemId = typeof body?.itemId === "string" ? body.itemId : undefined;
  const itemType = typeof body?.itemType === "string" ? (body.itemType as ItemType) : undefined;
  const data = getAiDashboardData();
  const selection = findSelection(data, itemType, itemId);
  if (!selection) {
    return Response.json({ error: "Unknown AI dashboard item" }, { status: 400 });
  }
  return explainItem(
    req,
    "ai",
    `${selection.type}:${itemId}`,
    420,
    aiExplainSystemPrompt(data),
    `Explain this ${selection.type}: ${JSON.stringify(selection.item)}`,
  );
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
