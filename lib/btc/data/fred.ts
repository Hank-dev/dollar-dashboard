// FRED API. Requires FRED_API_KEY env var (free from https://fred.stlouisfed.org/docs/api/api_key.html).
const BASE = "https://api.stlouisfed.org/fred";

export type FredObs = { t: number; v: number };

type FredResponse = {
  observations: { date: string; value: string }[];
};

function getFredKey(): string {
  const key = process.env.FRED_API_KEY;
  if (!key || key === "REPLACE_ME") {
    throw new Error(
      "FRED_API_KEY env var is required. Get a free key at https://fred.stlouisfed.org/docs/api/api_key.html",
    );
  }
  return key;
}

export async function fetchFredSeries(
  seriesId: string,
  startIso = "2018-01-01",
): Promise<FredObs[]> {
  const key = getFredKey();
  const url =
    `${BASE}/series/observations?series_id=${encodeURIComponent(seriesId)}` +
    `&api_key=${key}&file_type=json&observation_start=${startIso}`;

  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), 8000);
  try {
    const res = await fetch(url, {
      next: { revalidate: 21_600 }, // 6h
      headers: { Accept: "application/json" },
      signal: controller.signal,
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
  } finally {
    clearTimeout(timeout);
  }
}
