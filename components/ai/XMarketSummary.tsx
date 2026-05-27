"use client";

import { useEffect, useMemo, useState } from "react";
import type { XSummaryError, XSummaryResponse } from "@/lib/xSummary";

export default function XMarketSummary() {
  const [data, setData] = useState<XSummaryResponse | null>(null);
  const [error, setError] = useState<XSummaryError | null>(null);
  const [loading, setLoading] = useState(true);
  const [refreshKey, setRefreshKey] = useState(0);

  useEffect(() => {
    const controller = new AbortController();

    async function load() {
      setLoading(true);
      setError(null);
      try {
        const res = await fetch("/api/ai/x-summary", {
          cache: "no-store",
          signal: controller.signal,
        });
        const body = (await res.json()) as XSummaryResponse | XSummaryError;
        if (!res.ok) {
          setError(body as XSummaryError);
          setData(null);
          return;
        }
        setData(body as XSummaryResponse);
      } catch (err) {
        if ((err as { name?: string }).name === "AbortError") return;
        setError({
          error: err instanceof Error ? err.message : "Could not load X summary.",
        });
        setData(null);
      } finally {
        setLoading(false);
      }
    }

    load();
    return () => controller.abort();
  }, [refreshKey]);

  const checkedLabel = useMemo(() => {
    if (!data) return null;
    return new Intl.DateTimeFormat("en-GB", {
      day: "numeric",
      month: "short",
      hour: "2-digit",
      minute: "2-digit",
    }).format(new Date(data.checkedAt));
  }, [data]);

  const nextRefreshLabel = useMemo(() => {
    if (!data) return null;
    return new Intl.DateTimeFormat("en-GB", {
      day: "numeric",
      month: "short",
      hour: "2-digit",
      minute: "2-digit",
    }).format(new Date(data.nextRefreshAt));
  }, [data]);

  const sections = useMemo(
    () => (data ? parseSummarySections(data.summary) : []),
    [data],
  );

  return (
    <div className="border border-[var(--border)] bg-[var(--bg-surface)] p-4 sm:p-5" style={{ borderRadius: 8 }}>
      <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
        <div>
          <h3 className="text-[14px] font-medium text-[var(--text-primary)]">
            X stories via Grok
          </h3>
          <p className="mt-1 max-w-[760px] text-[11.5px] leading-relaxed text-[var(--text-tertiary)]">
            Grok searches recent X posts and summarizes the AI / agent market
            conversation. The server only refreshes stories twice per day.
          </p>
        </div>
        <div className="flex shrink-0 items-center gap-3">
          {checkedLabel && (
            <span className="mono text-[10.5px] uppercase tracking-wide text-[var(--text-tertiary)]">
              Checked {checkedLabel}
            </span>
          )}
          <button
            type="button"
            onClick={() => setRefreshKey((key) => key + 1)}
            disabled={loading}
            className="border border-[var(--border-strong)] px-3 py-1.5 text-[12px] text-[var(--text-primary)] hover:bg-[var(--bg-surface-hover)] disabled:opacity-50 focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--text-secondary)]"
            style={{ borderRadius: 6 }}
          >
            {loading ? "Loading..." : "Reload"}
          </button>
        </div>
      </div>

      <div className="mt-4 border-t border-[var(--border)] pt-4">
        {loading && (
          <div className="space-y-2.5" aria-live="polite" aria-busy="true">
            <div className="skeleton-line h-3 w-full" />
            <div className="skeleton-line h-3 w-[92%]" />
            <div className="skeleton-line h-3 w-[84%]" />
            <span className="sr-only">Loading X summary...</span>
          </div>
        )}

        {!loading && error && (
          <div className="text-[12.5px] leading-relaxed text-[var(--text-primary)]">
            <p>{error.error}</p>
            {error.setup && (
              <p className="mt-2 text-[var(--text-tertiary)]">{error.setup}</p>
            )}
          </div>
        )}

        {!loading && data && (
          <>
            <div className="grid gap-2 sm:grid-cols-3">
              <SummaryStat label="Cadence" value={`${data.refreshCadenceHours}h`} />
              <SummaryStat label="Cache" value={data.cacheStatus} />
              <SummaryStat
                label="Next fetch"
                value={nextRefreshLabel ?? "scheduled"}
              />
            </div>

            <div className="mt-4 grid gap-4 lg:grid-cols-[minmax(0,1.15fr)_minmax(280px,0.85fr)]">
              <div className="grid gap-3">
                {sections.map((section) => (
                  <section
                    key={section.title}
                    className="border border-[var(--border)] bg-[var(--bg-base)] p-3"
                    style={{ borderRadius: 6 }}
                  >
                    <h4 className="mono text-[10.5px] font-medium uppercase tracking-[0.12em] text-[var(--accent-purple)]">
                      {section.title}
                    </h4>
                    <ul className="mt-2 space-y-2 text-[12.5px] leading-relaxed text-[var(--text-primary)]">
                      {section.items.map((item) => (
                        <li key={item} className="flex gap-2">
                          <span className="mt-[0.65em] h-1 w-1 shrink-0 rounded-full bg-[var(--accent-purple)]" />
                          <span>{item}</span>
                        </li>
                      ))}
                    </ul>
                  </section>
                ))}
              </div>

              <aside
                className="border border-[var(--border)] bg-[var(--bg-base)] p-3"
                style={{ borderRadius: 6 }}
              >
                <div className="flex items-center justify-between gap-3">
                  <h4 className="mono text-[10.5px] font-medium uppercase tracking-[0.12em] text-[var(--text-tertiary)]">
                    X feed
                  </h4>
                  <span className="mono text-[10px] text-[var(--text-tertiary)]">
                    {data.feed.length} posts
                  </span>
                </div>
                <div className="mt-2 grid gap-2">
                  {data.feed.length > 0 ? (
                    data.feed.map((item) => (
                      <a
                        key={item.url}
                        href={item.url}
                        target="_blank"
                        rel="noreferrer"
                        className="border border-[var(--border)] bg-[var(--bg-surface)] p-3 text-[12px] leading-relaxed text-[var(--text-primary)] hover:bg-[var(--bg-surface-hover)] focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--text-secondary)]"
                        style={{ borderRadius: 6 }}
                      >
                        <span className="mono block text-[10.5px] uppercase tracking-[0.12em] text-[var(--accent-purple)]">
                          {item.handle ? `@${item.handle}` : "X post"}
                        </span>
                        <span className="mt-1 block text-[var(--text-secondary)]">
                          {feedTitle(item.title)}
                        </span>
                      </a>
                    ))
                  ) : (
                    <p className="text-[12.5px] leading-relaxed text-[var(--text-tertiary)]">
                      Grok returned a summary, but no direct X post links were
                      included in the citations for this refresh window.
                    </p>
                  )}
                </div>
              </aside>
            </div>

            {data.citations.length > 0 && (
              <div className="mt-5">
                <p className="mono text-[10.5px] font-medium uppercase tracking-[0.12em] text-[var(--text-tertiary)]">
                  Sources
                </p>
                <div className="mt-2 flex flex-wrap gap-2">
                  {data.citations.map((citation, index) =>
                    citation.url ? (
                      <a
                        key={`${citation.url}-${index}`}
                        href={citation.url}
                        target="_blank"
                        rel="noreferrer"
                        className="border border-[var(--border)] px-2.5 py-1 text-[11.5px] text-[var(--text-secondary)] hover:bg-[var(--bg-surface-hover)] hover:text-[var(--text-primary)]"
                        style={{ borderRadius: 6 }}
                      >
                        {sourceLabel(citation.title, index)}
                      </a>
                    ) : (
                      <span
                        key={`${citation.title}-${index}`}
                        className="border border-[var(--border)] px-2.5 py-1 text-[11.5px] text-[var(--text-secondary)]"
                        style={{ borderRadius: 6 }}
                      >
                        {sourceLabel(citation.title, index)}
                      </span>
                    ),
                  )}
                </div>
              </div>
            )}
          </>
        )}
      </div>
    </div>
  );
}

function SummaryStat({ label, value }: { label: string; value: string }) {
  return (
    <div className="border border-[var(--border)] bg-[var(--bg-base)] px-3 py-2" style={{ borderRadius: 6 }}>
      <p className="mono text-[10px] uppercase tracking-[0.12em] text-[var(--text-tertiary)]">
        {label}
      </p>
      <p className="mt-1 text-[12.5px] font-medium text-[var(--text-primary)]">
        {value}
      </p>
    </div>
  );
}

function parseSummarySections(summary: string): { title: string; items: string[] }[] {
  const sections: { title: string; items: string[] }[] = [];
  let current: { title: string; items: string[] } | null = null;

  for (const rawLine of summary.split("\n")) {
    const line = rawLine.trim();
    if (!line) continue;

    const heading = line.match(/^\*\*(.+?)\*\*:?$/);
    if (heading) {
      current = { title: heading[1], items: [] };
      sections.push(current);
      continue;
    }

    const bullet = line.match(/^[-*]\s+(.+)/);
    if (bullet) {
      if (!current) {
        current = { title: "Summary", items: [] };
        sections.push(current);
      }
      current.items.push(cleanSummaryText(bullet[1]));
      continue;
    }

    if (!current) {
      current = { title: "Summary", items: [] };
      sections.push(current);
    }
    current.items.push(cleanSummaryText(line));
  }

  return sections.length > 0
    ? sections.filter((section) => section.items.length > 0)
    : [{ title: "Summary", items: [cleanSummaryText(summary)] }];
}

function cleanSummaryText(value: string): string {
  return value
    .replace(/\*\*/g, "")
    .replace(/\[\[(\d+)\]\]\([^)]+\)/g, "[$1]")
    .replace(/\[([^\]]+)\]\([^)]+\)/g, "$1")
    .trim();
}

function sourceLabel(title: string, index: number): string {
  if (title.startsWith("http")) return `Source ${index + 1}`;
  return title.length > 42 ? `${title.slice(0, 39)}...` : title;
}

function feedTitle(title: string): string {
  if (/^\d+$/.test(title)) return "Open referenced post on X";
  return title.length > 120 ? `${title.slice(0, 117)}...` : title;
}
