// Correlation utilities. Log returns + Pearson + rolling-window correlation.

export function logReturns(prices: number[]): number[] {
  const out: number[] = [];
  for (let i = 1; i < prices.length; i++) {
    const a = prices[i - 1];
    const b = prices[i];
    if (a > 0 && b > 0 && Number.isFinite(a) && Number.isFinite(b)) {
      out.push(Math.log(b / a));
    } else {
      out.push(0);
    }
  }
  return out;
}

export function pearson(a: number[], b: number[]): number {
  const n = a.length;
  if (n < 2 || n !== b.length) return NaN;
  let sumA = 0;
  let sumB = 0;
  for (let i = 0; i < n; i++) {
    sumA += a[i];
    sumB += b[i];
  }
  const meanA = sumA / n;
  const meanB = sumB / n;
  let num = 0;
  let vA = 0;
  let vB = 0;
  for (let i = 0; i < n; i++) {
    const da = a[i] - meanA;
    const db = b[i] - meanB;
    num += da * db;
    vA += da * da;
    vB += db * db;
  }
  const den = Math.sqrt(vA * vB);
  if (den === 0 || !Number.isFinite(den)) return 0;
  return num / den;
}

// Returns array of length a.length; entries before index (window-1) are NaN.
export function rollingCorrelation(
  a: number[],
  b: number[],
  window: number,
): number[] {
  const n = a.length;
  if (n !== b.length) throw new Error("rolling: length mismatch");
  const out = new Array<number>(n).fill(NaN);
  if (n < window || window < 2) return out;
  for (let i = window - 1; i < n; i++) {
    out[i] = pearson(
      a.slice(i - window + 1, i + 1),
      b.slice(i - window + 1, i + 1),
    );
  }
  return out;
}

// Intersect two daily time series by day-bucket (UTC midnight), returning the
// shared dates and the two aligned value arrays. BTC trades 7d/wk; FRED/equity
// series trade 5d/wk — we keep only days both have an observation.
export function alignByDay<
  A extends { t: number; v: number },
  B extends { t: number; v: number },
>(a: A[], b: B[]): { t: number[]; a: number[]; b: number[] } {
  const day = (t: number) => Math.floor(t / 86_400) * 86_400;
  const aMap = new Map<number, number>();
  for (const p of a) aMap.set(day(p.t), p.v);
  const t: number[] = [];
  const av: number[] = [];
  const bv: number[] = [];
  for (const p of b) {
    const d = day(p.t);
    const av0 = aMap.get(d);
    if (av0 == null) continue;
    t.push(d);
    av.push(av0);
    bv.push(p.v);
  }
  return { t, a: av, b: bv };
}
