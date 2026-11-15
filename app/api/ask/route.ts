import { getDashboardData } from "@/lib/fetchers";
import { answerQuestion } from "@/lib/llmRoutes";
import { askSystemPrompt } from "@/lib/prompts";

export async function POST(req: Request) {
  return answerQuestion(req, "ask", 800, async () =>
    askSystemPrompt(await getDashboardData()),
  );
}
