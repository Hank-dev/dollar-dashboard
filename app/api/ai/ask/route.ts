import { getAiDashboardData } from "@/lib/aiMetrics";
import { aiAskSystemPrompt } from "@/lib/aiPrompts";
import { answerQuestion } from "@/lib/llmRoutes";

export async function POST(req: Request) {
  return answerQuestion(req, "ai ask", 850, () => aiAskSystemPrompt(getAiDashboardData()));
}
