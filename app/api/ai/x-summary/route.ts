import { createXSummaryRoute } from "@/lib/xSummaryRoute";

export const dynamic = "force-dynamic";

export const GET = createXSummaryRoute({
  key: "ai",
  subject: "AI and agent companies",
  focus:
    "frontier models, OpenAI, Anthropic, Google Gemini, xAI/Grok, Meta AI, NVIDIA AI demand, Cursor/Anysphere, coding agents, enterprise agents, model pricing, benchmark releases, safety/regulation, and AI infrastructure",
  narrativesHeading: "Top X narratives",
  preferredSources: "company accounts, researchers, builders, investors, and credible reporters",
});
