import { NextResponse } from "next/server";
import { fetchFredSeries } from "@/lib/btc/data/fred";
import { fetchMvrvZ } from "@/lib/btc/data/bitcoinData";
import { fetchBtcDailyHistory } from "@/lib/btc/data/btcPrice";
import {
  REGIME_SCORE_CONFIG,
  computeRegimeScore,
  targetAllocationFromScore,
} from "@/lib/btc/calc/regimeScore";
import { SECONDS_PER_DAY } from "@/lib/btc/calc/util";
import type {
  DailyPoint,
  RegimeScoreInputDetail,
  RegimeScoreResponse,
} from "@/lib/btc/types";

export const revalidate = 86_400; // daily — matches Strategy D cadence

function toDailyPoints<T extends { t: number; v: number }>(s: T[]): DailyPoint[] {
  return s.map((p) => ({ t: p.t, v: p.v }));
}

function valueAtOrBefore(series: DailyPoint[], ts: number): number | null {
  if (series.length === 0) return null;
  let lo = 0;
  let hi = series.length - 1;
  let found = -1;
  while (lo <= hi) {
    const mid = (lo + hi) >> 1;
    if (series[mid].t <= ts) {
      found = mid;
      lo = mid + 1;
    } else {
      hi = mid - 1;
    }
  }
  if (found < 0) return null;
  const v = series[found].v;
  return Number.isFinite(v) ? v : null;
}

function describeFed(now: number | null, past: number | null): string {
  if (now == null || past == null) return "insufficient data";
  const diff = now - past;
  const band = REGIME_SCORE_CONFIG.fed.flatBandPctPoints;
  const dir = diff <= -band ? "falling" : diff >= band ? "rising" : "flat";
  return `now ${now.toFixed(2)}% · 6m ago ${past.toFixed(2)}% · ${dir} (Δ ${diff >= 0 ? "+" : ""}${diff.toFixed(2)} pp)`;
}

function describeM2(now: number | null, past: number | null): string {
  if (now == null || past == null || past === 0) return "insufficient data";
  const yoy = ((now - past) / past) * 100;
  return `YoY ${yoy >= 0 ? "+" : ""}${yoy.toFixed(2)}%`;
}

function describeMvrv(v: number | null): string {
  if (v == null) return "insufficient data";
  const zone =
    v < REGIME_SCORE_CONFIG.mvrv.lower
      ? "value zone"
      : v > REGIME_SCORE_CONFIG.mvrv.upper
        ? "overheated"
        : "neutral";
  return `Z ${v.toFixed(2)} · ${zone}`;
}

function describeBtc(weeklyCloses: number[]): string {
  const need = REGIME_SCORE_CONFIG.btc.smaWeeks;
  if (weeklyCloses.length < need) return "insufficient weekly history";
  const window = weeklyCloses.slice(-need);
  const sma = window.reduce((s, x) => s + x, 0) / need;
  const close = weeklyCloses[weeklyCloses.length - 1];
  const pct = ((close - sma) / sma) * 100;
  return `close $${close.toFixed(0)} · 200w SMA $${sma.toFixed(0)} · ${pct >= 0 ? "+" : ""}${pct.toFixed(1)}%`;
}

function describeDxy(now: number | null, past: number | null): string {
  if (now == null || past == null || past === 0) return "insufficient data";
  const chg = ((now - past) / past) * 100;
  return `now ${now.toFixed(2)} · 6m ago ${past.toFixed(2)} · ${chg >= 0 ? "+" : ""}${chg.toFixed(1)}%`;
}

function resampleDailyToWeekly(daily: DailyPoint[]): { ts: number; close: number }[] {
  const out: { ts: number; close: number }[] = [];
  let lastWeekKey: string | null = null;
  let lastPoint: DailyPoint | null = null;
  for (const p of daily) {
    const d = new Date(p.t * 1000);
    const year = d.getUTCFullYear();
    const oneJan = Date.UTC(year, 0, 1) / 1000;
    const dayOfYear = Math.floor((p.t - oneJan) / SECONDS_PER_DAY) + 1;
    const dow = (d.getUTCDay() + 6) % 7;
    const week = Math.ceil((dayOfYear + 6 - dow) / 7);
    const key = `${year}-W${week}`;
    if (key !== lastWeekKey) {
      if (lastPoint != null && lastWeekKey != null) out.push({ ts: lastPoint.t, close: lastPoint.v });
      lastWeekKey = key;
    }
    lastPoint = p;
  }
  if (lastPoint != null) out.push({ ts: lastPoint.t, close: lastPoint.v });
  return out;
}

export async function GET() {
  try {
    const startIso = new Date(Date.now() - 4 * 365 * 86_400_000)
      .toISOString()
      .slice(0, 10);

    // Fetch in parallel. If any source fails we still return a partial result
    // with the relevant input marked as "insufficient data" — same pattern as
    // /api/macro.
    const [fedfunds, m2, dxy, mvrvZ, btcRaw] = await Promise.all([
      fetchFredSeries("FEDFUNDS", startIso).catch(() => [] as DailyPoint[]),
      fetchFredSeries("WM2NS", startIso).catch(() => [] as DailyPoint[]),
      fetchFredSeries("DTWEXBGS", startIso).catch(() => [] as DailyPoint[]),
      fetchMvrvZ().catch(() => [] as DailyPoint[]),
      fetchBtcDailyHistory().catch(() => [] as DailyPoint[]),
    ]);

    const btcDailyClose: DailyPoint[] = toDailyPoints(btcRaw);
    const now = Math.floor(Date.now() / 1000);

    const sb = computeRegimeScore(
      { fedfunds, m2, mvrvZ, btcDailyClose, dxy },
      now,
    );

    // Per-input detail with raw values + rationale
    const fedNow = valueAtOrBefore(fedfunds, now);
    const fedPast = valueAtOrBefore(fedfunds, now - REGIME_SCORE_CONFIG.fed.lookbackMonths * 30 * SECONDS_PER_DAY);
    const m2Now = valueAtOrBefore(m2, now);
    const m2Past = valueAtOrBefore(m2, now - 365 * SECONDS_PER_DAY);
    const mvrvNow = valueAtOrBefore(mvrvZ, now);
    const dxyNow = valueAtOrBefore(dxy, now);
    const dxyPast = valueAtOrBefore(dxy, now - REGIME_SCORE_CONFIG.dxy.lookbackMonths * 30 * SECONDS_PER_DAY);

    const weekly = resampleDailyToWeekly(btcDailyClose);
    const weeklyCloses = weekly.map((w) => w.close);

    const inputs: RegimeScoreInputDetail[] = [
      {
        key: "fed",
        label: "Fed funds",
        source: "FRED · FEDFUNDS",
        rawValue: fedNow,
        rawAuxValue: fedPast,
        unit: "%",
        score: sb.fed,
        rationale: describeFed(fedNow, fedPast),
      },
      {
        key: "m2",
        label: "M2 YoY",
        source: "FRED · WM2NS",
        rawValue: m2Now,
        rawAuxValue: m2Past,
        unit: "$B",
        score: sb.m2,
        rationale: describeM2(m2Now, m2Past),
      },
      {
        key: "mvrv",
        label: "MVRV Z",
        source: "bitcoin-data.com",
        rawValue: mvrvNow,
        rawAuxValue: null,
        unit: "",
        score: sb.mvrv,
        rationale: describeMvrv(mvrvNow),
      },
      {
        key: "btc",
        label: "BTC vs 200w SMA",
        source: "CoinMetrics · resample",
        rawValue: weeklyCloses.length > 0 ? weeklyCloses[weeklyCloses.length - 1] : null,
        rawAuxValue: weeklyCloses.length >= REGIME_SCORE_CONFIG.btc.smaWeeks
          ? weeklyCloses.slice(-REGIME_SCORE_CONFIG.btc.smaWeeks).reduce((s, x) => s + x, 0) / REGIME_SCORE_CONFIG.btc.smaWeeks
          : null,
        unit: "$",
        score: sb.btc,
        rationale: describeBtc(weeklyCloses),
      },
      {
        key: "dxy",
        label: "USD 6m trend",
        source: "FRED · DTWEXBGS",
        rawValue: dxyNow,
        rawAuxValue: dxyPast,
        unit: "",
        score: sb.dxy,
        rationale: describeDxy(dxyNow, dxyPast),
      },
    ];

    // History: compute the score at each weekly boundary over the last ~2 years.
    const historyStart = now - 2 * 365 * SECONDS_PER_DAY;
    const weeklyTs = weekly
      .map((w) => w.ts)
      .filter((t) => t >= historyStart && t <= now);
    const history = weeklyTs.map((t) => {
      const s = computeRegimeScore(
        { fedfunds, m2, mvrvZ, btcDailyClose, dxy },
        t,
      );
      return { t, score: s.total, alloc: targetAllocationFromScore(s.total) };
    });

    const body: RegimeScoreResponse = {
      asOf: now,
      currentScore: sb.total,
      targetAllocPct: targetAllocationFromScore(sb.total) * 100,
      inputs,
      history,
      generatedAt: now,
    };
    return NextResponse.json(body);
  } catch (err) {
    return NextResponse.json(
      { error: err instanceof Error ? err.message : String(err) },
      { status: 502 },
    );
  }
}
