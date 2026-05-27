import type { Metadata } from "next";
import NuclearDashboard from "@/components/nuclear/NuclearDashboard";
import { getNuclearDashboardData } from "@/lib/nuclearMetrics";

export const metadata: Metadata = {
  title: "Nuclear Energy Monitor",
  description:
    "Nuclear energy dashboard with live market checks, uranium proxies, SMRs, fuel-cycle bottlenecks, and hyperscaler power demand.",
};

export default function NuclearPage() {
  const data = getNuclearDashboardData();
  return <NuclearDashboard data={data} />;
}
