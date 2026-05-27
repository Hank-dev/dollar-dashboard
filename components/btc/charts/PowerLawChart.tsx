"use client";

import { useEffect, useMemo, useRef } from "react";
import {
  createChart,
  LineSeries,
  HistogramSeries,
  type IChartApi,
  type UTCTimestamp,
  PriceScaleMode,
  LineStyle,
} from "lightweight-charts";
import type { DailyPoint, PriceHistoryResponse } from "@/lib/btc/types";
import type { Timeframe } from "./timeframe";
import { timeframeCutoff } from "./timeframe";

export function PowerLawChart({
  data,
  timeframe,
}: {
  data: PriceHistoryResponse;
  timeframe: Timeframe;
}) {
  const ref = useRef<HTMLDivElement | null>(null);
  const chartRef = useRef<IChartApi | null>(null);

  const filtered = useMemo(() => {
    const cutoff = timeframeCutoff(timeframe);
    const slice = (arr: DailyPoint[]) =>
      cutoff === null ? arr : arr.filter((p) => p.t >= cutoff);
    return {
      series: slice(data.series),
      fit: slice(data.fit),
      upper1: slice(data.upper1),
      lower1: slice(data.lower1),
      upper2: slice(data.upper2),
      lower2: slice(data.lower2),
      sma200: slice(data.sma200),
      volume: slice(data.volume ?? []),
    };
  }, [data, timeframe]);

  useEffect(() => {
    if (!ref.current) return;
    const el = ref.current;

    const chart = createChart(el, {
      width: el.clientWidth,
      height: el.clientHeight,
      autoSize: true,
      layout: {
        background: { color: "transparent" },
        textColor: "#8b929c",
        fontFamily: "var(--font-jetbrains-mono), monospace",
        fontSize: 11,
      },
      grid: {
        vertLines: { color: "#1a1d23" },
        horzLines: { color: "#1a1d23" },
      },
      rightPriceScale: {
        mode: PriceScaleMode.Logarithmic,
        borderColor: "#2a2f38",
        scaleMargins: { top: 0.05, bottom: 0.22 },
      },
      timeScale: {
        borderColor: "#2a2f38",
        timeVisible: false,
        secondsVisible: false,
      },
      crosshair: {
        vertLine: { color: "#06b6d4", width: 1, style: LineStyle.Dotted },
        horzLine: { color: "#06b6d4", width: 1, style: LineStyle.Dotted },
      },
    });
    chartRef.current = chart;

    const toLW = (arr: { t: number; v: number }[]) =>
      arr
        .filter((p) => Number.isFinite(p.v) && p.v > 0)
        .map((p) => ({ time: p.t as UTCTimestamp, value: p.v }));

    const upper2 = chart.addSeries(LineSeries, {
      color: "#ef4444",
      lineWidth: 1,
      lineStyle: LineStyle.Dashed,
      priceLineVisible: false,
      lastValueVisible: false,
      crosshairMarkerVisible: false,
    });
    upper2.setData(toLW(filtered.upper2));

    const upper1 = chart.addSeries(LineSeries, {
      color: "#f59e0b",
      lineWidth: 1,
      lineStyle: LineStyle.Dotted,
      priceLineVisible: false,
      lastValueVisible: false,
      crosshairMarkerVisible: false,
    });
    upper1.setData(toLW(filtered.upper1));

    const fit = chart.addSeries(LineSeries, {
      color: "#3b82f6",
      lineWidth: 2,
      priceLineVisible: false,
      lastValueVisible: false,
    });
    fit.setData(toLW(filtered.fit));

    const lower1 = chart.addSeries(LineSeries, {
      color: "#f59e0b",
      lineWidth: 1,
      lineStyle: LineStyle.Dotted,
      priceLineVisible: false,
      lastValueVisible: false,
      crosshairMarkerVisible: false,
    });
    lower1.setData(toLW(filtered.lower1));

    const lower2 = chart.addSeries(LineSeries, {
      color: "#22c55e",
      lineWidth: 1,
      lineStyle: LineStyle.Dashed,
      priceLineVisible: false,
      lastValueVisible: false,
      crosshairMarkerVisible: false,
    });
    lower2.setData(toLW(filtered.lower2));

    const sma200 = chart.addSeries(LineSeries, {
      color: "#a3a3a3",
      lineWidth: 1,
      priceLineVisible: false,
      lastValueVisible: false,
      crosshairMarkerVisible: false,
    });
    sma200.setData(toLW(filtered.sma200));

    const price = chart.addSeries(LineSeries, {
      color: "#e4e7eb",
      lineWidth: 2,
      priceFormat: { type: "price", precision: 2, minMove: 0.01 },
    });
    price.setData(toLW(filtered.series));

    // Volume sub-pane: histogram on its own overlay scale, pinned to the
    // bottom ~18% of the chart.
    const volume = chart.addSeries(HistogramSeries, {
      priceScaleId: "volume",
      priceFormat: { type: "volume" },
      priceLineVisible: false,
      lastValueVisible: false,
    });
    chart.priceScale("volume").applyOptions({
      scaleMargins: { top: 0.82, bottom: 0 },
    });
    volume.setData(
      filtered.volume
        .filter((p) => Number.isFinite(p.v) && p.v > 0)
        .map((p) => ({
          time: p.t as UTCTimestamp,
          value: p.v,
          color: "rgba(139, 146, 156, 0.45)",
        })),
    );

    chart.timeScale().fitContent();

    return () => {
      chart.remove();
      chartRef.current = null;
    };
  }, [filtered]);

  return (
    <div className="flex flex-col h-full">
      <div ref={ref} className="flex-1 min-h-0" />
      <Legend />
    </div>
  );
}

function Legend() {
  return (
    <div className="flex flex-wrap gap-x-4 gap-y-1 px-3 py-1 border-t border-[var(--border)] mono text-[10px] text-[var(--text-secondary)]">
      <Swatch color="#e4e7eb" label="BTC price" />
      <Swatch color="#3b82f6" label="Power-law fit" />
      <Swatch color="#f59e0b" label="±1σ" />
      <Swatch color="#ef4444" label="+2σ (top)" />
      <Swatch color="#22c55e" label="-2σ (bottom)" />
      <Swatch color="#a3a3a3" label="200-day SMA" />
      <Swatch color="#8b929c" label="Volume (Binance, USDT)" />
    </div>
  );
}

function Swatch({ color, label }: { color: string; label: string }) {
  return (
    <span className="inline-flex items-center gap-1.5">
      <span
        className="inline-block w-3 h-[2px]"
        style={{ background: color }}
      />
      {label}
    </span>
  );
}
