"use client";

import useSWR from "swr";
import { Panel } from "@/components/btc/layout/Panel";
import { Gauge } from "@/components/btc/metrics/Gauge";
import { FngSparkline } from "@/components/btc/charts/FngSparkline";
import type { FngResponse } from "@/lib/btc/types";

const fetcher = (u: string) =>
  fetch(u).then((r) => {
    if (!r.ok) throw new Error(`${r.status}`);
    return r.json();
  });

export function FngPanel() {
  const { data, error } = useSWR<FngResponse>("/api/fng", fetcher, {
    refreshInterval: 60 * 60 * 1000,
  });

  return (
    <Panel title="FEAR & GREED" right="alternative.me" className="h-full">
      {error ? (
        <div className="p-3 mono text-[12px] text-[var(--accent-red)]">
          F&G fetch failed
        </div>
      ) : data ? (
        <div className="flex flex-col h-full">
          <Gauge
            value={data.current.v}
            label="current"
            classification={data.current.label}
          />
          <div className="px-3 pb-1 mono text-[10px] tracking-widest uppercase text-[var(--text-tertiary)]">
            365d
          </div>
          <FngSparkline data={data} />
        </div>
      ) : (
        <div className="p-3 mono text-[12px] text-[var(--text-tertiary)]">
          loading…
        </div>
      )}
    </Panel>
  );
}
