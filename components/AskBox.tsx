"use client";

import { useRef, useState } from "react";

export default function AskBox({
  endpoint = "/api/ask",
  placeholder = "e.g. Why does a steeper curve worry the bond market?",
  snapshotNote = "grounded in the 22 May 2026 snapshot",
}: {
  endpoint?: string;
  placeholder?: string;
  snapshotNote?: string;
}) {
  const [question, setQuestion] = useState("");
  const [answer, setAnswer] = useState("");
  const [streaming, setStreaming] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const abortRef = useRef<AbortController | null>(null);

  async function submit(e: React.FormEvent) {
    e.preventDefault();
    const q = question.trim();
    if (!q || streaming) return;

    setAnswer("");
    setError(null);
    setStreaming(true);
    const controller = new AbortController();
    abortRef.current = controller;

    try {
      const res = await fetch(endpoint, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ question: q }),
        signal: controller.signal,
      });
      if (!res.ok || !res.body) {
        const text = await res.text().catch(() => "");
        throw new Error(text || `Request failed (${res.status})`);
      }
      const reader = res.body.getReader();
      const dec = new TextDecoder();
      for (;;) {
        const { done, value } = await reader.read();
        if (done) break;
        setAnswer((a) => a + dec.decode(value, { stream: true }));
      }
    } catch (err) {
      if ((err as { name?: string }).name === "AbortError") return;
      setError(
        err instanceof Error ? err.message : "Could not get an answer.",
      );
    } finally {
      setStreaming(false);
      abortRef.current = null;
    }
  }

  function stop() {
    abortRef.current?.abort();
  }

  return (
    <div className="border border-[var(--border)] bg-[var(--bg-surface)] p-4" style={{ borderRadius: 8 }}>
      <form onSubmit={submit} className="flex flex-col gap-3">
        <label htmlFor="ask-input" className="sr-only">
          Ask a question about the dashboard
        </label>
        <textarea
          id="ask-input"
          value={question}
          onChange={(e) => setQuestion(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) {
              e.preventDefault();
              (e.currentTarget.form as HTMLFormElement)?.requestSubmit();
            }
          }}
          rows={3}
          placeholder={placeholder}
          className="w-full resize-y border border-[var(--border)] bg-[var(--bg-base)] px-3 py-2 text-[13.5px] text-[var(--text-primary)] placeholder:text-[var(--text-tertiary)] focus:border-[var(--border-strong)] focus:outline-none"
          style={{ borderRadius: 6 }}
          disabled={streaming}
        />
        <div className="flex items-center justify-between gap-3">
          <p className="text-[11px] text-[var(--text-tertiary)]">
            Cmd/Ctrl + Enter to send · {snapshotNote}
          </p>
          <div className="flex items-center gap-2">
            {streaming && (
              <button
                type="button"
                onClick={stop}
                className="border border-[var(--border-strong)] px-3 py-1.5 text-[12px] text-[var(--text-primary)] hover:bg-[var(--bg-surface-hover)] focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--text-secondary)]"
                style={{ borderRadius: 6 }}
              >
                Stop
              </button>
            )}
            <button
              type="submit"
              disabled={streaming || question.trim().length === 0}
              className="border border-[var(--border-strong)] bg-[var(--text-primary)] px-3.5 py-1.5 text-[12px] font-medium text-[var(--bg-surface)] disabled:opacity-50 focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--text-secondary)]"
              style={{ borderRadius: 6 }}
            >
              {streaming ? "Answering..." : "Ask"}
            </button>
          </div>
        </div>
      </form>

      {(answer || streaming || error) && (
        <div
          className="mt-4 border-t border-[var(--border)] pt-4"
          aria-live="polite"
          aria-busy={streaming}
        >
          {error ? (
            <p className="text-[13px] text-[var(--text-primary)]">{error}</p>
          ) : (
            <p className="whitespace-pre-line text-[13.5px] leading-relaxed text-[var(--text-primary)]">
              {answer}
              {streaming && <span className="ml-0.5 inline-block animate-pulse">|</span>}
            </p>
          )}
        </div>
      )}
    </div>
  );
}
