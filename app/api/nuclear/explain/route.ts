import { readJson } from "@/lib/llmGuard";
import { explainItem } from "@/lib/llmRoutes";
import {
  getNuclearDashboardData,
  type NuclearExplainable,
} from "@/lib/nuclearMetrics";
import { nuclearExplainSystemPrompt } from "@/lib/nuclearPrompts";

type ItemType = NuclearExplainable["type"];

export async function POST(req: Request) {
  const body = await readJson(req);
  const itemId = typeof body?.itemId === "string" ? body.itemId : undefined;
  const itemType = typeof body?.itemType === "string" ? (body.itemType as ItemType) : undefined;
  const data = getNuclearDashboardData();
  const selection = findSelection(data, itemType, itemId);
  if (!selection) {
    return Response.json({ error: "Unknown nuclear dashboard item" }, { status: 400 });
  }
  return explainItem(
    req,
    "nuclear",
    `${selection.type}:${itemId}`,
    420,
    nuclearExplainSystemPrompt(data),
    `Explain this ${selection.type}: ${JSON.stringify(selection.item)}`,
  );
}

function findSelection(
  data: ReturnType<typeof getNuclearDashboardData>,
  itemType: ItemType | undefined,
  itemId: string | undefined,
): NuclearExplainable | null {
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
