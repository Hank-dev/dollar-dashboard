"use client";

import useSWR from "swr";
import { Panel } from "@/components/btc/layout/Panel";
import { MetricCard } from "@/components/btc/metrics/MetricCard";
import { BandIndicator } from "@/components/btc/metrics/BandIndicator";
import { fmtPrice, fmtSigned } from "@/lib/btc/format";
import type { RegimeResponse } from "@/lib/btc/types";

const fetcher = (u: string) =>
  fetch(u).then((r) => {
    if (!r.ok) throw new Error(`${r.status}`);
    return r.json();
  });

export function RegimePanel() {
  const { data, error } = useSWR<RegimeResponse>("/api/regime", fetcher, {
    refreshInterval: 30 * 60 * 1000,
  });

  const asOf =
    data?.asOf
      ? new Date(data.asOf * 1000).toISOString().slice(0, 10)
      : "loading…";

  return (
    <Panel title="REGIME" right={`as of ${asOf}`}>
      {error ? (
        <div className="p-3 mono text-[12px] text-[var(--accent-red)]">
          regime fetch failed: {String(error)}
        </div>
      ) : (
        <div className="flex flex-col">
          <BandIndicator
            label="MVRV Z-Score"
            value={data?.mvrvZ ?? null}
            min={-2}
            max={10}
            zones={[
              { from: -2, color: "var(--accent-green)", label: "deep value" },
              { from: 0, color: "#84cc16" },
              { from: 2, color: "#a3a3a3" },
              { from: 4, color: "var(--accent-amber)" },
              { from: 7, color: "var(--accent-red)", label: "top zone" },
            ]}
            formatValue={(v) => fmtSigned(v, 2)}
          />
          <BandIndicator
            label="NUPL"
            value={data?.nupl ?? null}
            min={-0.3}
            max={0.8}
            zones={[
              { from: -0.3, color: "var(--accent-green)", label: "capitulation" },
              { from: 0, color: "#84cc16", label: "hope/fear" },
              { from: 0.25, color: "#a3a3a3", label: "optimism" },
              { from: 0.5, color: "var(--accent-amber)", label: "belief" },
              { from: 0.75, color: "var(--accent-red)", label: "euphoria" },
            ]}
            formatValue={(v) => v.toFixed(2)}
          />
          <BandIndicator
            label="Mayer Multiple"
            value={data?.mayer ?? null}
            min={0.4}
            max={3.0}
            zones={[
              { from: 0.4, color: "var(--accent-green)" },
              { from: 0.8, color: "#84cc16" },
              { from: 1.2, color: "#a3a3a3" },
              { from: 1.8, color: "var(--accent-amber)" },
              { from: 2.4, color: "var(--accent-red)", label: "frothy" },
            ]}
            formatValue={(v) => v.toFixed(2)}
          />
          <BandIndicator
            label="Power-Law Z (dev)"
            value={data?.powerLawZ ?? null}
            min={-3}
            max={3}
            zones={[
              { from: -3, color: "var(--accent-green)" },
              { from: -1, color: "#a3a3a3" },
              { from: 1, color: "var(--accent-amber)" },
              { from: 2, color: "var(--accent-red)" },
            ]}
            formatValue={(v) => fmtSigned(v, 2)}
          />
          <MetricCard
            label="Realized Price"
            value={fmtPrice(data?.realizedPrice ?? null)}
            sub="cost basis of all coins"
          />
        </div>
      )}
    </Panel>
  );
}
