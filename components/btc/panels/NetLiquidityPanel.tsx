"use client";

import useSWR from "swr";
import { Panel } from "@/components/btc/layout/Panel";
import { NetLiquidityChart } from "@/components/btc/charts/NetLiquidityChart";
import { fmtSigned } from "@/lib/btc/format";
import type { NetLiquidityResponse } from "@/lib/btc/types";

const fetcher = (u: string) =>
  fetch(u).then((r) => {
    if (!r.ok) {
      return r.json().then((j) => {
        throw new Error(j?.error ?? `${r.status}`);
      });
    }
    return r.json();
  });

export function NetLiquidityPanel() {
  const { data, error } = useSWR<NetLiquidityResponse>(
    "/api/net-liquidity",
    fetcher,
    { refreshInterval: 6 * 60 * 60 * 1000 },
  );

  const right = data
    ? `${data.latest.toFixed(0)}B · 30d ${fmtSigned(data.delta30d, 0)}B`
    : "";

  return (
    <Panel title="NET LIQUIDITY (WALCL − TGA − RRP, $B)" right={right} className="h-full">
      {error ? (
        <div className="p-3 mono text-[11px] text-[var(--accent-red)] whitespace-pre-wrap">
          {String(error.message ?? error)}
        </div>
      ) : data ? (
        <NetLiquidityChart data={data} />
      ) : (
        <div className="p-3 mono text-[12px] text-[var(--text-tertiary)]">
          loading…
        </div>
      )}
    </Panel>
  );
}
