import type { Metadata } from "next";
import AiDashboard from "@/components/ai/AiDashboard";
import { getAiDashboardData } from "@/lib/aiMetrics";

export const metadata: Metadata = {
  title: "AI & Agent World Monitor",
  description:
    "Curated investor/founder dashboard for AI market leaders, private labs, agent adoption, and technology signals.",
};

export default function AiPage() {
  const data = getAiDashboardData();
  return <AiDashboard data={data} />;
}
