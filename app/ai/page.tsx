import type { Metadata } from "next";
import AiDashboard from "@/components/ai/AiDashboard";
import { getAiDashboardDataLive } from "@/lib/aiMetrics";

export const revalidate = 3600;

export const metadata: Metadata = {
  title: "AI & Agent World Monitor",
  description:
    "Curated investor/founder dashboard for AI market leaders, private labs, agent adoption, and technology signals.",
};

export default async function AiPage() {
  const data = await getAiDashboardDataLive();
  return <AiDashboard data={data} />;
}
