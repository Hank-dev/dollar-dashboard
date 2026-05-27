// FRED API. Free key is baked in so the deployed dashboard works without
// per-environment setup; override with FRED_API_KEY env var to rotate.
const BASE = "https://api.stlouisfed.org/fred";
const DEFAULT_KEY = "21de88f5e74f63ab29c0997e0a2d8584";

export type FredObs = { t: number; v: number };

type FredResponse = {
  observations: { date: string; value: string }[];
};

export async function fetchFredSeries(
  seriesId: string,
  startIso = "2018-01-01",
): Promise<FredObs[]> {
  const envKey = process.env.FRED_API_KEY;
  const key =
    envKey && envKey !== "REPLACE_ME" ? envKey : DEFAULT_KEY;
  const url =
    `${BASE}/series/observations?series_id=${encodeURIComponent(seriesId)}` +
    `&api_key=${key}&file_type=json&observation_start=${startIso}`;

  const res = await fetch(url, {
    next: { revalidate: 21_600 }, // 6h
    headers: { Accept: "application/json" },
  });
  if (!res.ok) {
    throw new Error(`FRED ${seriesId} ${res.status}: ${await res.text()}`);
  }
  const j = (await res.json()) as FredResponse;
  const out: FredObs[] = [];
  for (const obs of j.observations) {
    if (obs.value === "." || obs.value === "") continue;
    const v = parseFloat(obs.value);
    if (!Number.isFinite(v)) continue;
    out.push({ t: Math.floor(new Date(obs.date).getTime() / 1000), v });
  }
  return out;
}
