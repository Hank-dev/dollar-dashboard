"use client";

import { useEffect, useMemo, useState } from "react";
import {
  Chart as ChartJS,
  Legend,
  LinearScale,
  LogarithmicScale,
  PointElement,
  Tooltip,
  type ChartData,
  type ChartOptions,
} from "chart.js";
import { Scatter } from "react-chartjs-2";
import type {
  FrontierModelPoint,
  FrontierModelsResponse,
} from "@/lib/frontierModels";

ChartJS.register(LinearScale, LogarithmicScale, PointElement, Tooltip, Legend);

const COLORS = [
  "#9d7cd8",
  "#c2780f",
  "#4f8cc9",
  "#a8553a",
  "#5fa8d6",
  "#639922",
  "#d04f7a",
  "#77716a",
];

export default function FrontierModelChart() {
  const [data, setData] = useState<FrontierModelsResponse | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const controller = new AbortController();

    async function load() {
      try {
        setLoading(true);
        setError(null);
        const res = await fetch("/api/ai/frontier-models", {
          signal: controller.signal,
          cache: "no-store",
        });
        if (!res.ok) throw new Error(`Request failed (${res.status})`);
        const body = (await res.json()) as FrontierModelsResponse;
        setData(body);
      } catch (err) {
        if ((err as { name?: string }).name === "AbortError") return;
        setError(err instanceof Error ? err.message : "Could not load models.");
      } finally {
        setLoading(false);
      }
    }

    load();
    return () => controller.abort();
  }, []);

  const points = useMemo(() => data?.points ?? [], [data]);
  const chartData = useMemo<ChartData<"scatter">>(() => {
    return {
      datasets: points.map((point, index) => ({
        label: point.label,
        data: [{ x: point.blendedUsdPerMillion, y: point.intelligenceIndex }],
        pointRadius: 6,
        pointHoverRadius: 8,
        pointBackgroundColor: COLORS[index % COLORS.length],
        pointBorderColor: COLORS[index % COLORS.length],
        pointBorderWidth: 1,
        model: point,
      })),
    };
  }, [points]);

  const chartOptions = useMemo<ChartOptions<"scatter">>(
    () => ({
      responsive: true,
      maintainAspectRatio: false,
      animation: false,
      plugins: {
        legend: {
          position: "bottom",
          labels: {
            boxWidth: 8,
            boxHeight: 8,
            color: "rgb(120, 113, 108)",
            font: { size: 11 },
          },
        },
        tooltip: {
          displayColors: false,
          callbacks: {
            title: (items) => items[0]?.dataset.label ?? "",
            label: (ctx) => {
              const point = (ctx.dataset as { model?: FrontierModelPoint }).model;
              if (!point) return "";
              return [
                `Intelligence Index: ${point.intelligenceIndex.toFixed(1)}`,
                `Blended: $${point.blendedUsdPerMillion.toFixed(2)} / 1M tokens`,
                `Input: $${point.inputUsdPerMillion.toFixed(2)} / 1M`,
                `Output: $${point.outputUsdPerMillion.toFixed(2)} / 1M`,
              ];
            },
            footer: (items) => {
              const point = (items[0]?.dataset as { model?: FrontierModelPoint })
                ?.model;
              return point ? `Pricing source: ${point.pricingSource}` : "";
            },
          },
        },
      },
      scales: {
        x: {
          type: "logarithmic",
          title: {
            display: true,
            text: "Blended price, USD per 1M tokens (3 input : 1 output)",
            color: "rgb(120, 113, 108)",
            font: { size: 11 },
          },
          grid: { color: "rgba(120, 113, 108, 0.18)" },
          ticks: {
            color: "rgb(120, 113, 108)",
            callback: (value) => `$${Number(value).toFixed(2)}`,
          },
        },
        y: {
          title: {
            display: true,
            text: "Intelligence Index",
            color: "rgb(120, 113, 108)",
            font: { size: 11 },
          },
          grid: { color: "rgba(120, 113, 108, 0.18)" },
          ticks: { color: "rgb(120, 113, 108)" },
        },
      },
    }),
    [],
  );

  return (
    <div className="border border-[var(--border)] bg-[var(--bg-surface)] p-4" style={{ borderRadius: 8 }}>
      <div className="flex flex-col gap-2 sm:flex-row sm:items-start sm:justify-between">
        <div>
          <h3 className="text-[13px] font-medium text-[var(--text-primary)]">
            Frontier model intelligence vs token cost
          </h3>
          <p className="mt-1 text-[11.5px] leading-relaxed text-[var(--text-tertiary)]">
            Live fetch from AI API Cost model pages plus OpenRouter pricing
            where available. X-axis uses blended dollars per 1M tokens for
            readability.
          </p>
        </div>
        {data && (
          <p className="shrink-0 text-[11px] uppercase tracking-wide text-[var(--text-tertiary)]">
            Captured {new Date(data.capturedAt).toLocaleTimeString([], {
              hour: "2-digit",
              minute: "2-digit",
            })}
          </p>
        )}
      </div>

      <div className="mt-4 h-[360px]">
        {loading && (
          <div className="grid h-full place-items-center text-[12px] text-[var(--text-tertiary)]">
            Loading live model data...
          </div>
        )}
        {!loading && error && (
          <div className="grid h-full place-items-center text-[12.5px] text-[var(--text-primary)]">
            {error}
          </div>
        )}
        {!loading && !error && points.length > 0 && (
          <Scatter data={chartData} options={chartOptions} />
        )}
        {!loading && !error && points.length === 0 && (
          <div className="grid h-full place-items-center text-[12.5px] text-[var(--text-primary)]">
            No live model points were available.
          </div>
        )}
      </div>

      {points.length > 0 && (
        <div className="mt-4 overflow-x-auto">
          <table className="w-full min-w-[620px] text-left text-[11.5px]">
            <thead className="border-b border-[var(--border)] text-[10.5px] uppercase tracking-[0.12em] text-[var(--text-tertiary)]">
              <tr>
                <th className="py-2 pr-3 font-semibold">Model</th>
                <th className="py-2 pr-3 font-semibold">Index</th>
                <th className="py-2 pr-3 font-semibold">Blended</th>
                <th className="py-2 pr-3 font-semibold">Input</th>
                <th className="py-2 pr-3 font-semibold">Output</th>
                <th className="py-2 pr-3 font-semibold">Source</th>
              </tr>
            </thead>
            <tbody>
              {points.map((point) => (
                <tr key={point.id} className="border-b border-[var(--border)] last:border-b-0">
                  <td className="py-2 pr-3 text-[var(--text-primary)]">
                    <a
                      href={point.sourceUrl}
                      target="_blank"
                      rel="noreferrer"
                      className="underline decoration-[var(--border-strong)] underline-offset-2 hover:text-[var(--text-secondary)]"
                    >
                      {point.label}
                    </a>
                    <span className="block text-[10.5px] text-[var(--text-tertiary)]">
                      {point.provider}
                    </span>
                  </td>
                  <td className="py-2 pr-3 tabular-nums text-[var(--text-primary)]">
                    {point.intelligenceIndex.toFixed(1)}
                  </td>
                  <td className="py-2 pr-3 tabular-nums text-[var(--text-primary)]">
                    ${point.blendedUsdPerMillion.toFixed(2)}
                  </td>
                  <td className="py-2 pr-3 tabular-nums text-[var(--text-secondary)]">
                    ${point.inputUsdPerMillion.toFixed(2)}
                  </td>
                  <td className="py-2 pr-3 tabular-nums text-[var(--text-secondary)]">
                    ${point.outputUsdPerMillion.toFixed(2)}
                  </td>
                  <td className="py-2 pr-3 text-[var(--text-tertiary)]">
                    {point.pricingSource}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {data && data.errors.length > 0 && (
        <p className="mt-3 text-[11px] text-[var(--text-tertiary)]">
          Some live model pages could not be parsed:{" "}
          {data.errors.map((item) => item.id).join(", ")}.
        </p>
      )}
    </div>
  );
}
