import { NextResponse } from "next/server";
import { anthropic } from "@/lib/anthropic";

export const dynamic = "force-dynamic";

async function fetchJson(url: string) {
  const base = process.env.NEXT_PUBLIC_BASE_URL || "http://localhost:3000";
  const res = await fetch(`${base}${url}`, { cache: "no-store" });
  if (!res.ok) return null;
  return res.json();
}

export async function GET() {
  const [priceHistory, regime, macro, fng, aiModels] = await Promise.allSettled([
    fetchJson("/api/price-history"),
    fetchJson("/api/regime"),
    fetchJson("/api/macro"),
    fetchJson("/api/fng"),
    fetchJson("/api/ai/frontier-models"),
  ]);

  const context: string[] = [];

  if (priceHistory.status === "fulfilled" && priceHistory.value) {
    const d = priceHistory.value;
    const series = d.series as { v: number }[];
    const last = series.at(-1)?.v;
    const prev = series.at(-2)?.v;
    const change = last && prev ? ((last - prev) / prev * 100).toFixed(1) : "?";
    context.push(`BTC price: $${last?.toLocaleString() ?? "?"} (24h: ${change}%)`);
    context.push(`ATH: $${d.ath?.v?.toLocaleString() ?? "?"}, days since ATH: ${d.daysSinceAth ?? "?"}`);
    context.push(`Days since halving: ${d.daysSinceLastHalving ?? "?"}`);
    if (d.powerLaw) context.push(`Power law sigma: ${d.powerLaw.sigma?.toFixed(3) ?? "?"}`);
  }

  if (regime.status === "fulfilled" && regime.value) {
    const d = regime.value;
    if (d.powerLawZ != null) context.push(`Power law Z-score: ${d.powerLawZ.toFixed(2)}`);
    if (d.regime) context.push(`Current regime: ${d.regime}`);
    if (d.mayerMultiple) context.push(`Mayer multiple: ${d.mayerMultiple.toFixed(2)}`);
  }

  if (macro.status === "fulfilled" && macro.value) {
    const indicators = macro.value.indicators as { key: string; label: string; current: number | null; delta30d: number | null; unit: string }[];
    for (const i of indicators) {
      if (i.current != null) {
        const delta = i.delta30d != null ? ` (30d change: ${i.delta30d > 0 ? "+" : ""}${i.delta30d.toFixed(2)}${i.unit})` : "";
        context.push(`${i.label}: ${i.current.toFixed(2)}${i.unit}${delta}`);
      }
    }
  }

  if (fng.status === "fulfilled" && fng.value) {
    const c = fng.value.current;
    if (c) context.push(`Fear & Greed Index: ${c.v} (${c.label})`);
  }

  if (aiModels.status === "fulfilled" && aiModels.value) {
    const points = aiModels.value.points as { label: string; intelligenceIndex: number; blendedUsdPerMillion: number }[];
    if (points.length > 0) {
      const top = points[0];
      const cheapest = [...points].sort((a, b) => a.blendedUsdPerMillion - b.blendedUsdPerMillion)[0];
      context.push(`Top frontier model: ${top.label} (intelligence index: ${top.intelligenceIndex.toFixed(1)})`);
      context.push(`Cheapest frontier model: ${cheapest.label} ($${cheapest.blendedUsdPerMillion.toFixed(2)}/1M tokens)`);
      context.push(`${points.length} frontier models tracked`);
    }
  }

  if (context.length === 0) {
    return NextResponse.json({ briefing: "Unable to generate briefing — data sources unavailable.", generatedAt: new Date().toISOString() });
  }

  const msg = await anthropic.messages.create({
    model: "claude-sonnet-4-6",
    max_tokens: 400,
    messages: [
      {
        role: "user",
        content: `You are a concise market analyst writing a daily morning briefing for a trader who monitors BTC, macro/dollar, and AI markets. Based on the following live data, write a 3-paragraph briefing (each paragraph 2-3 sentences max). First paragraph: crypto/BTC state. Second: macro/dollar/rates. Third: AI frontier. Be direct, use specific numbers, highlight what changed and what matters. No greetings, no disclaimers.

Data:
${context.join("\n")}`,
      },
    ],
  });

  const briefing = msg.content
    .filter((b) => b.type === "text")
    .map((b) => b.text)
    .join("\n\n");

  return NextResponse.json({
    briefing,
    generatedAt: new Date().toISOString(),
    dataPoints: context.length,
  });
}
