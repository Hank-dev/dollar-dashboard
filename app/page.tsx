import { TickerStrip } from "@/components/TickerStrip";
import { SystemBar } from "@/components/SystemBar";
import { HomeHeader } from "@/components/HomeHeader";
import { HomeDashCards } from "@/components/HomeDashCards";
import { DailyBriefing } from "@/components/DailyBriefing";

export default function HomePage() {
  return (
    <div className="max-w-[1440px] mx-auto border-x border-[var(--border)] min-h-screen bg-[var(--bg-base)] flex flex-col">
      <TickerStrip />
      <HomeHeader />
      <HomeDashCards />
      <DailyBriefing />
      <div className="flex-1" />
      <SystemBar />
    </div>
  );
}
