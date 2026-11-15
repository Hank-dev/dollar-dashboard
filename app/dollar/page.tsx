import Dashboard from "@/components/Dashboard";
import { getDashboardData } from "@/lib/fetchers";

// Rendered per request; getDashboardData keeps its own 15-minute cache. A
// build-time prerender would bake the key-less fallback into the image.
export const dynamic = "force-dynamic";

export default async function Page() {
  const data = await getDashboardData();
  return <Dashboard data={data} />;
}
