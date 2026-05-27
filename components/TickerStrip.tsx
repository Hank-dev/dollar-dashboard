"use client";

import { useEffect, useRef, useState } from "react";

interface Tick {
  sym: string;
  value: number;
  decimals: number;
  suffix?: string;
  change?: string;
  changeDir?: "up" | "down" | "dim";
}

const TICKS: Tick[] = [
  { sym: "BTC", value: 0, decimals: 2, change: "", changeDir: "up" },
  { sym: "ETH", value: 0, decimals: 2, change: "", changeDir: "up" },
  { sym: "DXY", value: 0, decimals: 2, change: "", changeDir: "down" },
  { sym: "US10Y", value: 4.34, decimals: 2, suffix: "%", change: "+3 bp", changeDir: "up" },
  { sym: "VIX", value: 14.82, decimals: 2, change: "", changeDir: "down" },
  { sym: "GOLD", value: 2684, decimals: 0, change: "", changeDir: "up" },
  { sym: "SPX", value: 5902.71, decimals: 2, change: "", changeDir: "up" },
  { sym: "FNG", value: 71, decimals: 0, change: "greed", changeDir: "dim" },
];

export function TickerStrip() {
  const [prices, setPrices] = useState<Record<string, number>>({});
  const wsRef = useRef<WebSocket | null>(null);

  useEffect(() => {
    const ws = new WebSocket("wss://stream.binance.com:9443/ws/btcusdt@trade/ethusdt@trade");
    wsRef.current = ws;
    ws.onmessage = (e) => {
      try {
        const msg = JSON.parse(e.data) as { s: string; p: string };
        const px = parseFloat(msg.p);
        if (!Number.isFinite(px)) return;
        const sym = msg.s === "BTCUSDT" ? "BTC" : msg.s === "ETHUSDT" ? "ETH" : null;
        if (sym) setPrices((prev) => ({ ...prev, [sym]: px }));
      } catch {}
    };
    return () => ws.close();
  }, []);

  return (
    <div className="flex h-8 border-b border-[var(--border)] bg-[var(--bg-base)] mono text-[11.5px] overflow-hidden items-stretch">
      {TICKS.map((t) => {
        const live = prices[t.sym];
        const val = live ?? t.value;
        const display = val
          ? t.suffix
            ? val.toFixed(t.decimals) + t.suffix
            : val.toLocaleString("en-US", { minimumFractionDigits: t.decimals, maximumFractionDigits: t.decimals })
          : "—";

        return (
          <div
            key={t.sym}
            className="flex items-center gap-2 px-3.5 border-r border-[var(--border)] whitespace-nowrap shrink-0"
          >
            <span className="text-[var(--text-tertiary)] tracking-[0.06em]">{t.sym}</span>
            <span className="text-[var(--text-primary)] font-medium">{display}</span>
            {t.change && (
              <span
                className={`text-[10.5px] ${
                  t.changeDir === "up"
                    ? "text-[var(--accent-green)]"
                    : t.changeDir === "down"
                    ? "text-[var(--accent-red)]"
                    : "text-[var(--text-tertiary)]"
                }`}
              >
                {t.change}
              </span>
            )}
          </div>
        );
      })}
    </div>
  );
}
