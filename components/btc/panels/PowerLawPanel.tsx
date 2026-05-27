"use client";

import { useState } from "react";
import useSWR from "swr";
import { Panel } from "@/components/btc/layout/Panel";
import { PowerLawChart } from "@/components/btc/charts/PowerLawChart";
import { TimeframeSelector } from "@/components/btc/charts/TimeframeSelector";
import type { Timeframe } from "@/components/btc/charts/timeframe";
import type { PriceHistoryResponse } from "@/lib/btc/types";

const fetcher = (u: string) =>
  fetch(u).then((r) => {
    if (!r.ok) throw new Error(`${r.status}`);
    return r.json();
  });

export function PowerLawPanel() {
  const [timeframe, setTimeframe] = useState<Timeframe>("ALL");
  const { data, error } = useSWR<PriceHistoryResponse>(
    "/api/price-history",
    fetcher,
    { refreshInterval: 60 * 60 * 1000 },
  );

  const stats =
    data && data.powerLaw
      ? `slope ${data.powerLaw.b.toFixed(3)} · σ ${data.powerLaw.sigma.toFixed(3)}`
      : "";

  const right = (
    <div className="flex items-center gap-3">
      <TimeframeSelector value={timeframe} onChange={setTimeframe} />
      {stats ? <span>{stats}</span> : null}
    </div>
  );

  return (
    <Panel title="BTC POWER LAW (log)" right={right} className="h-full">
      {error ? (
        <div className="p-3 mono text-[12px] text-[var(--accent-red)]">
          chart fetch failed
        </div>
      ) : data ? (
        <PowerLawChart data={data} timeframe={timeframe} />
      ) : (
        <div className="p-3 mono text-[12px] text-[var(--text-tertiary)]">
          loading…
        </div>
      )}
    </Panel>
  );
}
