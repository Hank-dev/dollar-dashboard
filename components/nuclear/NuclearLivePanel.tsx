"use client";

import { useEffect, useMemo, useState } from "react";
import type {
  NuclearUraniumIndex,
  NuclearMarketLiveResponse,
  NuclearQuote,
} from "@/lib/nuclearLive";
import type { XSummaryError, XSummaryResponse } from "@/lib/xSummary";

const MONEY_COMPACT = new Intl.NumberFormat("en-US", {
  style: "currency",
  currency: "USD",
  notation: "compact",
  maximumFractionDigits: 1,
});

const MONEY_PRICE = new Intl.NumberFormat("en-US", {
  style: "currency",
  currency: "USD",
  maximumFractionDigits: 2,
});

const DATE_TIME = new Intl.DateTimeFormat("en-GB", {
  day: "numeric",
  month: "short",
  hour: "2-digit",
  minute: "2-digit",
});

const DATE_MONTH = new Intl.DateTimeFormat("en-GB", {
  month: "short",
  year: "2-digit",
});

export default function NuclearLivePanel() {
  const [market, setMarket] = useState<NuclearMarketLiveResponse | null>(null);
  const [marketError, setMarketError] = useState<string | null>(null);
  const [summary, setSummary] = useState<XSummaryResponse | null>(null);
  const [summaryError, setSummaryError] = useState<XSummaryError | null>(null);
  const [loadingMarket, setLoadingMarket] = useState(true);
  const [loadingSummary, setLoadingSummary] = useState(true);
  const [refreshKey, setRefreshKey] = useState(0);

  useEffect(() => {
    const controller = new AbortController();

    async function loadMarket() {
      setLoadingMarket(true);
      setMarketError(null);
      try {
        const res = await fetch("/api/nuclear/market", {
          cache: "no-store",
          signal: controller.signal,
        });
        const body = await res.json();
        if (!res.ok) {
          throw new Error(body.error || `Request failed (${res.status})`);
        }
        setMarket(body as NuclearMarketLiveResponse);
      } catch (err) {
        if ((err as { name?: string }).name === "AbortError") return;
        setMarket(null);
        setMarketError(
          err instanceof Error ? err.message : "Could not load market data.",
        );
      } finally {
        setLoadingMarket(false);
      }
    }

    async function loadSummary() {
      setLoadingSummary(true);
      setSummaryError(null);
      try {
        const res = await fetch("/api/nuclear/x-summary", {
          cache: "no-store",
          signal: controller.signal,
        });
        const body = (await res.json()) as XSummaryResponse | XSummaryError;
        if (!res.ok) {
          setSummary(null);
          setSummaryError(body as XSummaryError);
          return;
        }
        setSummary(body as XSummaryResponse);
      } catch (err) {
        if ((err as { name?: string }).name === "AbortError") return;
        setSummary(null);
        setSummaryError({
          error:
            err instanceof Error ? err.message : "Could not load Grok summary.",
        });
      } finally {
        setLoadingSummary(false);
      }
    }

    loadMarket();
    loadSummary();

    return () => controller.abort();
  }, [refreshKey]);

  const publicMovers = useMemo(() => {
    const quotes = market?.publicEquities.filter(
      (quote) => quote.changePercent != null,
    );
    if (!quotes || quotes.length === 0) return { best: null, worst: null };
    const sorted = [...quotes].sort(
      (a, b) => (b.changePercent ?? 0) - (a.changePercent ?? 0),
    );
    return { best: sorted[0], worst: sorted[sorted.length - 1] };
  }, [market]);

  const primaryProxy = market?.uraniumProxies[0] ?? null;
  const checkedLabel = market ? DATE_TIME.format(new Date(market.checkedAt)) : null;
  const nextSummaryLabel = summary
    ? DATE_TIME.format(new Date(summary.nextRefreshAt))
    : null;
  const summarySections = useMemo(
    () => (summary ? parseSummarySections(summary.summary) : []),
    [summary],
  );
  const loading = loadingMarket || loadingSummary;

  return (
    <div
      className="border border-[var(--border)] bg-[var(--bg-surface)] p-4 sm:p-5"
      style={{ borderRadius: 8 }}
    >
      <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
        <div>
          <h3 className="text-[14px] font-medium text-[var(--text-primary)]">
            Live nuclear market check
          </h3>
          <p className="mt-1 max-w-[760px] text-[11.5px] leading-relaxed text-[var(--text-tertiary)]">
            Quotes refresh hourly from Yahoo Finance. Grok searches recent X
            posts twice per day. Uranium is shown through equity ETF proxies,
            not physical spot pricing.
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

      <div className="mt-4 grid gap-3 lg:grid-cols-[0.95fr_1.05fr]">
        <section
          className="border border-[var(--border)] bg-[var(--bg-base)] p-3"
          style={{ borderRadius: 6 }}
        >
          <div className="flex items-center justify-between gap-3">
            <h4 className="mono text-[10.5px] font-medium uppercase tracking-[0.12em] text-[var(--accent-cyan)]">
              Market tape
            </h4>
            {market && (
              <span className="mono max-w-[220px] text-right text-[10px] uppercase tracking-wide text-[var(--text-tertiary)]">
                {market.source} · {market.refreshCadenceSeconds / 60}m cache
              </span>
            )}
          </div>

          {loadingMarket && <SkeletonLines label="Loading market data..." />}

          {!loadingMarket && marketError && (
            <p className="mt-3 text-[12.5px] leading-relaxed text-[var(--text-primary)]">
              {marketError}
            </p>
          )}

          {!loadingMarket && market && (
            <>
              <div className="mt-3 grid gap-2 sm:grid-cols-3">
                <LiveStat
                  label="Tracked public cap"
                  value={formatMoneyCompact(market.totalPublicMarketCapUsd)}
                />
                <LiveStat
                  label="Best mover"
                  value={formatMover(publicMovers.best)}
                  tone={toneFor(publicMovers.best?.changePercent)}
                />
                <LiveStat
                  label="Uranium proxy"
                  value={formatMover(primaryProxy)}
                  tone={toneFor(primaryProxy?.changePercent)}
                />
              </div>

              {market.uraniumIndex && (
                <UraniumIndexChart index={market.uraniumIndex} />
              )}

              {market.errors.length > 0 && (
                <p className="mt-3 border border-[var(--border)] bg-[var(--bg-surface)] px-3 py-2 text-[11.5px] leading-relaxed text-[var(--text-tertiary)]">
                  Live quote source warning: {market.errors.join("; ")}
                </p>
              )}

              {market.publicEquities.length > 0 ? (
                <div className="mt-4 grid gap-2">
                  {market.publicEquities.map((quote) => (
                    <QuoteRow key={quote.symbol} quote={quote} showMarketCap />
                  ))}
                </div>
              ) : (
                <p className="mt-4 text-[12.5px] leading-relaxed text-[var(--text-secondary)]">
                  No live public-equity quotes were returned. The snapshot below
                  remains available.
                </p>
              )}

              <div className="mt-4 border-t border-[var(--border)] pt-3">
                <p className="mono text-[10px] uppercase tracking-[0.12em] text-[var(--text-tertiary)]">
                  Uranium and nuclear equity proxies
                </p>
                {market.uraniumProxies.length > 0 ? (
                  <div className="mt-2 grid gap-2">
                    {market.uraniumProxies.map((quote) => (
                      <QuoteRow key={quote.symbol} quote={quote} />
                    ))}
                  </div>
                ) : (
                  <p className="mt-2 text-[12.5px] leading-relaxed text-[var(--text-secondary)]">
                    No uranium proxy quotes were returned.
                  </p>
                )}
              </div>

              <p className="mt-3 text-[11px] leading-relaxed text-[var(--text-tertiary)]">
                {market.note}
              </p>
            </>
          )}
        </section>

        <section
          className="border border-[var(--border)] bg-[var(--bg-base)] p-3"
          style={{ borderRadius: 6 }}
        >
          <div className="flex items-center justify-between gap-3">
            <h4 className="mono text-[10.5px] font-medium uppercase tracking-[0.12em] text-[var(--accent-cyan)]">
              Grok narrative check
            </h4>
            {summary && (
              <span className="mono text-[10px] uppercase tracking-wide text-[var(--text-tertiary)]">
                {summary.cacheStatus} · next {nextSummaryLabel ?? "scheduled"}
              </span>
            )}
          </div>

          {loadingSummary && <SkeletonLines label="Loading Grok summary..." />}

          {!loadingSummary && summaryError && (
            <div className="mt-3 text-[12.5px] leading-relaxed text-[var(--text-primary)]">
              <p>{summaryError.error}</p>
              {summaryError.setup && (
                <p className="mt-2 text-[var(--text-tertiary)]">
                  {summaryError.setup}
                </p>
              )}
            </div>
          )}

          {!loadingSummary && summary && (
            <>
              <div className="mt-3 grid gap-3">
                {summarySections.map((section) => (
                  <section key={section.title}>
                    <h5 className="mono text-[10px] font-medium uppercase tracking-[0.12em] text-[var(--text-tertiary)]">
                      {section.title}
                    </h5>
                    <ul className="mt-1.5 space-y-2 text-[12.5px] leading-relaxed text-[var(--text-primary)]">
                      {section.items.map((item) => (
                        <li key={item} className="flex gap-2">
                          <span className="mt-[0.65em] h-1 w-1 shrink-0 rounded-full bg-[var(--accent-cyan)]" />
                          <span>{item}</span>
                        </li>
                      ))}
                    </ul>
                  </section>
                ))}
              </div>

              {summary.citations.length > 0 && (
                <div className="mt-4 border-t border-[var(--border)] pt-3">
                  <p className="mono text-[10px] uppercase tracking-[0.12em] text-[var(--text-tertiary)]">
                    Sources
                  </p>
                  <div className="mt-2 flex flex-wrap gap-2">
                    {summary.citations.slice(0, 6).map((citation, index) =>
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
        </section>
      </div>
    </div>
  );
}

function LiveStat({
  label,
  value,
  tone,
}: {
  label: string;
  value: string;
  tone?: "up" | "down";
}) {
  return (
    <div
      className="border border-[var(--border)] bg-[var(--bg-surface)] px-3 py-2"
      style={{ borderRadius: 6 }}
    >
      <p className="mono text-[9.5px] uppercase tracking-[0.12em] text-[var(--text-tertiary)]">
        {label}
      </p>
      <p
        className={`mt-1 text-[12.5px] font-medium tabular-nums ${
          tone === "up"
            ? "text-[var(--accent-green)]"
            : tone === "down"
            ? "text-[var(--accent-red)]"
            : "text-[var(--text-primary)]"
        }`}
      >
        {value}
      </p>
    </div>
  );
}

function QuoteRow({
  quote,
  showMarketCap = false,
}: {
  quote: NuclearQuote;
  showMarketCap?: boolean;
}) {
  const tone = toneFor(quote.changePercent);
  return (
    <a
      href={quote.sourceUrl}
      target="_blank"
      rel="noreferrer"
      className="grid grid-cols-[58px_1fr_auto] items-center gap-2 border border-[var(--border)] bg-[var(--bg-surface)] px-3 py-2 text-inherit no-underline hover:bg-[var(--bg-surface-hover)]"
      style={{ borderRadius: 6 }}
    >
      <div>
        <p className="mono text-[12px] font-medium text-[var(--text-primary)]">
          {quote.symbol}
        </p>
        <p className="mono mt-0.5 text-[9px] uppercase tracking-[0.08em] text-[var(--text-tertiary)]">
          {quote.currency}
        </p>
      </div>
      <div className="min-w-0">
        <p className="truncate text-[12px] text-[var(--text-secondary)]">
          {quote.name}
        </p>
        {showMarketCap && (
          <p className="mt-0.5 text-[10.5px] text-[var(--text-tertiary)]">
            Cap {formatMoneyCompact(quote.marketCapUsd)}
            {quote.marketCapSource === "snapshot" ? " snapshot" : ""}
          </p>
        )}
      </div>
      <div className="text-right">
        <p className="mono text-[12px] tabular-nums text-[var(--text-primary)]">
          {quote.price == null ? "—" : MONEY_PRICE.format(quote.price)}
        </p>
        <p
          className={`mono mt-0.5 text-[11px] tabular-nums ${
            tone === "up"
              ? "text-[var(--accent-green)]"
              : tone === "down"
              ? "text-[var(--accent-red)]"
              : "text-[var(--text-tertiary)]"
          }`}
        >
          {formatPercentPoints(quote.changePercent)}
        </p>
      </div>
    </a>
  );
}

function UraniumIndexChart({ index }: { index: NuclearUraniumIndex }) {
  const points = index.points;
  const latestLabel =
    index.latest == null ? "—" : `$${index.latest.toFixed(2)}/${index.unit.replace("USD/", "")}`;
  const pathData = linePath(points, 640, 178, 18);
  const areaData = areaPath(points, 640, 178, 18);
  const min = points.length ? Math.min(...points.map((point) => point.value)) : null;
  const max = points.length ? Math.max(...points.map((point) => point.value)) : null;
  const first = points[0]?.date;
  const last = points.at(-1)?.date;
  const changeTone = toneFor(index.change12mPercent);

  return (
    <div
      className="mt-4 border border-[var(--border)] bg-[var(--bg-surface)] p-3"
      style={{ borderRadius: 6 }}
    >
      <div className="flex flex-col gap-2 sm:flex-row sm:items-start sm:justify-between">
        <div>
          <h5 className="mono text-[10px] font-medium uppercase tracking-[0.12em] text-[var(--text-tertiary)]">
            Uranium index
          </h5>
          <p className="mt-1 text-[12.5px] text-[var(--text-primary)]">
            {index.label} · {index.source}
          </p>
        </div>
        <div className="text-left sm:text-right">
          <p className="mono text-[16px] font-medium tabular-nums text-[var(--text-primary)]">
            {latestLabel}
          </p>
          <p
            className={`mono mt-0.5 text-[11px] tabular-nums ${
              changeTone === "up"
                ? "text-[var(--accent-green)]"
                : changeTone === "down"
                ? "text-[var(--accent-red)]"
                : "text-[var(--text-tertiary)]"
            }`}
          >
            {formatPercentPoints(index.change12mPercent)} · 12m
          </p>
          <p className="mono mt-0.5 text-[10px] uppercase tracking-wide text-[var(--text-tertiary)]">
            As of {index.asOf ? formatDateMonth(index.asOf) : "—"}
          </p>
        </div>
      </div>

      {points.length > 1 ? (
        <div className="mt-3">
          <svg
            className="h-[178px] w-full overflow-visible"
            viewBox="0 0 640 178"
            preserveAspectRatio="none"
            role="img"
            aria-label={`${index.label} chart`}
          >
            <line
              x1="0"
              x2="640"
              y1="18"
              y2="18"
              stroke="var(--border)"
              strokeWidth="1"
            />
            <line
              x1="0"
              x2="640"
              y1="160"
              y2="160"
              stroke="var(--border)"
              strokeWidth="1"
            />
            {areaData && (
              <path
                d={areaData}
                fill="var(--accent-cyan)"
                opacity="0.08"
              />
            )}
            {pathData && (
              <path
                d={pathData}
                fill="none"
                stroke="var(--accent-cyan)"
                strokeWidth="2"
                vectorEffect="non-scaling-stroke"
              />
            )}
          </svg>
          <div className="mt-1 flex items-center justify-between text-[10.5px] text-[var(--text-tertiary)]">
            <span>{first ? formatDateMonth(first) : "—"}</span>
            <span>
              Range {min == null ? "—" : `$${min.toFixed(0)}`}-
              {max == null ? "—" : `$${max.toFixed(0)}`}
            </span>
            <span>{last ? formatDateMonth(last) : "—"}</span>
          </div>
        </div>
      ) : (
        <p className="mt-3 text-[12.5px] text-[var(--text-secondary)]">
          Uranium index history is unavailable.
        </p>
      )}

      <p className="mt-3 text-[11px] leading-relaxed text-[var(--text-tertiary)]">
        {index.note}{" "}
        <a
          href={index.sourceUrl}
          target="_blank"
          rel="noreferrer"
          className="underline decoration-[var(--border-strong)] underline-offset-2 hover:text-[var(--text-secondary)]"
        >
          Source
        </a>
      </p>
    </div>
  );
}

function SkeletonLines({ label }: { label: string }) {
  return (
    <div className="mt-3 space-y-2.5" aria-live="polite" aria-busy="true">
      <div className="skeleton-line h-3 w-full" />
      <div className="skeleton-line h-3 w-[92%]" />
      <div className="skeleton-line h-3 w-[78%]" />
      <span className="sr-only">{label}</span>
    </div>
  );
}

function linePath(
  points: { value: number }[],
  width: number,
  height: number,
  pad: number,
): string {
  if (points.length < 2) return "";
  const min = Math.min(...points.map((point) => point.value));
  const max = Math.max(...points.map((point) => point.value));
  const range = max - min || 1;
  return points
    .map((point, index) => {
      const x = (index / (points.length - 1)) * width;
      const y = pad + (1 - (point.value - min) / range) * (height - pad * 2);
      return `${index === 0 ? "M" : "L"}${x.toFixed(1)},${y.toFixed(1)}`;
    })
    .join(" ");
}

function areaPath(
  points: { value: number }[],
  width: number,
  height: number,
  pad: number,
): string {
  const line = linePath(points, width, height, pad);
  return line ? `${line} L${width},${height - pad} L0,${height - pad} Z` : "";
}

function formatMoneyCompact(value: number | null | undefined): string {
  return value == null ? "—" : MONEY_COMPACT.format(value);
}

function formatMover(quote: NuclearQuote | null | undefined): string {
  if (!quote) return "—";
  return `${quote.symbol} ${formatPercentPoints(quote.changePercent)}`;
}

function formatPercentPoints(value: number | null | undefined): string {
  if (value == null) return "—";
  const sign = value >= 0 ? "+" : "";
  return `${sign}${value.toFixed(1)}%`;
}

function toneFor(value: number | null | undefined): "up" | "down" | undefined {
  if (value == null || value === 0) return undefined;
  return value > 0 ? "up" : "down";
}

function formatDateMonth(date: string): string {
  return DATE_MONTH.format(new Date(date + "T00:00:00Z"));
}

function parseSummarySections(summary: string): { title: string; items: string[] }[] {
  const sections: { title: string; items: string[] }[] = [];
  let current: { title: string; items: string[] } | null = null;

  for (const rawLine of summary.split("\n")) {
    const line = rawLine.trim();
    if (!line) continue;

    const heading = parseHeading(line);
    if (heading) {
      current = { title: heading, items: [] };
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

  const populated = sections.filter((section) => section.items.length > 0);
  return populated.length > 0
    ? populated
    : [{ title: "Summary", items: [cleanSummaryText(summary)] }];
}

function parseHeading(line: string): string | null {
  const cleaned = line
    .replace(/^#{1,3}\s*/, "")
    .replace(/\*\*/g, "")
    .replace(/:$/, "")
    .trim();
  return /^(top nuclear narratives|what changed|what to watch|summary)$/i.test(
    cleaned,
  )
    ? cleaned
    : null;
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
