import Dashboard from "@/components/Dashboard";
import { getDashboardData } from "@/lib/fetchers";

export const revalidate = 900; // 15 minutes

export default async function Page() {
  const data = await getDashboardData();
  return <Dashboard data={data} />;
}
