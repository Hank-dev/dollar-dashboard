"use client";

import { useEffect, useRef } from "react";
import {
  createChart,
  LineSeries,
  type IChartApi,
  type UTCTimestamp,
} from "lightweight-charts";
import type { FngResponse } from "@/lib/btc/types";

export function FngSparkline({ data }: { data: FngResponse }) {
  const ref = useRef<HTMLDivElement | null>(null);
  const chartRef = useRef<IChartApi | null>(null);

  useEffect(() => {
    if (!ref.current) return;
    const el = ref.current;
    const chart = createChart(el, {
      width: el.clientWidth,
      height: 60,
      autoSize: true,
      layout: { background: { color: "transparent" }, textColor: "#555b65" },
      grid: {
        vertLines: { visible: false },
        horzLines: { visible: false },
      },
      rightPriceScale: { visible: false },
      leftPriceScale: { visible: false },
      timeScale: { visible: false },
      handleScroll: false,
      handleScale: false,
      crosshair: { horzLine: { visible: false }, vertLine: { visible: false } },
    });
    chartRef.current = chart;

    const line = chart.addSeries(LineSeries, {
      color: "#06b6d4",
      lineWidth: 1,
      priceLineVisible: false,
      lastValueVisible: false,
      crosshairMarkerVisible: false,
    });
    line.setData(
      data.history.map((p) => ({
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

  return <div ref={ref} className="w-full h-[60px]" />;
}
