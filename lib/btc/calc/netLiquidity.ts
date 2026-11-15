import type { FredObs } from "@/lib/btc/data/fred";
import type { DailyPoint } from "@/lib/btc/types";
import { SECONDS_PER_DAY } from "./util";

/**
 * Net Liquidity = WALCL - WTREGEN - RRPONTSYD
 * - WALCL and WTREGEN are weekly, in millions USD. Forward-fill to daily.
 * - RRPONTSYD is daily (business days), already in billions USD.
 * - Return billions USD only once all three components have an observation.
 *   An unavailable component must not be interpreted as a zero balance.
 */
export function computeNetLiquidity(
  walcl: FredObs[],
  tga: FredObs[],
  rrp: FredObs[],
): DailyPoint[] {
  if (walcl.length === 0 || tga.length === 0 || rrp.length === 0) return [];

  const start = Math.max(walcl[0].t, tga[0].t, rrp[0].t);
  const end = Math.max(
    walcl[walcl.length - 1].t,
    tga[tga.length - 1].t,
    rrp[rrp.length - 1].t,
  );
  if (!Number.isFinite(start) || !Number.isFinite(end)) return [];

  const fill = (s: FredObs[]): Map<number, number> => {
    const m = new Map<number, number>();
    let last: number | null = null;
    let si = 0;
    for (let t = start; t <= end; t += SECONDS_PER_DAY) {
      while (si < s.length && s[si].t <= t) {
        last = s[si].v;
        si++;
      }
      if (last != null) m.set(t, last);
    }
    return m;
  };

  const wMap = fill(walcl);
  const tMap = fill(tga);
  const rMap = fill(rrp);

  const out: DailyPoint[] = [];
  for (let t = start; t <= end; t += SECONDS_PER_DAY) {
    const w = wMap.get(t);
    const tg = tMap.get(t);
    const r = rMap.get(t);
    if (w == null || tg == null || r == null) continue;
    const value = (w - tg) / 1000 - r;
    if (Number.isFinite(value)) out.push({ t, v: value });
  }
  return out;
}
