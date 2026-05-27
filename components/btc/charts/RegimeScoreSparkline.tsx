"use client";

import { useEffect, useRef } from "react";
import {
  createChart,
  BaselineSeries,
  type IChartApi,
  type UTCTimestamp,
  LineStyle,
} from "lightweight-charts";
import type { RegimeScoreHistoryPoint } from "@/lib/btc/types";

export function RegimeScoreSparkline({
  data,
}: {
  data: RegimeScoreHistoryPoint[];
}) {
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
        textColor: "#555b65",
        fontFamily: "var(--font-jetbrains-mono), monospace",
        fontSize: 10,
      },
      grid: {
        vertLines: { visible: false },
        horzLines: { color: "#1a1d23" },
      },
      rightPriceScale: {
        borderColor: "#2a2f38",
        autoScale: false,
        scaleMargins: { top: 0.05, bottom: 0.05 },
      },
      leftPriceScale: { visible: false },
      timeScale: { borderColor: "#2a2f38", timeVisible: false },
      handleScroll: false,
      handleScale: false,
      crosshair: {
        horzLine: { visible: false },
        vertLine: { visible: false },
      },
    });
    chartRef.current = chart;

    const series = chart.addSeries(BaselineSeries, {
      baseValue: { type: "price", price: 0 },
      topLineColor: "rgba(34, 197, 94, 1)",
      topFillColor1: "rgba(34, 197, 94, 0.3)",
      topFillColor2: "rgba(34, 197, 94, 0.05)",
      bottomLineColor: "rgba(239, 68, 68, 1)",
      bottomFillColor1: "rgba(239, 68, 68, 0.05)",
      bottomFillColor2: "rgba(239, 68, 68, 0.3)",
      lineWidth: 2,
      priceLineVisible: false,
      lastValueVisible: false,
      crosshairMarkerVisible: false,
    });
    series.setData(
      data.map((p) => ({
        time: p.t as UTCTimestamp,
        value: p.score,
      })),
    );
    // Anchor the axis to [-5, +5] so the visual scale is consistent.
    const anchor = chart.addSeries(BaselineSeries, {
      baseValue: { type: "price", price: 0 },
      topLineColor: "rgba(0,0,0,0)", topFillColor1: "rgba(0,0,0,0)", topFillColor2: "rgba(0,0,0,0)",
      bottomLineColor: "rgba(0,0,0,0)", bottomFillColor1: "rgba(0,0,0,0)", bottomFillColor2: "rgba(0,0,0,0)",
      lineWidth: 1,
      priceLineVisible: false,
      lastValueVisible: false,
      crosshairMarkerVisible: false,
    });
    if (data.length > 0) {
      anchor.setData([
        { time: data[0].t as UTCTimestamp, value: 5 },
        { time: data[data.length - 1].t as UTCTimestamp, value: -5 },
      ]);
    }
    // Reference lines at +3 (full long) and -2 (fully out)
    series.createPriceLine({ price: 3, color: "#2a2f38", lineStyle: LineStyle.Dashed, lineWidth: 1, axisLabelVisible: false, title: "" });
    series.createPriceLine({ price: -2, color: "#2a2f38", lineStyle: LineStyle.Dashed, lineWidth: 1, axisLabelVisible: false, title: "" });
    series.createPriceLine({ price: 0, color: "#1a1d23", lineStyle: LineStyle.Dotted, lineWidth: 1, axisLabelVisible: false, title: "" });

    chart.timeScale().fitContent();
    return () => {
      chart.remove();
      chartRef.current = null;
    };
  }, [data]);

  return <div ref={ref} className="w-full h-full" />;
}
