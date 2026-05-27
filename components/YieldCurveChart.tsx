"use client";

import { useEffect, useMemo, useRef, useState } from "react";
import {
  CategoryScale,
  Chart as ChartJS,
  Filler,
  Legend,
  LinearScale,
  LineElement,
  PointElement,
  Tooltip,
  type ChartOptions,
} from "chart.js";
import { Line } from "react-chartjs-2";
import type { CurvePoint } from "@/lib/metrics";

ChartJS.register(
  CategoryScale,
  LinearScale,
  LineElement,
  PointElement,
  Tooltip,
  Filler,
  Legend,
);

interface CurveHistory {
  current: CurvePoint[];
  past30: CurvePoint[];
  past90: CurvePoint[];
  dates: { current: string; past30: string; past90: string };
}

const valueLabelPlugin = {
  id: "valueLabels",
  afterDatasetsDraw(chart: ChartJS) {
    const { ctx } = chart;
    const meta = chart.getDatasetMeta(0);
    if (!meta) return;
    const values = chart.data.datasets[0]?.data as number[] | undefined;
    if (!values) return;
    ctx.save();
    ctx.font = "500 11px var(--font-mono), system-ui, sans-serif";
    ctx.textAlign = "center";
    ctx.textBaseline = "bottom";
    ctx.fillStyle =
      (chart.options as { _labelColor?: string })._labelColor ?? "#e5e8ef";
    meta.data.forEach((point, i) => {
      const y = values[i];
      if (typeof y !== "number") return;
      const { x, y: py } = point.getProps(["x", "y"], true) as {
        x: number;
        y: number;
      };
      ctx.fillText(`${y.toFixed(2)}%`, x, py - 8);
    });
    ctx.restore();
  },
};

export default function YieldCurveChart({ points }: { points: CurvePoint[] }) {
  const containerRef = useRef<HTMLDivElement>(null);
  const [history, setHistory] = useState<CurveHistory | null>(null);
  const [showPeriod, setShowPeriod] = useState<"30d" | "90d">("30d");

  useEffect(() => {
    fetch("/api/yield-curve-history")
      .then((r) => r.json())
      .then((d) => setHistory(d))
      .catch(() => {});
  }, []);

  const currentCurve = history?.current ?? points;
  const pastCurve = showPeriod === "30d" ? history?.past30 : history?.past90;
  const pastDate = showPeriod === "30d" ? history?.dates.past30 : history?.dates.past90;

  const labels = currentCurve.map((p) => p.m);

  const data = useMemo(
    () => ({
      labels,
      datasets: [
        {
          label: "Current",
          data: currentCurve.map((p) => p.y),
          borderColor: "oklch(0.73 0.14 245)",
          backgroundColor: "rgba(100, 140, 220, 0.08)",
          borderWidth: 2,
          pointRadius: 4,
          pointBackgroundColor: "oklch(0.73 0.14 245)",
          pointBorderColor: "oklch(0.73 0.14 245)",
          tension: 0.25,
          fill: true,
        },
        ...(pastCurve && pastCurve.length > 0
          ? [
              {
                label: pastDate ? `${showPeriod} ago (${pastDate})` : `${showPeriod} ago`,
                data: pastCurve.map((p) => p.y),
                borderColor: "var(--text-tertiary)",
                backgroundColor: "transparent",
                borderWidth: 1.2,
                borderDash: [4, 3],
                pointRadius: 2.5,
                pointBackgroundColor: "var(--text-tertiary)",
                pointBorderColor: "var(--text-tertiary)",
                tension: 0.25,
                fill: false,
              },
            ]
          : []),
      ],
    }),
    [currentCurve, pastCurve, pastDate, labels, showPeriod],
  );

  const { yMin, yMax } = useMemo(() => {
    const allPoints = [...currentCurve, ...(pastCurve ?? [])];
    const ys = allPoints.map((p) => p.y);
    if (ys.length === 0) return { yMin: 0, yMax: 6 };
    const lo = Math.min(...ys);
    const hi = Math.max(...ys);
    const pad = Math.max(0.5, (hi - lo) * 0.4);
    return {
      yMin: Math.floor((lo - pad) * 2) / 2,
      yMax: Math.ceil((hi + pad) * 2) / 2,
    };
  }, [currentCurve, pastCurve]);

  const options: ChartOptions<"line"> & { _labelColor?: string } = useMemo(
    () => ({
      responsive: true,
      maintainAspectRatio: false,
      animation: false,
      _labelColor: "#e5e8ef",
      layout: { padding: { top: 22, right: 12, bottom: 4, left: 4 } },
      plugins: {
        legend: {
          display: pastCurve != null && pastCurve.length > 0,
          position: "bottom" as const,
          labels: {
            boxWidth: 12,
            boxHeight: 2,
            color: "#828a98",
            font: { size: 11 },
            padding: 16,
          },
        },
        tooltip: {
          enabled: true,
          backgroundColor: "#0e1015",
          titleColor: "#e5e8ef",
          bodyColor: "#e5e8ef",
          borderColor: "#1a1e26",
          borderWidth: 1,
          displayColors: true,
          padding: 8,
          callbacks: {
            label: (ctx) =>
              ctx.parsed.y != null ? ` ${ctx.dataset.label}: ${ctx.parsed.y.toFixed(2)}%` : "",
          },
        },
      },
      scales: {
        x: {
          grid: { color: "#1a1e26", drawTicks: false },
          border: { display: false },
          ticks: { color: "#828a98", font: { size: 11 } },
        },
        y: {
          min: yMin,
          max: yMax,
          grid: { color: "#1a1e26", drawTicks: false },
          border: { display: false },
          ticks: {
            color: "#828a98",
            font: { size: 11 },
            stepSize: 0.5,
            callback: (v: string | number) => `${Number(v).toFixed(1)}%`,
          },
        },
      },
    }),
    [pastCurve, yMin, yMax],
  );

  return (
    <div>
      {history && (
        <div className="flex items-center gap-1 mb-3">
          <span className="mono text-[10px] text-[var(--text-tertiary)] tracking-[0.1em] uppercase mr-2">
            Compare:
          </span>
          {(["30d", "90d"] as const).map((p) => (
            <button
              key={p}
              onClick={() => setShowPeriod(p)}
              className={`mono text-[10.5px] px-1.5 py-0.5 rounded-sm cursor-pointer transition-colors ${
                showPeriod === p
                  ? "bg-[var(--bg-elevated)] text-[var(--text-primary)]"
                  : "text-[var(--text-tertiary)] hover:text-[var(--text-secondary)]"
              }`}
              style={{ border: "none", background: showPeriod === p ? "var(--bg-elevated)" : "transparent" }}
            >
              {p}
            </button>
          ))}
        </div>
      )}
      <div
        ref={containerRef}
        style={{ height: 240 }}
        role="img"
        aria-label={`US Treasury yield curve: ${currentCurve.map((p) => `${p.m} ${p.y.toFixed(2)}%`).join(", ")}.`}
      >
        <Line data={data} options={options} plugins={[valueLabelPlugin]} />
      </div>
    </div>
  );
}
