"use client";

import { useState } from "react";
import NuclearDashboardContent from "./NuclearDashboardContent";
import NuclearExplainDrawer from "./NuclearExplainDrawer";
import type { NuclearDashboardData, NuclearExplainable } from "@/lib/nuclearMetrics";

export default function NuclearClientShell({ data }: { data: NuclearDashboardData }) {
  const [openItem, setOpenItem] = useState<NuclearExplainable | null>(null);

  return (
    <>
      <NuclearDashboardContent
        data={data}
        onOpenExplain={setOpenItem}
      />
      <NuclearExplainDrawer
        selection={openItem}
        onClose={() => setOpenItem(null)}
      />
    </>
  );
}
