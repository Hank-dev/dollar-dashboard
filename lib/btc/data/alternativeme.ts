// Fear & Greed Index — free, no key.
const BASE = "https://api.alternative.me/fng";

type FngApi = {
  data: {
    value: string;
    value_classification: string;
    timestamp: string; // unix seconds
  }[];
};

export type FngRow = { t: number; v: number; label: string };

export async function fetchFng(limit = 365): Promise<FngRow[]> {
  const res = await fetch(`${BASE}/?limit=${limit}&format=json`, {
    next: { revalidate: 3600 },
    headers: { Accept: "application/json" },
  });
  if (!res.ok) throw new Error(`F&G ${res.status}: ${await res.text()}`);
  const j = (await res.json()) as FngApi;
  return j.data
    .map((row) => ({
      t: parseInt(row.timestamp, 10),
      v: parseInt(row.value, 10),
      label: row.value_classification,
    }))
    .sort((a, b) => a.t - b.t);
}
