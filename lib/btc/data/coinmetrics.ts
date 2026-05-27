// Coin Metrics community API (no key). 10 req/sec.
const BASE = "https://community-api.coinmetrics.io/v4";

type MetricsRow = Record<string, string | number | null> & {
  asset: string;
  time: string;
};
type MetricsResponse = {
  data: MetricsRow[];
  next_page_token?: string | null;
  next_page_url?: string | null;
};

export type CmDailyPoint = { t: number; v: number };

export async function fetchBtcMetrics(
  metrics: string[],
  startIso = "2010-07-17", // first day with usable data
): Promise<Record<string, CmDailyPoint[]>> {
  const out: Record<string, CmDailyPoint[]> = {};
  for (const m of metrics) out[m] = [];

  let url:
    | string
    | null = `${BASE}/timeseries/asset-metrics?assets=btc&metrics=${metrics.join(
    ",",
  )}&frequency=1d&start_time=${startIso}&page_size=10000`;

  let safety = 6; // pagination guard
  while (url && safety-- > 0) {
    const res: Response = await fetch(url, {
      next: { revalidate: 21_600 }, // 6h
      headers: { Accept: "application/json" },
    });
    if (!res.ok) {
      throw new Error(`CoinMetrics ${res.status}: ${await res.text()}`);
    }
    const j = (await res.json()) as MetricsResponse;
    for (const row of j.data) {
      const t = Math.floor(new Date(row.time).getTime() / 1000);
      for (const m of metrics) {
        const raw = row[m];
        if (raw == null) continue;
        const num = typeof raw === "number" ? raw : parseFloat(raw);
        if (!Number.isFinite(num)) continue;
        out[m].push({ t, v: num });
      }
    }
    url = j.next_page_url ?? null;
  }

  for (const m of metrics) out[m].sort((a, b) => a.t - b.t);
  return out;
}
