import { NextResponse } from "next/server";
import Anthropic from "@anthropic-ai/sdk";
import { fetchBtcDailyHistory } from "@/lib/btc/data/btcPrice";
import {
  fetchMvrvZ,
  fetchNupl,
  fetchRealizedPrice,
} from "@/lib/btc/data/bitcoinData";
import { fetchFng } from "@/lib/btc/data/alternativeme";
import { fetchFredSeries } from "@/lib/btc/data/fred";
import { computeNetLiquidity } from "@/lib/btc/calc/netLiquidity";
import { mayerMultiple } from "@/lib/btc/calc/indicators";
import { fitPowerLaw, powerLawZ } from "@/lib/btc/calc/powerLaw";
import { LAST_HALVING_UNIX, SECONDS_PER_DAY, sma } from "@/lib/btc/calc/util";
import type { InterpretationResponse } from "@/lib/btc/types";

export const revalidate = 86400;

type Summary = {
  spot: number;
  daysSinceAth: number;
  daysSinceLastHalving: number;
  powerLawZ: number | null;
  powerLawSlope: number;
  mvrvZ: number | null;
  nupl: number | null;
  mayer: number | null;
  realizedPrice: number | null;
  netLiquidityB: number | null;
  netLiquidityDelta30dB: number | null;
  fng: number | null;
  fngLabel: string | null;
};

async function buildSummary(): Promise<Summary> {
  const [prices, mvrvZ, nupl, realized, walcl, tga, rrp, fng] =
    await Promise.all([
      fetchBtcDailyHistory(),
      fetchMvrvZ(),
      fetchNupl(),
      fetchRealizedPrice(),
      fetchFredSeries("WALCL", "2015-01-01").catch(() => []),
      fetchFredSeries("WTREGEN", "2015-01-01").catch(() => []),
      fetchFredSeries("RRPONTSYD", "2015-01-01").catch(() => []),
      fetchFng(30).catch(() => []),
    ]);

  if (prices.length === 0) throw new Error("no price data");
  const last = prices[prices.length - 1];

  const fit = fitPowerLaw(prices);
  const pwZ = powerLawZ(fit, last.t, last.v);

  const priceArr = prices.map((p) => p.v);
  const smaArr = sma(priceArr, 200);
  const sma200 = smaArr[smaArr.length - 1];
  const mayer = mayerMultiple(last.v, sma200);

  let athIdx = 0;
  for (let i = 1; i < prices.length; i++) {
    if (prices[i].v > prices[athIdx].v) athIdx = i;
  }
  const daysSinceAth = Math.max(
    0,
    Math.floor((last.t - prices[athIdx].t) / SECONDS_PER_DAY),
  );
  const daysSinceLastHalving = Math.max(
    0,
    Math.floor((last.t - LAST_HALVING_UNIX) / SECONDS_PER_DAY),
  );

  const tail = <T extends { v: number }>(arr: T[]): number | null =>
    arr.length > 0 ? arr[arr.length - 1].v : null;

  const nl = computeNetLiquidity(walcl, tga, rrp);
  let netLiquidityB: number | null = null;
  let netLiquidityDelta30dB: number | null = null;
  if (nl.length > 0) {
    const latest = nl[nl.length - 1];
    netLiquidityB = latest.v;
    const back30 = latest.t - 30 * SECONDS_PER_DAY;
    for (let i = nl.length - 1; i >= 0; i--) {
      if (nl[i].t <= back30) {
        netLiquidityDelta30dB = latest.v - nl[i].v;
        break;
      }
    }
  }

  const fngCurrent = fng.length > 0 ? fng[fng.length - 1] : null;

  return {
    spot: last.v,
    daysSinceAth,
    daysSinceLastHalving,
    powerLawZ: Number.isFinite(pwZ) ? pwZ : null,
    powerLawSlope: fit.b,
    mvrvZ: tail(mvrvZ),
    nupl: tail(nupl),
    mayer,
    realizedPrice: tail(realized),
    netLiquidityB,
    netLiquidityDelta30dB,
    fng: fngCurrent?.v ?? null,
    fngLabel: fngCurrent?.label ?? null,
  };
}

function formatNum(v: number | null, digits = 2): string {
  return v == null ? "n/a" : v.toFixed(digits);
}

function buildPrompt(s: Summary): string {
  const lines = [
    `BTC spot: $${Math.round(s.spot).toLocaleString()}`,
    `Days since ATH: ${s.daysSinceAth}`,
    `Days since last halving: ${s.daysSinceLastHalving}`,
    `Power-law Z (price deviation from fit, in σ): ${formatNum(s.powerLawZ)}`,
    `Power-law slope: ${formatNum(s.powerLawSlope, 3)}`,
    `MVRV Z-Score: ${formatNum(s.mvrvZ)}`,
    `NUPL: ${formatNum(s.nupl)}`,
    `Mayer Multiple (price / 200d SMA): ${formatNum(s.mayer)}`,
    `Realized price: ${s.realizedPrice == null ? "n/a" : `$${Math.round(s.realizedPrice).toLocaleString()}`}`,
    `Net Liquidity (WALCL−TGA−RRP): ${s.netLiquidityB == null ? "n/a" : `$${s.netLiquidityB.toFixed(0)}B`}`,
    `Net Liquidity 30d change: ${s.netLiquidityDelta30dB == null ? "n/a" : `${s.netLiquidityDelta30dB >= 0 ? "+" : ""}$${s.netLiquidityDelta30dB.toFixed(0)}B`}`,
    `Fear & Greed: ${s.fng == null ? "n/a" : `${s.fng} (${s.fngLabel})`}`,
  ];
  return lines.join("\n");
}

const SYSTEM_PROMPT = `You are a quantitative BTC market analyst writing for a Bloomberg-style terminal dashboard. Your audience is a sophisticated trader who already understands MVRV, NUPL, Mayer multiple, power-law fits, and US dollar net liquidity. Be terse, technical, and direct — no marketing fluff, no hedging language, no disclaimers.

Write a market interpretation in 3 short paragraphs (≤180 words total):
1. Valuation: where does BTC sit on the power-law fit and SMA-based measures (Mayer, power-law Z)?
2. On-chain regime: what do MVRV-Z and NUPL imply about holder behavior (capitulation / fair / euphoria)?
3. Macro & sentiment: net-liquidity direction and F&G; flag any divergences between price, on-chain, and macro.

Use plain text only. No markdown, no headers, no bullet points, no emoji. Numbers should be embedded inline.`;

export async function GET() {
  if (!process.env.ANTHROPIC_API_KEY) {
    return NextResponse.json({
      disabled: true,
      reason:
        "ANTHROPIC_API_KEY not set in .env.local — see .env.local.example",
    } satisfies InterpretationResponse);
  }

  try {
    const inputs = await buildSummary();
    const prompt = buildPrompt(inputs);

    const client = new Anthropic();
    const response = await client.messages.create({
      model: "claude-haiku-4-5",
      max_tokens: 1024,
      system: SYSTEM_PROMPT,
      messages: [{ role: "user", content: prompt }],
    });

    const text = response.content
      .filter((b) => b.type === "text")
      .map((b) => (b.type === "text" ? b.text : ""))
      .join("")
      .trim();

    return NextResponse.json({
      text,
      generatedAt: Math.floor(Date.now() / 1000),
      inputs,
    } satisfies InterpretationResponse);
  } catch (err) {
    return NextResponse.json(
      { error: err instanceof Error ? err.message : String(err) },
      { status: 502 },
    );
  }
}
