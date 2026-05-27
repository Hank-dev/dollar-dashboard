"use client";

import { useEffect, useRef } from "react";
import {
  createChart,
  AreaSeries,
  type IChartApi,
  type UTCTimestamp,
  LineStyle,
} from "lightweight-charts";
import type { NetLiquidityResponse } from "@/lib/btc/types";

export function NetLiquidityChart({ data }: { data: NetLiquidityResponse }) {
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
      rightPriceScale: { borderColor: "#2a2f38" },
      timeScale: {
        borderColor: "#2a2f38",
        timeVisible: false,
      },
      crosshair: {
        vertLine: { color: "#06b6d4", width: 1, style: LineStyle.Dotted },
        horzLine: { color: "#06b6d4", width: 1, style: LineStyle.Dotted },
      },
    });
    chartRef.current = chart;

    const nl = chart.addSeries(AreaSeries, {
      lineColor: "#06b6d4",
      topColor: "rgba(6, 182, 212, 0.25)",
      bottomColor: "rgba(6, 182, 212, 0.0)",
      lineWidth: 2,
      priceFormat: { type: "price", precision: 0, minMove: 1 },
    });
    nl.setData(
      data.series.map((p) => ({
        time: p.t as UTCTimestamp,
        value: p.v,
      })),
    );

    chart.timeScale().fitContent();
    return () => {
      chart.remove();
      chartRef.current = null;
    };
  }, [data]);

  return <div ref={ref} className="w-full h-full" />;
}
