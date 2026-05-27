import type { FredObs } from "@/lib/btc/data/fred";
import type { DailyPoint } from "@/lib/btc/types";
import { SECONDS_PER_DAY } from "./util";

/**
 * Net Liquidity = WALCL - WTREGEN - RRPONTSYD
 * - WALCL is weekly (Wednesdays). Forward-fill to daily.
 * - WTREGEN and RRPONTSYD are daily (business days). Forward-fill weekends.
 * - All FRED series report in millions USD; convert to billions for display.
 */
export function computeNetLiquidity(
  walcl: FredObs[],
  tga: FredObs[],
  rrp: FredObs[],
): DailyPoint[] {
  if (walcl.length === 0) return [];

  const start = Math.min(walcl[0].t, tga[0]?.t ?? Infinity, rrp[0]?.t ?? Infinity);
  const end = Math.max(
    walcl[walcl.length - 1].t,
    tga[tga.length - 1]?.t ?? -Infinity,
    rrp[rrp.length - 1]?.t ?? -Infinity,
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
    if (w == null) continue;
    // Treat missing TGA/RRP as 0 (very early data); after 2018 all three exist.
    const value = w - (tg ?? 0) - (r ?? 0);
    out.push({ t, v: value / 1000 }); // millions -> billions
  }
  return out;
}
