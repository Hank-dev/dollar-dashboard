import { NextResponse } from "next/server";
import { anthropic } from "@/lib/anthropic";
import { fetchBtcDailyHistory } from "@/lib/btc/data/btcPrice";
import { fetchFredSeries } from "@/lib/btc/data/fred";
import { fetchFng } from "@/lib/btc/data/alternativeme";
import { fitPowerLaw, powerLawZ } from "@/lib/btc/calc/powerLaw";
import { SECONDS_PER_DAY, sma } from "@/lib/btc/calc/util";
import { mayerMultiple } from "@/lib/btc/calc/indicators";
import type { DailyPoint } from "@/lib/btc/types";
import {
  FRONTIER_MODEL_CANDIDATES,
  type FrontierModelPoint,
} from "@/lib/frontierModels";

export const dynamic = "force-dynamic";

async function fetchFrontierModels(): Promise<FrontierModelPoint[]> {
  const OPENROUTER_URL = "https://openrouter.ai/api/v1/models";
  let openRouterMap = new Map<string, { prompt?: string; completion?: string }>();
  try {
    const res = await fetch(OPENROUTER_URL, { cache: "no-store" });
    if (res.ok) {
      const body = (await res.json()) as { data?: { id: string; pricing?: { prompt?: string; completion?: string } }[] };
      openRouterMap = new Map((body.data ?? []).map((m) => [m.id, m.pricing ?? {}]));
    }
  } catch {}

  const points: FrontierModelPoint[] = [];
  const AI_API_COST_BASE = "https://www.aiapicost.com";

  await Promise.allSettled(
    FRONTIER_MODEL_CANDIDATES.map(async (c) => {
      const sourceUrl = `${AI_API_COST_BASE}${c.aiApiCostPath}`;
      const res = await fetch(sourceUrl, { cache: "no-store", headers: { Accept: "text/html", "User-Agent": "market-monitor/0.1" } });
      if (!res.ok) return;
      const html = await res.text();
      const idxMatch = html.match(/Intelligence Index:\s*([0-9.]+)/) ?? html.match(/Intelligence Index<\/a><span[^>]*>([0-9.]+)<\/span>/);
      const intelligenceIndex = idxMatch ? Number(idxMatch[1]) : null;
      if (intelligenceIndex == null || !Number.isFinite(intelligenceIndex)) return;

      const inputMatch = html.match(/Input:\s*\$([0-9.]+)\/M tokens/i) ?? html.match(/:\s*\$([0-9.]+)\/M input/i);
      const outputMatch = html.match(/Output:\s*\$([0-9.]+)\/M tokens/i) ?? html.match(/\$[0-9.]+\/M input,\s*\$([0-9.]+)\/M output/i);
      let inputUsd = inputMatch ? Number(inputMatch[1]) : null;
      let outputUsd = outputMatch ? Number(outputMatch[1]) : null;

      if (inputUsd == null || outputUsd == null) {
        const orPricing = c.openRouterId ? openRouterMap.get(c.openRouterId) : undefined;
        if (orPricing?.prompt && inputUsd == null) inputUsd = Number(orPricing.prompt) * 1_000_000;
        if (orPricing?.completion && outputUsd == null) outputUsd = Number(orPricing.completion) * 1_000_000;
      }
      if (inputUsd == null || outputUsd == null) return;

      const blended = (inputUsd * 3 + outputUsd) / 4;
      points.push({ id: c.id, label: c.label, provider: c.provider, intelligenceIndex, inputUsdPerMillion: inputUsd, outputUsdPerMillion: outputUsd, blendedUsdPerMillion: blended, blendedUsdPerToken: blended / 1_000_000, sourceUrl, pricingSource: "AI API Cost" });
    }),
  );

  return points.sort((a, b) => b.intelligenceIndex - a.intelligenceIndex);
}

export async function GET() {
  const context: string[] = [];

  const [btcResult, macroResults, fngResult, aiResult] = await Promise.allSettled([
    fetchBtcDailyHistory(),
    Promise.allSettled([
      fetchFredSeries("DTWEXBGS", new Date(Date.now() - 90 * 86400000).toISOString().slice(0, 10)),
      fetchFredSeries("VIXCLS", new Date(Date.now() - 90 * 86400000).toISOString().slice(0, 10)),
      fetchFredSeries("DFII10", new Date(Date.now() - 90 * 86400000).toISOString().slice(0, 10)),
      fetchFredSeries("BAMLH0A0HYM2", new Date(Date.now() - 90 * 86400000).toISOString().slice(0, 10)),
    ]),
    fetchFng(30),
    fetchFrontierModels(),
  ]);

  if (btcResult.status === "fulfilled") {
    const series = btcResult.value;
    const daily: DailyPoint[] = series.map((r) => ({ t: r.t, v: r.v }));
    const last = daily.at(-1);
    const prev = daily.at(-2);
    if (last && prev) {
      const change = ((last.v - prev.v) / prev.v * 100).toFixed(1);
      context.push(`BTC price: $${Math.round(last.v).toLocaleString()} (24h: ${change}%)`);
    }
    if (daily.length > 200) {
      const prices = daily.map((d) => d.v);
      const sma200 = sma(prices, 200);
      const lastSma = sma200.at(-1);
      if (last && lastSma) {
        const mm = mayerMultiple(last.v, lastSma);
        if (mm != null) context.push(`Mayer multiple: ${mm.toFixed(2)}`);
      }
      const fit = fitPowerLaw(daily);
      const z = powerLawZ(fit, last!.t, last!.v);
      context.push(`Power law Z-score: ${z.toFixed(2)}`);
    }
    const ath = daily.reduce((max, d) => d.v > max.v ? d : max, daily[0]);
    if (last && ath) {
      const fromAth = ((last.v - ath.v) / ath.v * 100).toFixed(1);
      context.push(`ATH: $${Math.round(ath.v).toLocaleString()} (${fromAth}% from ATH)`);
      const daysSinceAth = Math.floor((last.t - ath.t) / SECONDS_PER_DAY);
      context.push(`Days since ATH: ${daysSinceAth}`);
    }
  }

  if (macroResults.status === "fulfilled") {
    const [dxyR, vixR, realYieldR, hyR] = macroResults.value;
    const summarize = (name: string, unit: string, r: PromiseSettledResult<{ t: number; v: number }[]>) => {
      if (r.status !== "fulfilled" || r.value.length === 0) return;
      const s = r.value;
      const latest = s.at(-1)!;
      const cutoff = latest.t - 30 * SECONDS_PER_DAY;
      const prev = s.findLast((p) => p.t <= cutoff);
      const delta = prev ? (latest.v - prev.v) : null;
      const deltaStr = delta != null ? ` (30d: ${delta > 0 ? "+" : ""}${delta.toFixed(2)}${unit})` : "";
      context.push(`${name}: ${latest.v.toFixed(2)}${unit}${deltaStr}`);
    };
    summarize("DXY (broad USD)", "", dxyR);
    summarize("VIX", "", vixR);
    summarize("10Y real yield", "%", realYieldR);
    summarize("HY spread", "%", hyR);
  }

  if (fngResult.status === "fulfilled" && fngResult.value.length > 0) {
    const latest = fngResult.value.at(-1)!;
    context.push(`Fear & Greed Index: ${latest.v} (${latest.label})`);
  }

  if (aiResult.status === "fulfilled" && aiResult.value.length > 0) {
    const points = aiResult.value;
    const top = points[0];
    const cheapest = [...points].sort((a, b) => a.blendedUsdPerMillion - b.blendedUsdPerMillion)[0];
    context.push(`Top frontier model: ${top.label} (intelligence index: ${top.intelligenceIndex.toFixed(1)})`);
    context.push(`Cheapest frontier model: ${cheapest.label} ($${cheapest.blendedUsdPerMillion.toFixed(2)}/1M tokens)`);
    context.push(`${points.length} frontier models tracked`);
  }

  if (context.length === 0) {
    return NextResponse.json({
      briefing: "Unable to generate briefing — all data sources failed. Check API keys and network connectivity.",
      generatedAt: new Date().toISOString(),
    });
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
