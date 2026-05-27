import type { DailyPoint } from "@/lib/btc/types";

export const GENESIS_UNIX = 1230940800; // 2009-01-03 UTC
export const SECONDS_PER_DAY = 86_400;

// Most recent halving: 2024-04-20 (block 840000).
export const LAST_HALVING_UNIX = 1713571200;

export function daysSince(unixSec: number, fromUnixSec: number): number {
  return Math.floor((unixSec - fromUnixSec) / SECONDS_PER_DAY);
}

// Trailing simple moving average. Returns NaN for indices < period-1.
export function sma(values: number[], period: number): number[] {
  const out = new Array(values.length).fill(NaN);
  let sum = 0;
  for (let i = 0; i < values.length; i++) {
    sum += values[i];
    if (i >= period) sum -= values[i - period];
    if (i >= period - 1) out[i] = sum / period;
  }
  return out;
}

export function mean(xs: number[]): number {
  let s = 0;
  for (const x of xs) s += x;
  return s / xs.length;
}

export function stddev(xs: number[], mu?: number): number {
  const m = mu ?? mean(xs);
  let acc = 0;
  for (const x of xs) acc += (x - m) ** 2;
  return Math.sqrt(acc / xs.length);
}

// Align series B to series A by timestamp, forward-filling B's last known value.
// Both must be sorted by t ascending.
export function alignForwardFill(
  a: DailyPoint[],
  b: DailyPoint[],
): { t: number; a: number; b: number | null }[] {
  const out: { t: number; a: number; b: number | null }[] = [];
  let bi = 0;
  let last: number | null = null;
  for (const pa of a) {
    while (bi < b.length && b[bi].t <= pa.t) {
      last = b[bi].v;
      bi++;
    }
    out.push({ t: pa.t, a: pa.v, b: last });
  }
  return out;
}

// Build a daily timeline between two unix seconds (inclusive both ends).
export function dailyTimeline(startSec: number, endSec: number): number[] {
  const startDay = Math.floor(startSec / SECONDS_PER_DAY) * SECONDS_PER_DAY;
  const endDay = Math.floor(endSec / SECONDS_PER_DAY) * SECONDS_PER_DAY;
  const out: number[] = [];
  for (let t = startDay; t <= endDay; t += SECONDS_PER_DAY) out.push(t);
  return out;
}
