// Market Emotion Indicator (MEI) — lookahead-safe regime classifier.
//
// TypeScript port of btc_bot/src/indicators/market_emotion.py. The dashboard
// runs without full OHLC history (CoinMetrics gives close-only, Binance OHLC
// is capped at ~1000d), so the "range" arousal feature is approximated by
// |daily return| — close-only TR proxy that captures the same arousal signal.
//
// Caveats (spec §13):
//   Panic mean-reverts more reliably than Euphoria — the four corners are NOT
//   symmetric for strategy purposes. Apathy is by definition the absence of
//   edge. This is a descriptive regime classifier, not a timing oracle.

export type MeiInputRow = {
  t: number; // unix seconds, UTC midnight
  close: number;
  volume?: number | null; // daily USD volume; may be missing pre-Binance window
  funding?: number | null; // per-bar mean realized funding rate
};

export type MeiBar = {
  t: number;
  close: number;
  valence: number | null;
  arousal: number | null;
  valenceRaw: number | null;
  arousalRaw: number | null;
  state: RegimeState;
  intensity: number | null;
  isWarmup: boolean;
};

export const REGIMES = [
  "Panic",
  "Volatile",
  "Euphoria",
  "Anxiety",
  "Neutral",
  "Optimism",
  "Despondency",
  "Apathy",
  "Complacency",
] as const;
export type Regime = (typeof REGIMES)[number];
export type RegimeState = Regime | "warmup";

export const REGIME_COLORS: Record<RegimeState, string> = {
  Panic: "#7a0010",
  Anxiety: "#c0392b",
  Despondency: "#5d4037",
  Volatile: "#8e44ad",
  Neutral: "#7f8c8d",
  Apathy: "#bdc3c7",
  Euphoria: "#f1c40f",
  Optimism: "#27ae60",
  Complacency: "#85c1e9",
  warmup: "#1c2833",
};

// (valence_band, arousal_band) -> regime, with bands: 0=Bearish/Low, 1=Mid/Neutral, 2=Bullish/High.
const REGIME_GRID: Record<string, Regime> = {
  "0,2": "Panic",
  "1,2": "Volatile",
  "2,2": "Euphoria",
  "0,1": "Anxiety",
  "1,1": "Neutral",
  "2,1": "Optimism",
  "0,0": "Despondency",
  "1,0": "Apathy",
  "2,0": "Complacency",
};

// Features that may be sparse on the dashboard timeline. Per spec §4 they
// are dropped per-bar (weights renormalized) rather than NaNing the composite.
// `volume` is here because Binance daily klines only go back ~1000 days; we
// still want regime classification across the full BTC history.
const OPTIONAL_FEATURES = new Set([
  "volume",
  "funding_signed",
  "funding_abs",
]);

export type MeiConfig = {
  normWindow: number;
  trendWindow: number;
  momWindow: number;
  volWindow: number;
  smoothSpan: number;
  useFunding: boolean;
  valenceBear: number;
  valenceBull: number;
  arousalLow: number;
  arousalHigh: number;
  valenceWeights: Record<string, number>;
  arousalWeights: Record<string, number>;
};

export const DEFAULT_MEI_CONFIG: MeiConfig = {
  normWindow: 504,
  trendWindow: 200,
  momWindow: 60,
  volWindow: 20,
  smoothSpan: 3,
  useFunding: false,
  valenceBear: 40,
  valenceBull: 60,
  arousalLow: 35,
  arousalHigh: 65,
  valenceWeights: { trend: 0.35, momentum: 0.3, drawdown: 0.2 },
  arousalWeights: { rvol: 0.4, volume: 0.3, range: 0.3 },
};

// ---- causal rolling helpers ---------------------------------------------

function rollingMean(xs: (number | null)[], window: number): (number | null)[] {
  const out = new Array<number | null>(xs.length).fill(null);
  let sum = 0;
  let count = 0;
  for (let i = 0; i < xs.length; i++) {
    const v = xs[i];
    if (v != null && Number.isFinite(v)) {
      sum += v;
      count += 1;
    }
    if (i >= window) {
      const old = xs[i - window];
      if (old != null && Number.isFinite(old)) {
        sum -= old;
        count -= 1;
      }
    }
    if (i >= window - 1 && count === window) {
      out[i] = sum / window;
    }
  }
  return out;
}

function rollingStd(xs: (number | null)[], window: number): (number | null)[] {
  // Two-pass per window: O(n*w). n is ~5800 (15y daily), w=20 — fast enough.
  const out = new Array<number | null>(xs.length).fill(null);
  for (let i = window - 1; i < xs.length; i++) {
    let sum = 0;
    let nValid = 0;
    for (let j = i - window + 1; j <= i; j++) {
      const v = xs[j];
      if (v == null || !Number.isFinite(v)) {
        nValid = -1;
        break;
      }
      sum += v;
      nValid += 1;
    }
    if (nValid !== window) continue;
    const mu = sum / window;
    let sq = 0;
    for (let j = i - window + 1; j <= i; j++) {
      const d = (xs[j] as number) - mu;
      sq += d * d;
    }
    // Sample stddev (ddof=1) — matches pandas .std() default.
    out[i] = Math.sqrt(sq / (window - 1));
  }
  return out;
}

function rollingMax(xs: number[], window: number): number[] {
  // min_periods=1 trailing max (used for drawdown denominator).
  const out = new Array<number>(xs.length).fill(NaN);
  for (let i = 0; i < xs.length; i++) {
    const start = Math.max(0, i - window + 1);
    let m = -Infinity;
    for (let j = start; j <= i; j++) if (xs[j] > m) m = xs[j];
    out[i] = m;
  }
  return out;
}

function rollingMedian(
  xs: (number | null)[],
  window: number,
): (number | null)[] {
  const out = new Array<number | null>(xs.length).fill(null);
  for (let i = window - 1; i < xs.length; i++) {
    const buf: number[] = [];
    for (let j = i - window + 1; j <= i; j++) {
      const v = xs[j];
      if (v != null && Number.isFinite(v)) buf.push(v);
    }
    if (buf.length !== window) continue;
    buf.sort((a, b) => a - b);
    const mid = window >> 1;
    out[i] = window % 2 === 1 ? buf[mid] : (buf[mid - 1] + buf[mid]) / 2;
  }
  return out;
}

/**
 * Trailing percentile rank (0–100) of each value within its rolling window.
 * Uses only data up to and including index t. Bars where the window contains
 * any NaN are emitted as null. Ties: average rank (matches pandas `pct=True`).
 */
export function rollingPct(
  xs: (number | null)[],
  window: number,
): (number | null)[] {
  const out = new Array<number | null>(xs.length).fill(null);
  for (let i = window - 1; i < xs.length; i++) {
    const buf: number[] = [];
    for (let j = i - window + 1; j <= i; j++) {
      const v = xs[j];
      if (v == null || !Number.isFinite(v)) {
        buf.length = 0;
        break;
      }
      buf.push(v);
    }
    if (buf.length !== window) continue;
    const target = buf[buf.length - 1];
    // Average rank — pandas default for ties under `pct=True`.
    let less = 0;
    let equal = 0;
    for (const x of buf) {
      if (x < target) less += 1;
      else if (x === target) equal += 1;
    }
    const avgRank = less + (equal + 1) / 2;
    out[i] = (avgRank / window) * 100;
  }
  return out;
}

function ewmSpan(xs: (number | null)[], span: number): (number | null)[] {
  if (span <= 1) return xs.slice();
  const alpha = 2 / (span + 1);
  const out = new Array<number | null>(xs.length).fill(null);
  let prev: number | null = null;
  for (let i = 0; i < xs.length; i++) {
    const v = xs[i];
    if (v == null || !Number.isFinite(v)) {
      // Don't propagate EMA across a warmup gap — keep aligned with the raw series.
      prev = null;
      out[i] = null;
      continue;
    }
    prev = prev == null ? v : prev + alpha * (v - prev);
    out[i] = prev;
  }
  return out;
}

// ---- composite ----------------------------------------------------------

function weightedComposite(
  features: Record<string, (number | null)[]>,
  weights: Record<string, number>,
  length: number,
): (number | null)[] {
  const available = Object.keys(weights).filter((k) => k in features);
  if (available.length === 0) return new Array<number | null>(length).fill(null);

  const coreCols = available.filter((c) => !OPTIONAL_FEATURES.has(c));

  const out = new Array<number | null>(length).fill(null);
  for (let i = 0; i < length; i++) {
    let coreOk = true;
    for (const c of coreCols) {
      if (features[c][i] == null) {
        coreOk = false;
        break;
      }
    }
    if (!coreOk) continue;

    let sumW = 0;
    let sumWV = 0;
    for (const c of available) {
      const v = features[c][i];
      if (v == null) continue;
      const w = weights[c];
      sumW += w;
      sumWV += w * v;
    }
    out[i] = sumW > 0 ? sumWV / sumW : null;
  }
  return out;
}

// ---- main ---------------------------------------------------------------

export function computeMEI(
  rows: MeiInputRow[],
  cfg: MeiConfig = DEFAULT_MEI_CONFIG,
): MeiBar[] {
  const n = rows.length;
  const closes = rows.map((r) => r.close);
  const volumes = rows.map((r) => (r.volume == null ? null : r.volume));
  const fundings = rows.map((r) => (r.funding == null ? null : r.funding));

  // --- valence raw features ---
  const trendSma = rollingMean(closes, cfg.trendWindow);
  const trend: (number | null)[] = closes.map((c, i) => {
    const s = trendSma[i];
    return s == null || s === 0 ? null : c / s - 1;
  });

  const momentum: (number | null)[] = closes.map((c, i) => {
    if (i < cfg.momWindow) return null;
    const ref = closes[i - cfg.momWindow];
    return ref === 0 ? null : c / ref - 1;
  });

  const cummax = rollingMax(closes, cfg.normWindow);
  const drawdown: (number | null)[] = closes.map((c, i) => {
    const m = cummax[i];
    return !Number.isFinite(m) || m === 0 ? null : c / m - 1;
  });

  // --- arousal raw features ---
  const returns: (number | null)[] = closes.map((c, i) => {
    if (i === 0) return null;
    const p = closes[i - 1];
    return p === 0 ? null : c / p - 1;
  });
  const rvolRaw = rollingStd(returns, cfg.volWindow);
  const SQRT_252 = Math.sqrt(252);
  const rvol: (number | null)[] = rvolRaw.map((v) =>
    v == null ? null : v * SQRT_252,
  );

  const volumeSma = rollingMean(volumes, cfg.volWindow);
  const volumeRatio: (number | null)[] = volumes.map((v, i) => {
    const s = volumeSma[i];
    return v == null || s == null || s === 0 ? null : v / s;
  });

  // Range proxy without OHLC: |return|. True ATR/close would be ideal but
  // OHLC isn't available for the full history.
  const absReturns: (number | null)[] = returns.map((r) =>
    r == null ? null : Math.abs(r),
  );
  const rangeFeat = rollingMean(absReturns, cfg.volWindow);

  // --- optional funding features ---
  let fundingSigned: (number | null)[] | null = null;
  let fundingAbs: (number | null)[] | null = null;
  if (cfg.useFunding) {
    fundingSigned = fundings.slice();
    const baseline = rollingMedian(fundings, cfg.normWindow);
    fundingAbs = fundings.map((f, i) => {
      const b = baseline[i];
      return f == null || b == null ? null : Math.abs(f - b);
    });
  }

  // --- rolling percentile ranks ---
  const W = cfg.normWindow;
  const valFeats: Record<string, (number | null)[]> = {
    trend: rollingPct(trend, W),
    momentum: rollingPct(momentum, W),
    drawdown: rollingPct(drawdown, W),
  };
  const aroFeats: Record<string, (number | null)[]> = {
    rvol: rollingPct(rvol, W),
    volume: rollingPct(volumeRatio, W),
    range: rollingPct(rangeFeat, W),
  };
  if (cfg.useFunding && fundingSigned) valFeats.funding_signed = rollingPct(fundingSigned, W);
  if (cfg.useFunding && fundingAbs) aroFeats.funding_abs = rollingPct(fundingAbs, W);

  // --- composites + smoothing ---
  const valenceRaw = weightedComposite(valFeats, cfg.valenceWeights, n);
  const arousalRaw = weightedComposite(aroFeats, cfg.arousalWeights, n);

  const valenceSmooth = ewmSpan(valenceRaw, cfg.smoothSpan);
  const arousalSmooth = ewmSpan(arousalRaw, cfg.smoothSpan);

  // --- classify ---
  const out: MeiBar[] = new Array(n);
  for (let i = 0; i < n; i++) {
    const v = valenceSmooth[i];
    const a = arousalSmooth[i];
    let state: RegimeState = "warmup";
    let intensity: number | null = null;
    if (v != null && a != null) {
      const vb = v < cfg.valenceBear ? 0 : v > cfg.valenceBull ? 2 : 1;
      const ab = a < cfg.arousalLow ? 0 : a > cfg.arousalHigh ? 2 : 1;
      state = REGIME_GRID[`${vb},${ab}`];
      intensity = Math.min(
        1,
        Math.max(Math.abs(v - 50), Math.abs(a - 50)) / 50,
      );
    }
    out[i] = {
      t: rows[i].t,
      close: rows[i].close,
      valence: v,
      arousal: a,
      valenceRaw: valenceRaw[i],
      arousalRaw: arousalRaw[i],
      state,
      intensity,
      isWarmup: valenceRaw[i] == null || arousalRaw[i] == null,
    };
  }
  return out;
}

export function regimeDistribution(bars: MeiBar[]): Record<RegimeState, number> {
  const out = Object.fromEntries(
    [...REGIMES, "warmup" as const].map((r) => [r, 0]),
  ) as Record<RegimeState, number>;
  for (const b of bars) out[b.state] += 1;
  return out;
}
