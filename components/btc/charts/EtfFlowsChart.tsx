"use client";

import { useEffect, useRef } from "react";
import {
  createChart,
  HistogramSeries,
  LineSeries,
  type IChartApi,
  type UTCTimestamp,
  LineStyle,
} from "lightweight-charts";
import type { EtfFlowsResponse } from "@/lib/btc/types";

export function EtfFlowsChart({ data }: { data: EtfFlowsResponse }) {
  const ref = useRef<HTMLDivElement | null>(null);
  const chartRef = useRef<IChartApi | null>(null);

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
        borderColor: "#2a2f38",
        scaleMargins: { top: 0.1, bottom: 0.25 },
      },
      leftPriceScale: {
        visible: true,
        borderColor: "#2a2f38",
        scaleMargins: { top: 0.1, bottom: 0.25 },
      },
      timeScale: { borderColor: "#2a2f38", timeVisible: false },
      crosshair: {
        vertLine: { color: "#06b6d4", width: 1, style: LineStyle.Dotted },
        horzLine: { color: "#06b6d4", width: 1, style: LineStyle.Dotted },
      },
    });
    chartRef.current = chart;

    // Daily total bars, red/green by sign.
    const bars = chart.addSeries(HistogramSeries, {
      priceScaleId: "left",
      priceFormat: { type: "price", precision: 0, minMove: 1 },
      base: 0,
    });
    bars.setData(
      data.days
        .filter((d) => d.total != null)
        .map((d) => ({
          time: d.t as UTCTimestamp,
          value: d.total as number,
          color:
            (d.total as number) >= 0
              ? "rgba(34, 197, 94, 0.75)"
              : "rgba(239, 68, 68, 0.75)",
        })),
    );

    // Cumulative line on the right axis.
    let cum = 0;
    const cumSeries = data.days.map((d) => {
      cum += d.total ?? 0;
      return { time: d.t as UTCTimestamp, value: cum };
    });
    const cumLine = chart.addSeries(LineSeries, {
      priceScaleId: "right",
      color: "#06b6d4",
      lineWidth: 2,
      priceFormat: { type: "price", precision: 0, minMove: 1 },
    });
    cumLine.setData(cumSeries);

    chart.timeScale().fitContent();
    return () => {
      chart.remove();
      chartRef.current = null;
    };
  }, [data]);

  return <div ref={ref} className="w-full h-full" />;
}
