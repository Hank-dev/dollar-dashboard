"use client";

import { useEffect, useState } from "react";

interface BriefingData {
  briefing: string;
  generatedAt: string;
  dataPoints?: number;
}

async function fetchBriefing(signal?: AbortSignal): Promise<BriefingData> {
  const response = await fetch("/api/daily-briefing", { signal });
  if (!response.ok) throw new Error(`${response.status}`);
  return response.json();
}

function InlineBriefing({ text }: { text: string }) {
  return text.split(/(\*\*[^*]+\*\*)/g).map((part, index) =>
    part.startsWith("**") && part.endsWith("**") ? (
      <strong key={index} className="font-semibold text-[var(--text-primary)]">
        {part.slice(2, -2)}
      </strong>
    ) : (
      <span key={index}>{part}</span>
    ),
  );
}

export function DailyBriefing() {
  const [data, setData] = useState<BriefingData | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const controller = new AbortController();
    fetchBriefing(controller.signal)
      .then(setData)
      .catch((reason: unknown) => {
        if (reason instanceof DOMException && reason.name === "AbortError") return;
        setError(reason instanceof Error ? reason.message : "unknown error");
      })
      .finally(() => setLoading(false));
    return () => controller.abort();
  }, []);

  function loadBriefing() {
    setLoading(true);
    setError(null);
    fetchBriefing()
      .then(setData)
      .catch((reason: unknown) =>
        setError(reason instanceof Error ? reason.message : "unknown error"),
      )
      .finally(() => setLoading(false));
  }

  const time = data?.generatedAt
    ? new Date(data.generatedAt).toLocaleTimeString([], {
        hour: "2-digit",
        minute: "2-digit",
        timeZone: "UTC",
      })
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
            {data.briefing.split("\n\n").map((paragraph, index) => (
              <p key={index}><InlineBriefing text={paragraph} /></p>
            ))}
          </div>
        )}
      </div>
    </section>
  );
}
