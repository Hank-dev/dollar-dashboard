import type { DailyPoint } from "@/lib/btc/types";
import { GENESIS_UNIX, SECONDS_PER_DAY, stddev } from "./util";

export type PowerLawFit = {
  a: number; // intercept (log10 space)
  b: number; // slope (log10 space)
  sigma: number; // stddev of residuals in log10 space
};

/**
 * Fit log10(price) = a + b * log10(days_since_genesis).
 * Drops points before 2010-07-17 (illiquid early history).
 */
export function fitPowerLaw(series: DailyPoint[]): PowerLawFit {
  const cutoff = Date.UTC(2010, 6, 17) / 1000;
  const xs: number[] = [];
  const ys: number[] = [];
  for (const p of series) {
    if (p.t < cutoff) continue;
    if (!Number.isFinite(p.v) || p.v <= 0) continue;
    const days = (p.t - GENESIS_UNIX) / SECONDS_PER_DAY;
    if (days <= 0) continue;
    xs.push(Math.log10(days));
    ys.push(Math.log10(p.v));
  }
  const n = xs.length;
  if (n < 100) throw new Error("Not enough data points to fit power law");

  let sx = 0,
    sy = 0,
    sxx = 0,
    sxy = 0;
  for (let i = 0; i < n; i++) {
    sx += xs[i];
    sy += ys[i];
    sxx += xs[i] * xs[i];
    sxy += xs[i] * ys[i];
  }
  const meanX = sx / n;
  const meanY = sy / n;
  const b = (sxy - n * meanX * meanY) / (sxx - n * meanX * meanX);
  const a = meanY - b * meanX;

  const residuals = ys.map((y, i) => y - (a + b * xs[i]));
  const sigma = stddev(residuals);

  return { a, b, sigma };
}

export function powerLawValue(fit: PowerLawFit, unixSec: number): number {
  const days = (unixSec - GENESIS_UNIX) / SECONDS_PER_DAY;
  if (days <= 0) return NaN;
  const logVal = fit.a + fit.b * Math.log10(days);
  return Math.pow(10, logVal);
}

export function powerLawBand(
  fit: PowerLawFit,
  unixSec: number,
  sigmas: number,
): number {
  const days = (unixSec - GENESIS_UNIX) / SECONDS_PER_DAY;
  if (days <= 0) return NaN;
  const logVal = fit.a + fit.b * Math.log10(days) + sigmas * fit.sigma;
  return Math.pow(10, logVal);
}

export function powerLawZ(
  fit: PowerLawFit,
  unixSec: number,
  price: number,
): number {
  const days = (unixSec - GENESIS_UNIX) / SECONDS_PER_DAY;
  if (days <= 0 || price <= 0) return NaN;
  const expected = fit.a + fit.b * Math.log10(days);
  return (Math.log10(price) - expected) / fit.sigma;
}
