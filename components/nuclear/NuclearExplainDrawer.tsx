"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { StatusDot } from "./NuclearDashboardContent";
import type { NuclearExplainable } from "@/lib/nuclearMetrics";
import { STATUS_LABEL } from "@/lib/metrics";

type CacheEntry = { explanation?: string; error?: string };

export default function NuclearExplainDrawer({
  selection,
  onClose,
}: {
  selection: NuclearExplainable | null;
  onClose: () => void;
}) {
  const [cache, setCache] = useState<Record<string, CacheEntry>>({});
  const [loadingKey, setLoadingKey] = useState<string | null>(null);
  const panelRef = useRef<HTMLDivElement>(null);
  const closeBtnRef = useRef<HTMLButtonElement>(null);
  const inflight = useRef<Record<string, boolean>>({});

  const key = selection ? `${selection.type}:${selection.item.id}` : "";

  const fetchExplanation = useCallback(
    async (nextSelection: NuclearExplainable) => {
      const nextKey = `${nextSelection.type}:${nextSelection.item.id}`;
      if (cache[nextKey]?.explanation || inflight.current[nextKey]) return;
      inflight.current[nextKey] = true;
      setLoadingKey(nextKey);
      try {
        const res = await fetch("/api/nuclear/explain", {
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
    const scrollbarWidth = window.innerWidth - document.documentElement.clientWidth;
    document.addEventListener("keydown", onKey);
    if (scrollbarWidth > 0) {
      document.body.style.paddingRight = `${scrollbarWidth}px`;
    }
    document.body.style.overflow = "hidden";
    return () => {
      document.removeEventListener("keydown", onKey);
      document.body.style.overflow = "";
      document.body.style.paddingRight = "";
      prevActive?.focus?.();
    };
  }, [selection, onClose]);

  if (!selection) return null;

  const title = itemTitle(selection);
  const subtitle = itemSubtitle(selection);
  const status = selection.item.status;
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
        aria-labelledby="nuclear-explain-title"
        className="drawer-panel absolute right-0 top-0 flex h-full w-full max-w-[460px] flex-col border-l border-[var(--border)] bg-[var(--bg-surface)]"
      >
        <header className="flex items-start justify-between gap-4 border-b border-[var(--border)] px-5 py-4">
          <div className="min-w-0">
            <p className="text-[11px] font-semibold uppercase tracking-[0.12em] text-[var(--text-tertiary)]">
              Nuclear dashboard explanation
            </p>
            <h3
              id="nuclear-explain-title"
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
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth="1.75"
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

function itemTitle(selection: NuclearExplainable): string {
  if (selection.type === "player") return selection.item.name;
  return selection.item.label;
}

function itemSubtitle(selection: NuclearExplainable): string {
  if (selection.type === "player") {
    const value = selection.item.marketCap ?? selection.item.projectScale;
    const label =
      selection.item.kind === "public" ? "market cap" : "project scale";
    return `${selection.item.category} · ${label} ${value}`;
  }
  if (selection.type === "metric") {
    return `${selection.item.value} · ${selection.item.context}`;
  }
  return `${selection.item.track} · watch next`;
}
