"use client";

import { useEffect, useState } from "react";

interface BriefingData {
  briefing: string;
  generatedAt: string;
  dataPoints?: number;
}

export function DailyBriefing() {
  const [data, setData] = useState<BriefingData | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  function loadBriefing() {
    setLoading(true);
    setError(null);
    fetch("/api/daily-briefing")
      .then((r) => {
        if (!r.ok) throw new Error(`${r.status}`);
        return r.json();
      })
      .then((d) => setData(d))
      .catch((e) => setError(e.message))
      .finally(() => setLoading(false));
  }

  useEffect(() => {
    loadBriefing();
  }, []);

  const time = data?.generatedAt
    ? new Date(data.generatedAt).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })
    : null;

  return (
    <section className="mx-5 mb-5 border border-[var(--border)] bg-[var(--bg-surface)]">
      <div className="flex items-center justify-between px-4 py-2 border-b border-[var(--border)]" style={{ background: "linear-gradient(180deg, var(--bg-elevated), transparent)" }}>
        <span className="mono text-[10.5px] font-medium tracking-[0.12em] uppercase text-[var(--text-secondary)]">
          Claude &middot; daily briefing
        </span>
        <div className="flex items-center gap-3">
          {time && (
            <span className="mono text-[10px] text-[var(--text-tertiary)]">
              generated {time} UTC
            </span>
          )}
          <button
            onClick={loadBriefing}
            disabled={loading}
            className="mono text-[10px] tracking-[0.06em] uppercase px-2 py-0.5 border border-[var(--border)] text-[var(--text-tertiary)] hover:text-[var(--text-secondary)] hover:border-[var(--border-strong)] cursor-pointer disabled:opacity-50 transition-colors"
            style={{ background: "transparent", borderRadius: "var(--r-1)" }}
          >
            {loading ? "generating..." : "refresh"}
          </button>
        </div>
      </div>
      <div className="px-4 py-3">
        {loading && !data && (
          <div className="space-y-2">
            <div className="skeleton-line h-3 w-full" />
            <div className="skeleton-line h-3 w-[90%]" />
            <div className="skeleton-line h-3 w-[95%]" />
            <div className="skeleton-line h-3 w-[60%]" />
          </div>
        )}
        {error && !data && (
          <p className="mono text-[12px] text-[var(--accent-red)]">
            Failed to generate briefing: {error}
          </p>
        )}
        {data && (
          <div className="text-[13px] leading-[1.65] text-[var(--text-secondary)] space-y-3">
            {data.briefing.split("\n\n").map((para, i) => (
              <p key={i}>{para}</p>
            ))}
          </div>
        )}
      </div>
    </section>
  );
}
