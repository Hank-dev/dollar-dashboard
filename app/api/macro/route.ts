import { NextResponse } from "next/server";
import { fetchFredSeries } from "@/lib/btc/data/fred";
import { fetchYahooDaily } from "@/lib/btc/data/yahoo";
import { SECONDS_PER_DAY } from "@/lib/btc/calc/util";
import type { MacroIndicator, MacroKey, MacroResponse } from "@/lib/btc/types";

export const revalidate = 21600;

type Series = { t: number; v: number }[];

function summarise(
  key: MacroKey,
  label: string,
  source: string,
  unit: string,
  series: Series,
): MacroIndicator {
  if (series.length === 0) {
    return {
      key,
      label,
      source,
      unit,
      current: null,
      prev30d: null,
      delta30d: null,
      asOf: null,
      error: "no data",
    };
  }
  const latest = series[series.length - 1];
  const cutoff = latest.t - 30 * SECONDS_PER_DAY;
  let prev: number | null = null;
  for (let i = series.length - 1; i >= 0; i--) {
    if (series[i].t <= cutoff) {
      prev = series[i].v;
      break;
    }
  }
  if (prev == null && series.length > 1) prev = series[0].v;
  return {
    key,
    label,
    source,
    unit,
    current: latest.v,
    prev30d: prev,
    delta30d: prev != null ? latest.v - prev : null,
    asOf: latest.t,
  };
}

async function safe<T>(
  task: () => Promise<T>,
  onErr: (msg: string) => MacroIndicator,
): Promise<MacroIndicator | T> {
  try {
    return await task();
  } catch (e) {
    return onErr(e instanceof Error ? e.message : String(e));
  }
}

export async function GET() {
  const start = new Date(Date.now() - 90 * 24 * 3600 * 1000)
    .toISOString()
    .slice(0, 10);

  const [dxy, dfii10, hyspread, vix, move] = await Promise.all([
    safe(
      async () =>
        summarise(
          "dxy",
          "USD INDEX",
          "FRED · DTWEXBGS",
          "",
          await fetchFredSeries("DTWEXBGS", start),
        ),
      (err) => ({
        key: "dxy",
        label: "USD INDEX",
        source: "FRED · DTWEXBGS",
        unit: "",
        current: null,
        prev30d: null,
        delta30d: null,
        asOf: null,
        error: err,
      }),
    ),
    safe(
      async () =>
        summarise(
          "dfii10",
          "10Y Real Yield",
          "FRED · DFII10",
          "%",
          await fetchFredSeries("DFII10", start),
        ),
      (err) => ({
        key: "dfii10",
        label: "10Y Real Yield",
        source: "FRED · DFII10",
        unit: "%",
        current: null,
        prev30d: null,
        delta30d: null,
        asOf: null,
        error: err,
      }),
    ),
    safe(
      async () =>
        summarise(
          "hyspread",
          "HY Spread",
          "FRED · BAMLH0A0HYM2",
          "%",
          await fetchFredSeries("BAMLH0A0HYM2", start),
        ),
      (err) => ({
        key: "hyspread",
        label: "HY Spread",
        source: "FRED · BAMLH0A0HYM2",
        unit: "%",
        current: null,
        prev30d: null,
        delta30d: null,
        asOf: null,
        error: err,
      }),
    ),
    safe(
      async () =>
        summarise(
          "vix",
          "VIX",
          "FRED · VIXCLS",
          "",
          await fetchFredSeries("VIXCLS", start),
        ),
      (err) => ({
        key: "vix",
        label: "VIX",
        source: "FRED · VIXCLS",
        unit: "",
        current: null,
        prev30d: null,
        delta30d: null,
        asOf: null,
        error: err,
      }),
    ),
    safe(
      async () =>
        summarise(
          "move",
          "MOVE",
          "Yahoo",
          "",
          await fetchYahooDaily("^MOVE", "3mo"),
        ),
      (err) => ({
        key: "move",
        label: "MOVE",
        source: "Yahoo",
        unit: "",
        current: null,
        prev30d: null,
        delta30d: null,
        asOf: null,
        error: err,
      }),
    ),
  ]);

  const body: MacroResponse = {
    indicators: [dxy, dfii10, hyspread, vix, move] as MacroIndicator[],
    generatedAt: Math.floor(Date.now() / 1000),
  };
  return NextResponse.json(body);
}
