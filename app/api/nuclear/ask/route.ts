import { answerQuestion } from "@/lib/llmRoutes";
import { fetchNuclearMarketLive } from "@/lib/nuclearLive";
import { getNuclearDashboardData } from "@/lib/nuclearMetrics";
import { nuclearAskSystemPrompt } from "@/lib/nuclearPrompts";

export async function POST(req: Request) {
  return answerQuestion(req, "nuclear ask", 850, async () => {
    const live = await fetchNuclearMarketLive().catch(() => null);
    return nuclearAskSystemPrompt(getNuclearDashboardData(), live);
  });
}
