// CNBC daily chart bars. No key. Used for ICE DXY history: FRED's DTWEXBGS is
// the broad trade-weighted index (~120), a different series from DXY (~100),
// and Yahoo rate-limits server IPs.
export type CnbcPoint = { t: number; v: number };

type CnbcResponse = {
  barData?: { priceBars?: { close?: string | number; tradeTimeinMills?: string | number }[] };
};

const SECONDS_PER_DAY = 86_400;
const CLIENT_NAME = "Mozilla/5.0 (compatible; market-monitor/0.1)";

/** One close per UTC day, oldest first. Same reading the desktop client uses. */
export function parseCnbcBars(body: CnbcResponse): CnbcPoint[] {
  const out: CnbcPoint[] = [];
  for (const row of body.barData?.priceBars ?? []) {
    const ms = Number(row.tradeTimeinMills);
    const close = Number(row.close);
    if (!Number.isFinite(ms) || !Number.isFinite(close) || close <= 0) continue;
    const t = Math.floor(ms / 1000 / SECONDS_PER_DAY) * SECONDS_PER_DAY;
    const last = out[out.length - 1];
    if (last && last.t === t) last.v = close;
    else out.push({ t, v: close });
  }
  return out;
}

export async function fetchCnbcDaily(symbol: string): Promise<CnbcPoint[]> {
  const url = `https://ts-api.cnbc.com/harmony/app/charts/1Y.json?symbol=${encodeURIComponent(symbol)}`;
  const res = await fetch(url, {
    cache: "no-store",
    headers: { Accept: "application/json", "User-Agent": CLIENT_NAME },
    signal: AbortSignal.timeout(8000),
  });
  if (!res.ok) throw new Error(`CNBC ${symbol} ${res.status}`);
  const points = parseCnbcBars((await res.json()) as CnbcResponse);
  if (points.length === 0) throw new Error(`CNBC ${symbol}: no bars`);
  return points;
}

/** ICE US Dollar Index daily closes for the trailing year. */
export function fetchDxyDaily(): Promise<CnbcPoint[]> {
  return fetchCnbcDaily(".DXY");
}
