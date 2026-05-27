"use client";

import { useEffect, useRef } from "react";
import {
  createChart,
  BaselineSeries,
  type IChartApi,
  type UTCTimestamp,
} from "lightweight-charts";
import type { CbPremiumPoint } from "@/lib/btc/types";

export function CbPremiumSparkline({ data }: { data: CbPremiumPoint[] }) {
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
        horzLines: { visible: false },
      },
      rightPriceScale: { visible: false },
      leftPriceScale: { visible: false },
      timeScale: { visible: false },
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
      topFillColor2: "rgba(34, 197, 94, 0.0)",
      bottomLineColor: "rgba(239, 68, 68, 1)",
      bottomFillColor1: "rgba(239, 68, 68, 0.0)",
      bottomFillColor2: "rgba(239, 68, 68, 0.3)",
      lineWidth: 2,
      priceLineVisible: false,
      lastValueVisible: false,
      crosshairMarkerVisible: false,
    });
    series.setData(
      data.map((p) => ({
        time: p.t as UTCTimestamp,
        value: p.premiumBps,
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
