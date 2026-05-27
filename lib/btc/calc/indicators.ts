export function mayerMultiple(price: number, sma200: number): number | null {
  if (!Number.isFinite(price) || !Number.isFinite(sma200) || sma200 <= 0)
    return null;
  return price / sma200;
}
