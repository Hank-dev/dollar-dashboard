"use client";

import { useEffect, useRef, useState } from "react";
import useSWR from "swr";
import { fmtPrice, fmtPct } from "@/lib/btc/format";
import type { PriceHistoryResponse } from "@/lib/btc/types";

const fetcher = (u: string) =>
  fetch(u).then((r) => {
    if (!r.ok) throw new Error(`${r.status}`);
    return r.json();
  });

export function HeaderBar() {
  const { data: history } = useSWR<PriceHistoryResponse>(
    "/api/price-history",
    fetcher,
    { refreshInterval: 60 * 60 * 1000 },
  );

  const [livePrice, setLivePrice] = useState<number | null>(null);
  const [flash, setFlash] = useState<"up" | "down" | null>(null);
  const lastRef = useRef<number | null>(null);

  useEffect(() => {
    const ws = new WebSocket(
      "wss://stream.binance.com:9443/ws/btcusdt@trade",
    );
    let alive = true;
    ws.onmessage = (e) => {
      if (!alive) return;
      try {
        const msg = JSON.parse(e.data) as { p: string };
        const px = parseFloat(msg.p);
        if (!Number.isFinite(px)) return;
        setLivePrice((prev) => {
          if (prev != null) {
            if (px > prev) setFlash("up");
            else if (px < prev) setFlash("down");
            window.setTimeout(() => setFlash(null), 200);
          }
          lastRef.current = px;
          return px;
        });
      } catch {
        /* ignore */
      }
    };
    ws.onerror = () => {
      /* SWR fallback covers static price */
    };
    return () => {
      alive = false;
      ws.close();
    };
  }, []);

  const yesterday = history?.series.at(-2)?.v ?? null;
  const today = livePrice ?? history?.series.at(-1)?.v ?? null;
  const change24h =
    today != null && yesterday ? ((today - yesterday) / yesterday) * 100 : null;
  const ath = history?.ath;
  const fromAth =
    today != null && ath ? ((today - ath.v) / ath.v) * 100 : null;

  const flashCls =
    flash === "up" ? "flash-up" : flash === "down" ? "flash-down" : "";

  return (
    <header className="border-b border-[var(--border-strong)] bg-[var(--bg-panel)] overflow-x-auto">
      <div className="flex items-stretch min-w-0">
        <Cell label="BTC / USD">
          <span className={`mono text-xl sm:text-3xl font-medium tnum ${flashCls}`}>
            {fmtPrice(today)}
          </span>
        </Cell>
        <Cell label="24h">
          <span
            className={`mono text-base sm:text-lg tnum ${
              change24h != null && change24h >= 0
                ? "text-[var(--accent-green)]"
                : "text-[var(--accent-red)]"
            }`}
          >
            {fmtPct(change24h)}
          </span>
        </Cell>
        <Cell label="ATH" className="hidden sm:flex">
          <span className="mono text-lg tnum">{fmtPrice(ath?.v)}</span>
          <span className="mono text-[11px] text-[var(--text-tertiary)]">
            {fromAth != null ? `${fmtPct(fromAth, 1)} from ATH` : ""}
          </span>
        </Cell>
        <Cell label="DAYS SINCE ATH" className="hidden md:flex">
          <span className="mono text-lg tnum">
            {history?.daysSinceAth ?? "—"}
          </span>
        </Cell>
        <Cell label="CYCLE DAY" className="hidden md:flex">
          <span className="mono text-lg tnum">
            {history?.daysSinceLastHalving ?? "—"}
          </span>
          <span className="mono text-[11px] text-[var(--text-tertiary)]">
            since 2024-04-20
          </span>
        </Cell>
        <Cell label="UTC" className="ml-auto hidden sm:flex">
          <UtcClock />
        </Cell>
      </div>
    </header>
  );
}

function Cell({
  label,
  children,
  className,
}: {
  label: string;
  children: React.ReactNode;
  className?: string;
}) {
  return (
    <div
      className={`shrink-0 px-3 sm:px-4 py-2 flex flex-col justify-center border-r border-[var(--border)] last:border-r-0 ${className ?? ""}`}
    >
      <div className="mono text-[10px] tracking-widest text-[var(--text-tertiary)] uppercase">
        {label}
      </div>
      <div className="flex flex-col">{children}</div>
    </div>
  );
}

function UtcClock() {
  const [now, setNow] = useState<Date | null>(null);
  useEffect(() => {
    setNow(new Date());
    const id = setInterval(() => setNow(new Date()), 1000);
    return () => clearInterval(id);
  }, []);
  const hh = now ? String(now.getUTCHours()).padStart(2, "0") : "--";
  const mm = now ? String(now.getUTCMinutes()).padStart(2, "0") : "--";
  const ss = now ? String(now.getUTCSeconds()).padStart(2, "0") : "--";
  const y = now ? now.getUTCFullYear() : "----";
  const m = now ? String(now.getUTCMonth() + 1).padStart(2, "0") : "--";
  const d = now ? String(now.getUTCDate()).padStart(2, "0") : "--";
  return (
    <>
      <span className="mono text-lg tnum" suppressHydrationWarning>
        {`${hh}:${mm}:${ss}`}
      </span>
      <span
        className="mono text-[11px] text-[var(--text-tertiary)]"
        suppressHydrationWarning
      >
        {`${y}-${m}-${d}`}
      </span>
    </>
  );
}
