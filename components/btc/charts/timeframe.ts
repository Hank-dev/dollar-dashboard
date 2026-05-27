export const TIMEFRAMES = ["1Y", "2Y", "5Y", "ALL"] as const;
export type Timeframe = (typeof TIMEFRAMES)[number];

const SECONDS_PER_DAY = 86400;

export function timeframeCutoff(tf: Timeframe): number | null {
  if (tf === "ALL") return null;
  const now = Math.floor(Date.now() / 1000);
  const days = tf === "1Y" ? 365 : tf === "2Y" ? 365 * 2 : 365 * 5;
  return now - days * SECONDS_PER_DAY;
}
