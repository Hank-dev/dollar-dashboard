import { getDashboardData } from "@/lib/fetchers";
import { readJson } from "@/lib/llmGuard";
import { explainItem } from "@/lib/llmRoutes";
import { explainSystemPrompt } from "@/lib/prompts";

export async function POST(req: Request) {
  const metricId = (await readJson(req))?.metricId;
  const data = await getDashboardData();
  const metric = data.metrics.find((m) => m.id === metricId);
  if (!metric) {
    return Response.json({ error: "Unknown metric" }, { status: 400 });
  }
  return explainItem(
    req,
    "dollar",
    `${metric.id}:${metric.value}`,
    400,
    explainSystemPrompt(data),
    `Explain the metric "${metric.label}", currently ${metric.value} (${metric.context}).`,
  );
}
