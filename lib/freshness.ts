// Shared "how stale is this?" helper, reused across dashboards.
export type Freshness = "fresh" | "aging" | "stale";

export interface FreshnessThresholds {
  agingDays?: number;
  staleDays?: number;
}

const MS_PER_DAY = 24 * 60 * 60 * 1000;
const ORDER: Freshness[] = ["fresh", "aging", "stale"];

export function daysSince(isoDate: string, now: Date = new Date()): number {
  const then = new Date(`${isoDate}T00:00:00Z`).getTime();
  if (!Number.isFinite(then)) return Number.POSITIVE_INFINITY;
  return Math.floor((now.getTime() - then) / MS_PER_DAY);
}

export function freshnessOf(
  isoDate: string,
  { agingDays = 30, staleDays = 90 }: FreshnessThresholds = {},
): Freshness {
  const age = daysSince(isoDate);
  if (age > staleDays) return "stale";
  if (age > agingDays) return "aging";
  return "fresh";
}

// Worst (most stale) value wins — for an at-a-glance overall indicator.
export function overallFreshness(
  isoDates: string[],
  thresholds?: FreshnessThresholds,
): Freshness {
  return isoDates.reduce<Freshness>((worst, date) => {
    const f = freshnessOf(date, thresholds);
    return ORDER.indexOf(f) > ORDER.indexOf(worst) ? f : worst;
  }, "fresh");
}
