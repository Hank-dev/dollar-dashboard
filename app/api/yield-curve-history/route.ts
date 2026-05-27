import { NextResponse } from "next/server";

export const revalidate = 3600;

const FRED_URL = "https://api.stlouisfed.org/fred/series/observations";

const MATURITIES = [
  { label: "1M", series: "DGS1MO" },
  { label: "3M", series: "DGS3MO" },
  { label: "6M", series: "DGS6MO" },
  { label: "1Y", series: "DGS1" },
  { label: "2Y", series: "DGS2" },
  { label: "3Y", series: "DGS3" },
  { label: "5Y", series: "DGS5" },
  { label: "7Y", series: "DGS7" },
  { label: "10Y", series: "DGS10" },
  { label: "20Y", series: "DGS20" },
  { label: "30Y", series: "DGS30" },
];

async function fetchFredRecent(seriesId: string, limit: number): Promise<{ date: string; value: number }[]> {
  const key = process.env.FRED_API_KEY;
  if (!key) return [];
  const url = `${FRED_URL}?series_id=${seriesId}&api_key=${key}&file_type=json&sort_order=desc&limit=${limit}`;
  const res = await fetch(url, { next: { revalidate: 3600 } });
  if (!res.ok) return [];
  const json = (await res.json()) as { observations?: { date: string; value: string }[] };
  return (json.observations ?? [])
    .filter((o) => o.value && o.value !== ".")
    .map((o) => ({ date: o.date, value: Number(o.value) }))
    .filter((o) => Number.isFinite(o.value));
}

function pickByDate(observations: { date: string; value: number }[], targetDate: string): number | null {
  const exact = observations.find((o) => o.date === targetDate);
  if (exact) return exact.value;
  const sorted = [...observations].sort((a, b) => b.date.localeCompare(a.date));
  const closest = sorted.find((o) => o.date <= targetDate);
  return closest?.value ?? null;
}

export async function GET() {
  const today = new Date();
  const ago30 = new Date(Date.now() - 30 * 24 * 3600 * 1000);
  const ago90 = new Date(Date.now() - 90 * 24 * 3600 * 1000);

  const allObs = await Promise.all(
    MATURITIES.map(async (m) => ({
      label: m.label,
      observations: await fetchFredRecent(m.series, 100),
    })),
  );

  const todayStr = today.toISOString().slice(0, 10);
  const ago30Str = ago30.toISOString().slice(0, 10);
  const ago90Str = ago90.toISOString().slice(0, 10);

  const current: { m: string; y: number }[] = [];
  const past30: { m: string; y: number }[] = [];
  const past90: { m: string; y: number }[] = [];

  for (const { label, observations } of allObs) {
    const nowVal = pickByDate(observations, todayStr);
    const val30 = pickByDate(observations, ago30Str);
    const val90 = pickByDate(observations, ago90Str);
    if (nowVal != null) current.push({ m: label, y: nowVal });
    if (val30 != null) past30.push({ m: label, y: val30 });
    if (val90 != null) past90.push({ m: label, y: val90 });
  }

  return NextResponse.json({
    current,
    past30,
    past90,
    dates: { current: todayStr, past30: ago30Str, past90: ago90Str },
  });
}
