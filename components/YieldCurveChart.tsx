"use client";

import { useEffect, useMemo, useRef, useState } from "react";
import {
  CategoryScale,
  Chart as ChartJS,
  Filler,
  LinearScale,
  LineElement,
  PointElement,
  Tooltip,
  type ChartOptions,
} from "chart.js";
import { Line } from "react-chartjs-2";
import { YIELD_CURVE } from "@/lib/metrics";

ChartJS.register(
  CategoryScale,
  LinearScale,
  LineElement,
  PointElement,
  Tooltip,
  Filler,
);

function useDarkMode() {
  const [dark, setDark] = useState(false);
  useEffect(() => {
    if (typeof window === "undefined") return;
    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    setDark(mq.matches);
    const onChange = (e: MediaQueryListEvent) => setDark(e.matches);
    mq.addEventListener("change", onChange);
    return () => mq.removeEventListener("change", onChange);
  }, []);
  return dark;
}

const valueLabelPlugin = {
  id: "valueLabels",
  afterDatasetsDraw(chart: ChartJS) {
    const { ctx } = chart;
    const meta = chart.getDatasetMeta(0);
    if (!meta) return;
    ctx.save();
    ctx.font = "500 11px system-ui, -apple-system, 'Segoe UI', sans-serif";
    ctx.textAlign = "center";
    ctx.textBaseline = "bottom";
    ctx.fillStyle =
      (chart.options as { _labelColor?: string })._labelColor ?? "#1c1917";
    meta.data.forEach((point, i) => {
      const y = YIELD_CURVE[i]?.y;
      if (y === undefined) return;
      const { x, y: py } = point.getProps(["x", "y"], true) as {
        x: number;
        y: number;
      };
      ctx.fillText(`${y.toFixed(2)}%`, x, py - 8);
    });
    ctx.restore();
  },
};

export default function YieldCurveChart() {
  const dark = useDarkMode();
  const containerRef = useRef<HTMLDivElement>(null);

  const colors = useMemo(() => {
    return dark
      ? {
          line: "#f7b955",
          point: "#f7b955",
          grid: "#26262a",
          tick: "#a8a29e",
          label: "#e7e5e4",
          fillFrom: "rgba(247, 185, 85, 0.18)",
          fillTo: "rgba(247, 185, 85, 0.0)",
        }
      : {
          line: "#c2780f",
          point: "#c2780f",
          grid: "#e7e5e4",
          tick: "#57534e",
          label: "#1c1917",
          fillFrom: "rgba(194, 120, 15, 0.12)",
          fillTo: "rgba(194, 120, 15, 0.0)",
        };
  }, [dark]);

  const data = useMemo(
    () => ({
      labels: YIELD_CURVE.map((p) => p.m),
      datasets: [
        {
          data: YIELD_CURVE.map((p) => p.y),
          borderColor: colors.line,
          backgroundColor: (ctx: { chart: ChartJS }) => {
            const chart = ctx.chart;
            const { ctx: c, chartArea } = chart;
            if (!chartArea) return colors.fillFrom;
            const gradient = c.createLinearGradient(0, chartArea.top, 0, chartArea.bottom);
            gradient.addColorStop(0, colors.fillFrom);
            gradient.addColorStop(1, colors.fillTo);
            return gradient;
          },
          borderWidth: 1.75,
          pointRadius: 4,
          pointBackgroundColor: colors.point,
          pointBorderColor: colors.point,
          tension: 0.25,
          fill: true,
        },
      ],
    }),
    [colors],
  );

  const options: ChartOptions<"line"> & { _labelColor?: string } = useMemo(
    () => ({
      responsive: true,
      maintainAspectRatio: false,
      animation: false,
      _labelColor: colors.label,
      layout: { padding: { top: 22, right: 12, bottom: 4, left: 4 } },
      plugins: {
        legend: { display: false },
        tooltip: {
          enabled: true,
          backgroundColor: dark ? "#141416" : "#ffffff",
          titleColor: colors.label,
          bodyColor: colors.label,
          borderColor: colors.grid,
          borderWidth: 1,
          displayColors: false,
          padding: 8,
          callbacks: {
            label: (ctx) =>
              ctx.parsed.y != null ? `${ctx.parsed.y.toFixed(2)}%` : "",
          },
        },
      },
      scales: {
        x: {
          grid: { color: colors.grid, drawTicks: false },
          border: { display: false },
          ticks: { color: colors.tick, font: { size: 11 } },
        },
        y: {
          min: 3.5,
          max: 5.5,
          grid: { color: colors.grid, drawTicks: false },
          border: { display: false },
          ticks: {
            color: colors.tick,
            font: { size: 11 },
            stepSize: 0.5,
            callback: (v: string | number) => `${Number(v).toFixed(1)}%`,
          },
        },
      },
    }),
    [colors, dark],
  );

  return (
    <div
      ref={containerRef}
      style={{ height: 220 }}
      role="img"
      aria-label="US Treasury yield curve: 2-year 4.13%, 10-year 4.56%, 30-year 5.06%. Curve is steep at the long end."
    >
      <Line data={data} options={options} plugins={[valueLabelPlugin]} />
    </div>
  );
}
