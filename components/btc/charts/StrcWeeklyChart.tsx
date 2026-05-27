"use client";

import { useEffect, useRef } from "react";
import {
  createChart,
  HistogramSeries,
  type IChartApi,
  type PriceFormatCustom,
  type UTCTimestamp,
} from "lightweight-charts";

// Show "$XM" for sub-billion values and "$X.XB" for billions, so the y-axis
// tells you the unit at a glance. Without this the bars look bafflingly tall
// because the eye sees "1500" with no context.
const PROCEEDS_FORMAT: PriceFormatCustom = {
  type: "custom",
  minMove: 1,
  formatter: (price: number) => {
    const abs = Math.abs(price);
    if (abs >= 1000) return `$${(price / 1000).toFixed(1)}B`;
    if (abs >= 10) return `$${Math.round(price)}M`;
    return `$${price.toFixed(1)}M`;
  },
};
import type { StrcWeekPoint } from "@/lib/btc/types";

export function StrcWeeklyChart({ data }: { data: StrcWeekPoint[] }) {
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
        fontSize: 10,
      },
      grid: {
        vertLines: { visible: false },
        horzLines: { color: "#1a1d23" },
      },
      rightPriceScale: {
        borderColor: "#2a2f38",
        // Generous top margin so the tallest bar doesn't fill the entire pane.
        scaleMargins: { top: 0.25, bottom: 0.05 },
      },
      timeScale: {
        borderColor: "#2a2f38",
        timeVisible: false,
        secondsVisible: false,
        rightOffset: 1,
        barSpacing: 18,
        minBarSpacing: 12,
        fixLeftEdge: true,
        fixRightEdge: true,
        tickMarkFormatter: (t: number) => {
          const d = new Date(t * 1000);
          // "Nov 18" — concise for 12-bar histograms
          return d.toLocaleDateString("en-US", {
            month: "short",
            day: "numeric",
            timeZone: "UTC",
          });
        },
      },
      handleScroll: false,
      handleScale: false,
      crosshair: {
        horzLine: { visible: false },
        vertLine: { visible: false },
      },
    });
    chartRef.current = chart;

    // Total weekly ATM proceeds (STRC + STRF + STRK + STRD + MSTR combined).
    // STRC alone is rarely tapped; the total tells the real "how much money
    // is Strategy raising this week" story.
    const totalBars = chart.addSeries(HistogramSeries, {
      priceFormat: PROCEEDS_FORMAT,
      base: 0,
      priceLineVisible: false,
      lastValueVisible: false,
    });
    totalBars.setData(
      data.map((p) => ({
        time: p.t as UTCTimestamp,
        value: p.totalProceedsM,
        color: "rgba(6, 182, 212, 0.65)", // cyan — neutral capital-raise activity
      })),
    );

    // STRC-only overlay: smaller bars drawn on top of the total bars so the
    // contribution of STRC to the total is visible without needing a second
    // axis.
    const strcBars = chart.addSeries(HistogramSeries, {
      priceFormat: PROCEEDS_FORMAT,
      base: 0,
      priceLineVisible: false,
      lastValueVisible: false,
    });
    strcBars.setData(
      data.map((p) => ({
        time: p.t as UTCTimestamp,
        value: p.netProceedsM,
        color: "rgba(245, 158, 11, 0.9)", // amber for STRC
      })),
    );

    chart.timeScale().fitContent();
    return () => {
      chart.remove();
      chartRef.current = null;
    };
  }, [data]);

  return (
    <div className="flex flex-col h-full">
      <div ref={ref} className="flex-1 min-h-0" />
      <div className="flex flex-wrap gap-x-3 gap-y-0.5 px-3 py-0.5 border-t border-[var(--border)] mono text-[10px] text-[var(--text-secondary)]">
        <span className="inline-flex items-center gap-1.5">
          <span
            className="inline-block w-3 h-2"
            style={{ background: "rgba(6, 182, 212, 0.65)" }}
          />
          Total ($M)
        </span>
        <span className="inline-flex items-center gap-1.5">
          <span
            className="inline-block w-3 h-2"
            style={{ background: "rgba(245, 158, 11, 0.9)" }}
          />
          STRC only
        </span>
        <span className="text-[var(--text-tertiary)]">
          {data.length > 0 &&
            `${new Date(data[0].t * 1000)
              .toISOString()
              .slice(0, 10)} → ${new Date(data[data.length - 1].t * 1000)
              .toISOString()
              .slice(0, 10)}`}
        </span>
      </div>
    </div>
  );
}
