"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import AskBox from "@/components/AskBox";
import { GroupIcon } from "@/components/Dashboard";
import FrontierModelChart from "@/components/ai/FrontierModelChart";
import {
  AI_GROUPS,
  type AiDashboardData,
  type AiExplainable,
  type AiMarketMetric,
  type AiPlayer,
  type AiTechSignal,
} from "@/lib/aiMetrics";
import {
  STATUS_COLOR,
  STATUS_LABEL,
  type Status,
} from "@/lib/metrics";

const DATE_FMT = new Intl.DateTimeFormat("en-GB", {
  day: "numeric",
  month: "short",
  year: "numeric",
  timeZone: "UTC",
});

export default function AiDashboard({ data }: { data: AiDashboardData }) {
  const [openItem, setOpenItem] = useState<AiExplainable | null>(null);
  const displayDate = DATE_FMT.format(
    new Date(data.snapshotDate + "T00:00:00Z"),
  );

  const publicPlayers = data.players.filter((p) => p.kind === "public");
  const privatePlayers = data.players.filter((p) => p.kind === "private");

  return (
    <main className="mx-auto max-w-[1080px] px-5 py-8 sm:py-10">
      <header className="flex flex-col gap-3 border-b border-[var(--border)] pb-5 sm:flex-row sm:items-end sm:justify-between">
        <div>
          <p className="mono text-[11px] font-medium tracking-[0.16em] uppercase text-[var(--accent-purple)] mb-1">
            Investor / founder briefing
          </p>
          <h1 className="mt-1 text-[21px] font-semibold tracking-tight text-[var(--text-primary)] sm:text-[24px]">
            AI &amp; Agent World Monitor
          </h1>
        </div>
        <p className="mono text-[11.5px] font-medium uppercase tracking-wide text-[var(--text-tertiary)]">
          Curated snapshot · {displayDate}
        </p>
      </header>

      <section
        aria-label="Verdict"
        className="mt-6 border border-[var(--border)] bg-[var(--bg-surface)] p-4 sm:p-5"
        style={{ borderLeft: "3px solid var(--dash-ai)" }}
      >
        <p className="mono text-[10.5px] font-medium tracking-[0.12em] uppercase text-[var(--accent-purple)]">
          Verdict — capital cycle meets agent adoption
        </p>
        <p className="mt-2 max-w-[880px] text-[13.5px] leading-relaxed text-[var(--text-primary)]">
          {data.verdict}
        </p>
      </section>

      <StatusLegend />

      <section className="mt-7">
        <h2 className="mb-3 flex items-center gap-2 mono text-[10.5px] font-medium tracking-[0.12em] uppercase text-[var(--text-secondary)]">
          <GroupIcon name="activity" />
          Market state
        </h2>
        <div className="grid gap-2.5 md:grid-cols-2 xl:grid-cols-3">
          {data.marketMetrics.map((metric) => (
            <MetricTile
              key={metric.id}
              metric={metric}
              onSelect={() => setOpenItem({ type: "metric", item: metric })}
            />
          ))}
        </div>
      </section>

      <section className="mt-9 grid gap-7 lg:grid-cols-[1fr_1fr]">
        <PlayerPanel
          title="Public-market leaders"
          players={publicPlayers}
          onSelect={(player) => setOpenItem({ type: "player", item: player })}
        />
        <PlayerPanel
          title="Private labs & agent platforms"
          players={privatePlayers}
          onSelect={(player) => setOpenItem({ type: "player", item: player })}
        />
      </section>

      <section className="mt-9">
        <h2 className="mb-3 flex items-center gap-2 mono text-[10.5px] font-medium tracking-[0.12em] uppercase text-[var(--text-secondary)]">
          <GroupIcon name="trending-up" />
          Frontier model price / intelligence graph
        </h2>
        <FrontierModelChart />
      </section>

      <section className="mt-9">
        <h2 className="mb-3 flex items-center gap-2 mono text-[10.5px] font-medium tracking-[0.12em] uppercase text-[var(--text-secondary)]">
          <GroupIcon name="trending-up" />
          Technology radar
        </h2>
        <div className="grid gap-2.5 md:grid-cols-2">
          {data.techSignals.map((signal) => (
            <SignalTile
              key={signal.id}
              signal={signal}
              onSelect={() => setOpenItem({ type: "signal", item: signal })}
            />
          ))}
        </div>
      </section>

      <section className="mt-9">
        <h2 className="mb-3 flex items-center gap-2 mono text-[10.5px] font-medium tracking-[0.12em] uppercase text-[var(--text-secondary)]">
          <GroupIcon name="message" />
          Ask the AI dashboard
        </h2>
        <AskBox
          endpoint="/api/ai/ask"
          placeholder="e.g. Which part of the AI stack has the strongest business model right now?"
          snapshotNote={`grounded in the ${displayDate} AI snapshot`}
        />
      </section>

      <footer className="mt-10 border-t border-[var(--border)] pt-5 text-[11.5px] leading-relaxed text-[var(--text-tertiary)]">
        <p>
          Sources are linked on each item. Public-company values are labeled as
          market cap; private-company values are labeled as valuation estimates.
        </p>
        <p className="mt-2">
          This dashboard is informational market and technology context, not
          financial advice. Private valuations and adoption figures may be
          directional, delayed, or reported by third parties.
        </p>
      </footer>

      <AiExplainDrawer
        selection={openItem}
        onClose={() => setOpenItem(null)}
      />
    </main>
  );
}

function StatusLegend() {
  const statuses: Status[] = ["calm", "neutral", "elevated", "stressed"];
  return (
    <div className="mt-6 flex flex-wrap items-center gap-x-5 gap-y-2 mono text-[11px] text-[var(--text-secondary)]">
      {statuses.map((status) => (
        <span key={status} className="inline-flex items-center gap-2">
          <span className={`dot ${status}`} />
          {STATUS_LABEL[status]}
        </span>
      ))}
    </div>
  );
}

function StatusDot({ status }: { status: Status }) {
  return <span className={`dot ${status}`} />;
}

function MetricTile({
  metric,
  onSelect,
}: {
  metric: AiMarketMetric;
  onSelect: () => void;
}) {
  const group = AI_GROUPS[metric.group];
  return (
    <button
      type="button"
      onClick={onSelect}
      className="group flex min-h-[142px] flex-col justify-between border border-[var(--border)] bg-[var(--bg-surface)] p-3 text-left transition-colors hover:bg-[var(--bg-surface-hover)] focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--text-secondary)]"
      style={{ borderRadius: 8 }}
      aria-label={`${metric.label}, ${metric.value}, status ${STATUS_LABEL[metric.status]}. Click to explain.`}
    >
      <div>
        <div className="flex items-center justify-between gap-2">
          <span className="inline-flex min-w-0 items-center gap-1.5 text-[12px] text-[var(--text-secondary)]">
            <StatusDot status={metric.status} />
            <span className="truncate">{metric.label}</span>
          </span>
          <span className="text-[10px] text-[var(--text-tertiary)] opacity-0 transition-opacity group-hover:opacity-100">
            i
          </span>
        </div>
        <p className="mt-2 text-[24px] font-medium tabular-nums text-[var(--text-primary)]">
          {metric.value}
        </p>
        <p className="mt-1 text-[11.5px] text-[var(--text-tertiary)]">
          {group.title} · {metric.context}
        </p>
      </div>
      <p className="mt-3 text-[12.5px] leading-relaxed text-[var(--text-secondary)]">
        {metric.detail}
      </p>
    </button>
  );
}

function PlayerPanel({
  title,
  players,
  onSelect,
}: {
  title: string;
  players: AiPlayer[];
  onSelect: (player: AiPlayer) => void;
}) {
  return (
    <section>
      <h2 className="mb-3 flex items-center gap-2 mono text-[10.5px] font-medium tracking-[0.12em] uppercase text-[var(--text-secondary)]">
        <GroupIcon name="building-bank" />
        {title}
      </h2>
      <div className="overflow-hidden border border-[var(--border)] bg-[var(--bg-surface)]" style={{ borderRadius: 8 }}>
        {players.map((player, index) => (
          <button
            key={player.id}
            type="button"
            onClick={() => onSelect(player)}
            className="grid w-full gap-2 border-[var(--border)] p-3 text-left transition-colors hover:bg-[var(--bg-surface-hover)] focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--text-secondary)] sm:grid-cols-[minmax(120px,0.85fr)_minmax(110px,0.55fr)_1.3fr]"
            style={{ borderTopWidth: index === 0 ? 0 : 1 }}
          >
            <div className="min-w-0">
              <p className="flex items-center gap-1.5 text-[13px] font-medium text-[var(--text-primary)]">
                <StatusDot status={player.status} />
                <span className="truncate">{player.name}</span>
              </p>
              <p className="mt-0.5 text-[11.5px] text-[var(--text-tertiary)]">
                {player.ticker ? `${player.ticker} · ` : ""}
                {player.category}
              </p>
            </div>
            <div>
              <p className="text-[10.5px] font-semibold uppercase tracking-[0.12em] text-[var(--accent-purple)]">
                {player.kind === "public" ? "Market cap" : "Private valuation"}
              </p>
              <p className="mt-0.5 text-[15px] font-medium tabular-nums text-[var(--text-primary)]">
                {player.marketCap ?? player.valuationEstimate}
              </p>
            </div>
            <p className="text-[12.5px] leading-relaxed text-[var(--text-secondary)]">
              {player.aiExposure}
            </p>
          </button>
        ))}
      </div>
    </section>
  );
}

function SignalTile({
  signal,
  onSelect,
}: {
  signal: AiTechSignal;
  onSelect: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onSelect}
      className="group min-h-[154px] border border-[var(--border)] bg-[var(--bg-surface)] p-3 text-left transition-colors hover:bg-[var(--bg-surface-hover)] focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--text-secondary)]"
      style={{ borderRadius: 8 }}
      aria-label={`${signal.track}: ${signal.label}. Click to explain.`}
    >
      <div className="flex items-start justify-between gap-3">
        <div className="min-w-0">
          <p className="text-[10.5px] font-semibold uppercase tracking-[0.12em] text-[var(--accent-purple)]">
            {signal.track}
          </p>
          <h3 className="mt-1 flex items-center gap-1.5 text-[13px] font-medium text-[var(--text-primary)]">
            <StatusDot status={signal.status} />
            <span>{signal.label}</span>
          </h3>
        </div>
        <span className="text-[10px] text-[var(--text-tertiary)] opacity-0 transition-opacity group-hover:opacity-100">
          i
        </span>
      </div>
      <p className="mt-3 text-[12.5px] leading-relaxed text-[var(--text-secondary)]">
        {signal.summary}
      </p>
      <p className="mt-3 text-[11.5px] leading-relaxed text-[var(--text-tertiary)]">
        Watch: {signal.watchNext}
      </p>
    </button>
  );
}

type CacheEntry = { explanation?: string; error?: string };

function AiExplainDrawer({
  selection,
  onClose,
}: {
  selection: AiExplainable | null;
  onClose: () => void;
}) {
  const [cache, setCache] = useState<Record<string, CacheEntry>>({});
  const [loadingKey, setLoadingKey] = useState<string | null>(null);
  const panelRef = useRef<HTMLDivElement>(null);
  const closeBtnRef = useRef<HTMLButtonElement>(null);
  const inflight = useRef<Record<string, boolean>>({});

  const key = selection ? `${selection.type}:${selection.item.id}` : "";

  const fetchExplanation = useCallback(
    async (nextSelection: AiExplainable) => {
      const nextKey = `${nextSelection.type}:${nextSelection.item.id}`;
      if (cache[nextKey]?.explanation || inflight.current[nextKey]) return;
      inflight.current[nextKey] = true;
      setLoadingKey(nextKey);
      try {
        const res = await fetch("/api/ai/explain", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            itemType: nextSelection.type,
            itemId: nextSelection.item.id,
          }),
        });
        if (!res.ok) {
          const body = await res.json().catch(() => ({}));
          throw new Error(body.error || `Request failed (${res.status})`);
        }
        const body = (await res.json()) as { explanation: string };
        setCache((current) => ({
          ...current,
          [nextKey]: { explanation: body.explanation },
        }));
      } catch (err) {
        const message =
          err instanceof Error ? err.message : "Could not generate explanation.";
        setCache((current) => ({ ...current, [nextKey]: { error: message } }));
      } finally {
        inflight.current[nextKey] = false;
        setLoadingKey((current) => (current === nextKey ? null : current));
      }
    },
    [cache],
  );

  useEffect(() => {
    if (!selection) return;
    const timeout = window.setTimeout(() => {
      fetchExplanation(selection);
    }, 0);
    return () => window.clearTimeout(timeout);
  }, [selection, fetchExplanation]);

  useEffect(() => {
    if (!selection) return;
    const prevActive = document.activeElement as HTMLElement | null;
    closeBtnRef.current?.focus();
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        onClose();
      } else if (e.key === "Tab" && panelRef.current) {
        const focusables = panelRef.current.querySelectorAll<HTMLElement>(
          'button, [href], input, textarea, [tabindex]:not([tabindex="-1"])',
        );
        if (focusables.length === 0) return;
        const first = focusables[0];
        const last = focusables[focusables.length - 1];
        if (e.shiftKey && document.activeElement === first) {
          e.preventDefault();
          last.focus();
        } else if (!e.shiftKey && document.activeElement === last) {
          e.preventDefault();
          first.focus();
        }
      }
    };
    document.addEventListener("keydown", onKey);
    document.body.style.overflow = "hidden";
    return () => {
      document.removeEventListener("keydown", onKey);
      document.body.style.overflow = "";
      prevActive?.focus?.();
    };
  }, [selection, onClose]);

  if (!selection) return null;

  const title = itemTitle(selection);
  const subtitle = itemSubtitle(selection);
  const status = itemStatus(selection);
  const source = selection.item.source;
  const entry = cache[key];
  const loading = loadingKey === key && !entry?.explanation;

  return (
    <div className="fixed inset-0 z-50">
      <div
        className="drawer-overlay absolute inset-0"
        style={{ background: "var(--overlay)" }}
        onClick={onClose}
        aria-hidden
      />
      <aside
        ref={panelRef}
        role="dialog"
        aria-modal="true"
        aria-labelledby="ai-explain-title"
        className="drawer-panel absolute right-0 top-0 flex h-full w-full max-w-[460px] flex-col border-l border-[var(--border)] bg-[var(--bg-surface)]"
      >
        <header className="flex items-start justify-between gap-4 border-b border-[var(--border)] px-5 py-4">
          <div className="min-w-0">
            <p className="text-[11px] font-semibold uppercase tracking-[0.12em] text-[var(--text-tertiary)]">
              AI dashboard explanation
            </p>
            <h3
              id="ai-explain-title"
              className="mt-1 flex items-center gap-2 text-[14px] font-medium text-[var(--text-primary)]"
            >
              <StatusDot status={status} />
              {title}
            </h3>
            <p className="mt-1 text-[12px] leading-relaxed text-[var(--text-tertiary)]">
              {subtitle} · {STATUS_LABEL[status]}
            </p>
          </div>
          <button
            ref={closeBtnRef}
            type="button"
            onClick={onClose}
            aria-label="Close"
            className="-mr-1 -mt-1 grid h-8 w-8 place-items-center text-[var(--text-secondary)] hover:bg-[var(--bg-surface-hover)] focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--text-secondary)]"
            style={{ borderRadius: 6 }}
          >
            <svg
              width="16"
              height="16"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="1.75"
              strokeLinecap="round"
              strokeLinejoin="round"
              aria-hidden
            >
              <line x1="18" y1="6" x2="6" y2="18" />
              <line x1="6" y1="6" x2="18" y2="18" />
            </svg>
          </button>
        </header>

        <div className="flex-1 overflow-y-auto px-5 py-5">
          {loading && (
            <div className="space-y-2.5" aria-live="polite" aria-busy="true">
              <div className="skeleton-line h-3 w-full" />
              <div className="skeleton-line h-3 w-[92%]" />
              <div className="skeleton-line h-3 w-[78%]" />
              <div className="skeleton-line h-3 w-[88%]" />
              <span className="sr-only">Loading explanation...</span>
            </div>
          )}

          {!loading && entry?.explanation && (
            <p className="whitespace-pre-line text-[13.5px] leading-relaxed text-[var(--text-primary)]">
              {entry.explanation}
            </p>
          )}

          {!loading && entry?.error && (
            <div className="space-y-3 text-[13px]">
              <p className="text-[var(--text-primary)]">{entry.error}</p>
              <button
                type="button"
                onClick={() => {
                  setCache((current) => {
                    const next = { ...current };
                    delete next[key];
                    return next;
                  });
                  fetchExplanation(selection);
                }}
                className="border border-[var(--border-strong)] px-3 py-1.5 text-[12px] text-[var(--text-primary)] hover:bg-[var(--bg-surface-hover)] focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--text-secondary)]"
                style={{ borderRadius: 6 }}
              >
                Retry
              </button>
            </div>
          )}

          <div className="mt-5 border-t border-[var(--border)] pt-4 text-[11.5px] leading-relaxed text-[var(--text-tertiary)]">
            <p>
              Source:{" "}
              <a
                href={source.url}
                target="_blank"
                rel="noreferrer"
                className="underline decoration-[var(--border-strong)] underline-offset-2 hover:text-[var(--text-secondary)]"
              >
                {source.name}
              </a>
            </p>
            <p>
              As of {source.asOf} · confidence {source.confidence}
            </p>
          </div>
        </div>

        <footer className="border-t border-[var(--border)] px-5 py-3 text-[11px] text-[var(--text-tertiary)]">
          Informational market context, not financial advice.
        </footer>
      </aside>
    </div>
  );
}

function itemTitle(selection: AiExplainable): string {
  if (selection.type === "player") return selection.item.name;
  return selection.item.label;
}

function itemSubtitle(selection: AiExplainable): string {
  if (selection.type === "player") {
    const value = selection.item.marketCap ?? selection.item.valuationEstimate;
    const label =
      selection.item.kind === "public" ? "market cap" : "private valuation";
    return `${selection.item.category} · ${label} ${value}`;
  }
  if (selection.type === "metric") {
    return `${selection.item.value} · ${selection.item.context}`;
  }
  return `${selection.item.track} · watch next`;
}

function itemStatus(selection: AiExplainable): Status {
  return selection.item.status;
}
