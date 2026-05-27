// Strategy D macro-regime composite score, ported from btc_bot's Python
// implementation (src/strategies/strategy_d.py). Same thresholds; same
// per-input scoring; same allocation table. Surfaced here as a dashboard
// indicator — informational, not a trading signal.
//
// 5 inputs, each scored -1 / 0 / +1, summed to S ∈ [-5, +5], mapped to a
// target BTC allocation:
//   S >= +3 → 100% long
//   S = +2  → 75%
//   S = +1  → 50%
//   S =  0  → 25%
//   S = -1  → 10%
//   S <= -2 → 0%
import type { DailyPoint } from "@/lib/btc/types";

const SECONDS_PER_DAY = 86_400;

// Locked thresholds — match strategy_d.yaml exactly. Don't tune from
// dashboard observation; that's the historicist trap (cf. btc_bot spec §5.4).
export const REGIME_SCORE_CONFIG = {
  fed: {
    lookbackMonths: 6,
    flatBandPctPoints: 0.25,
  },
  m2: {
    yoyUpperPct: 3.0,
    yoyLowerPct: -1.0,
  },
  mvrv: {
    lower: 2.0,
    upper: 5.0,
  },
  btc: {
    smaWeeks: 200,
    bandPct: 0.10,
  },
  dxy: {
    lookbackMonths: 6,
    flatBandPct: 2.0,
  },
} as const;

function monthsAgo(ts: number, months: number): number {
  // 30-day months for simplicity (matches strategy_d.py's _months_ago).
  return ts - 30 * months * SECONDS_PER_DAY;
}

function valueAtOrBefore(series: DailyPoint[], ts: number): number | null {
  if (series.length === 0) return null;
  // series is sorted ascending by t; binary search for the rightmost point ≤ ts.
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

export function scoreFed(fedfunds: DailyPoint[], ts: number): -1 | 0 | 1 {
  const now = valueAtOrBefore(fedfunds, ts);
  const past = valueAtOrBefore(fedfunds, monthsAgo(ts, REGIME_SCORE_CONFIG.fed.lookbackMonths));
  if (now == null || past == null) return 0;
  const diff = now - past;
  const band = REGIME_SCORE_CONFIG.fed.flatBandPctPoints;
  if (diff <= -band) return 1; // rate falling → accommodative
  if (diff >= band) return -1; // rate rising → tightening
  return 0;
}

export function scoreM2(m2: DailyPoint[], ts: number): -1 | 0 | 1 {
  const now = valueAtOrBefore(m2, ts);
  const past = valueAtOrBefore(m2, ts - 365 * SECONDS_PER_DAY);
  if (now == null || past == null || past === 0) return 0;
  const yoyPct = ((now - past) / past) * 100;
  if (yoyPct > REGIME_SCORE_CONFIG.m2.yoyUpperPct) return 1;
  if (yoyPct < REGIME_SCORE_CONFIG.m2.yoyLowerPct) return -1;
  return 0;
}

export function scoreMvrv(mvrvZ: DailyPoint[], ts: number): -1 | 0 | 1 {
  const v = valueAtOrBefore(mvrvZ, ts);
  if (v == null) return 0;
  if (v < REGIME_SCORE_CONFIG.mvrv.lower) return 1; // value zone
  if (v > REGIME_SCORE_CONFIG.mvrv.upper) return -1; // overheated
  return 0;
}

export function scoreBtc200w(btcDailyClose: DailyPoint[], ts: number): -1 | 0 | 1 {
  // Resample to weekly closes (last close per ISO week ≤ ts), take last 200w.
  const cutoff = ts;
  const weekly: number[] = [];
  let lastWeekKey: string | null = null;
  let lastValueInWeek: number | null = null;
  for (const p of btcDailyClose) {
    if (p.t > cutoff) break;
    const d = new Date(p.t * 1000);
    // ISO week key
    const year = d.getUTCFullYear();
    const oneJan = Date.UTC(year, 0, 1) / 1000;
    const dayOfYear = Math.floor((p.t - oneJan) / SECONDS_PER_DAY) + 1;
    const dow = (d.getUTCDay() + 6) % 7; // Mon=0..Sun=6
    const week = Math.ceil((dayOfYear + 6 - dow) / 7);
    const key = `${year}-W${week}`;
    if (key !== lastWeekKey) {
      if (lastValueInWeek != null) weekly.push(lastValueInWeek);
      lastWeekKey = key;
    }
    lastValueInWeek = p.v;
  }
  if (lastValueInWeek != null) weekly.push(lastValueInWeek);

  const need = REGIME_SCORE_CONFIG.btc.smaWeeks;
  if (weekly.length < need) return 0;
  const window = weekly.slice(-need);
  const sma = window.reduce((s, x) => s + x, 0) / need;
  const close = weekly[weekly.length - 1];
  if (sma <= 0 || !Number.isFinite(sma)) return 0;
  const band = REGIME_SCORE_CONFIG.btc.bandPct;
  if (close >= sma * (1 + band)) return 1;
  if (close <= sma * (1 - band)) return -1;
  return 0;
}

export function scoreDxy(dxy: DailyPoint[], ts: number): -1 | 0 | 1 {
  const now = valueAtOrBefore(dxy, ts);
  const past = valueAtOrBefore(dxy, monthsAgo(ts, REGIME_SCORE_CONFIG.dxy.lookbackMonths));
  if (now == null || past == null || past === 0) return 0;
  const chgPct = ((now - past) / past) * 100;
  const band = REGIME_SCORE_CONFIG.dxy.flatBandPct;
  if (chgPct < -band) return 1; // USD weakening → bullish BTC
  if (chgPct > band) return -1;
  return 0;
}

export type ScoreBreakdown = {
  fed: -1 | 0 | 1;
  m2: -1 | 0 | 1;
  mvrv: -1 | 0 | 1;
  btc: -1 | 0 | 1;
  dxy: -1 | 0 | 1;
  total: number; // -5..+5
};

export function computeRegimeScore(
  inputs: {
    fedfunds: DailyPoint[];
    m2: DailyPoint[];
    mvrvZ: DailyPoint[];
    btcDailyClose: DailyPoint[];
    dxy: DailyPoint[];
  },
  ts: number,
): ScoreBreakdown {
  const fed = scoreFed(inputs.fedfunds, ts);
  const m2 = scoreM2(inputs.m2, ts);
  const mvrv = scoreMvrv(inputs.mvrvZ, ts);
  const btc = scoreBtc200w(inputs.btcDailyClose, ts);
  const dxy = scoreDxy(inputs.dxy, ts);
  return { fed, m2, mvrv, btc, dxy, total: fed + m2 + mvrv + btc + dxy };
}

export function targetAllocationFromScore(s: number): number {
  if (s >= 3) return 1.0;
  if (s === 2) return 0.75;
  if (s === 1) return 0.5;
  if (s === 0) return 0.25;
  if (s === -1) return 0.1;
  return 0; // s <= -2
}
