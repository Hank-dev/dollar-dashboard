"use client";

import useSWR from "swr";
import { Panel } from "@/components/btc/layout/Panel";
import type { InterpretationResponse } from "@/lib/btc/types";

const fetcher = (u: string) =>
  fetch(u).then((r) => {
    if (!r.ok) {
      return r.json().then((j) => {
        throw new Error(j?.error ?? `${r.status}`);
      });
    }
    return r.json();
  });

function formatAge(ts: number): string {
  const secs = Math.max(0, Math.floor(Date.now() / 1000) - ts);
  if (secs < 60) return `${secs}s ago`;
  const mins = Math.floor(secs / 60);
  if (mins < 60) return `${mins}m ago`;
  const hrs = Math.floor(mins / 60);
  if (hrs < 24) return `${hrs}h ago`;
  const days = Math.floor(hrs / 24);
  return `${days}d ago`;
}

export function InterpretationPanel() {
  const { data, error } = useSWR<InterpretationResponse>(
    "/api/interpretation",
    fetcher,
    { refreshInterval: 60 * 60 * 1000 },
  );

  const right =
    data && "generatedAt" in data
      ? `claude haiku 4.5 · ${formatAge(data.generatedAt)}`
      : data && "disabled" in data
        ? "disabled"
        : "";

  return (
    <Panel title="MARKET INTERPRETATION (AI)" right={right} className="h-full">
      {error ? (
        <div className="p-3 mono text-[11px] text-[var(--accent-red)] whitespace-pre-wrap">
          {String(error.message ?? error)}
        </div>
      ) : !data ? (
        <div className="p-3 mono text-[12px] text-[var(--text-tertiary)]">
          loading…
        </div>
      ) : "disabled" in data ? (
        <div className="p-3 mono text-[11px] text-[var(--text-tertiary)] leading-relaxed">
          {data.reason}
        </div>
      ) : (
        <div className="p-3 text-[12px] leading-relaxed text-[var(--text-primary)] whitespace-pre-wrap">
          {data.text}
        </div>
      )}
    </Panel>
  );
}
