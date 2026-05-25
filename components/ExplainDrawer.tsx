"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { STATUS_COLOR, STATUS_LABEL, type Metric } from "@/lib/metrics";

type CacheEntry = { explanation?: string; error?: string };

export default function ExplainDrawer({
  metric,
  onClose,
}: {
  metric: Metric | null;
  onClose: () => void;
}) {
  const [cache, setCache] = useState<Record<string, CacheEntry>>({});
  const [loadingId, setLoadingId] = useState<string | null>(null);
  const panelRef = useRef<HTMLDivElement>(null);
  const closeBtnRef = useRef<HTMLButtonElement>(null);
  const inflight = useRef<Record<string, boolean>>({});

  const fetchExplanation = useCallback(
    async (m: Metric) => {
      if (cache[m.id]?.explanation || inflight.current[m.id]) return;
      inflight.current[m.id] = true;
      setLoadingId(m.id);
      try {
        const res = await fetch("/api/explain", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ metricId: m.id }),
        });
        if (!res.ok) {
          const body = await res.json().catch(() => ({}));
          throw new Error(body.error || `Request failed (${res.status})`);
        }
        const data = (await res.json()) as { explanation: string };
        setCache((c) => ({ ...c, [m.id]: { explanation: data.explanation } }));
      } catch (err) {
        const message =
          err instanceof Error
            ? err.message
            : "Could not generate explanation.";
        setCache((c) => ({ ...c, [m.id]: { error: message } }));
      } finally {
        inflight.current[m.id] = false;
        setLoadingId((id) => (id === m.id ? null : id));
      }
    },
    [cache],
  );

  // Trigger fetch when a new metric is opened
  useEffect(() => {
    if (metric) fetchExplanation(metric);
  }, [metric, fetchExplanation]);

  // Escape to close + focus trap
  useEffect(() => {
    if (!metric) return;
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
  }, [metric, onClose]);

  if (!metric) return null;

  const entry = cache[metric.id];
  const loading = loadingId === metric.id && !entry?.explanation;

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
        aria-labelledby="explain-title"
        className="drawer-panel absolute right-0 top-0 flex h-full w-full max-w-[420px] flex-col border-l border-[var(--border)] bg-[var(--bg-surface)]"
      >
        <header className="flex items-start justify-between gap-4 border-b border-[var(--border)] px-5 py-4">
          <div className="min-w-0">
            <p className="text-[11px] font-semibold uppercase tracking-[0.12em] text-[var(--text-tertiary)]">
              Explanation
            </p>
            <h3
              id="explain-title"
              className="mt-1 flex items-center gap-2 text-[14px] font-medium text-[var(--text-primary)]"
            >
              <span
                aria-hidden
                className="inline-block h-[7px] w-[7px] rounded-full"
                style={{ backgroundColor: STATUS_COLOR[metric.status] }}
              />
              {metric.label}
            </h3>
            <p className="mt-1 text-[22px] font-medium tabular-nums text-[var(--text-primary)]">
              {metric.value}
            </p>
            <p className="text-[11.5px] text-[var(--text-tertiary)]">
              {metric.context} · {STATUS_LABEL[metric.status]}
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
              <span className="sr-only">Loading explanation…</span>
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
                  setCache((c) => {
                    const next = { ...c };
                    delete next[metric.id];
                    return next;
                  });
                  fetchExplanation(metric);
                }}
                className="border border-[var(--border-strong)] px-3 py-1.5 text-[12px] text-[var(--text-primary)] hover:bg-[var(--bg-surface-hover)] focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--text-secondary)]"
                style={{ borderRadius: 6 }}
              >
                Retry
              </button>
            </div>
          )}
        </div>

        <footer className="border-t border-[var(--border)] px-5 py-3 text-[11px] text-[var(--text-tertiary)]">
          Informational market context, not financial advice.
        </footer>
      </aside>
    </div>
  );
}
