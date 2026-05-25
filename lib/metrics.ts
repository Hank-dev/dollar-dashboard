export const SNAPSHOT_DATE = "2026-05-22"; // close of session

export type Status = "calm" | "neutral" | "elevated" | "stressed";

export interface Metric {
  id: string;
  group: "rates" | "dollar" | "risk";
  label: string;
  value: string;        // display string, already formatted
  unit?: string;
  status: Status;
  context: string;      // short sub-line shown on the card
}

export const STATUS_COLOR: Record<Status, string> = {
  calm:     "#639922",
  neutral:  "#888780",
  elevated: "#EF9F27",
  stressed: "#E24B4A",
};

export const STATUS_LABEL: Record<Status, string> = {
  calm:     "Calm",
  neutral:  "Neutral",
  elevated: "Elevated",
  stressed: "Stressed",
};

export const GROUPS = [
  { id: "rates",  title: "US rates & fiscal stress", icon: "building-bank" },
  { id: "dollar", title: "Dollar & the yen carry trade", icon: "currency-dollar" },
  { id: "risk",   title: "Risk appetite & havens", icon: "activity" },
] as const;

export const METRICS: Metric[] = [
  // US rates & fiscal stress
  { id: "ust30",    group: "rates",  label: "30Y Treasury yield", value: "5.06%",       status: "stressed", context: "19-yr high · peak 5.20% on 19 May" },
  { id: "ust10",    group: "rates",  label: "10Y Treasury yield", value: "4.56%",       status: "elevated", context: "inflation-risk premium rising" },
  { id: "ust2",     group: "rates",  label: "2Y Treasury yield",  value: "4.13%",       status: "neutral",  context: "hike risk being priced back in" },
  { id: "fedfunds", group: "rates",  label: "Fed funds target",   value: "3.50–3.75%",  status: "neutral",  context: "on hold · FOMC split 8–4" },
  { id: "curve",    group: "rates",  label: "2s/10s curve",       value: "+43 bps",     status: "elevated", context: "bear steepening — long end leading" },
  // Dollar & the yen carry trade
  { id: "dxy",      group: "dollar", label: "Dollar index (DXY)", value: "99.3",        status: "neutral",  context: "mid 52-wk range 95.6–100.6" },
  { id: "usdjpy",   group: "dollar", label: "USD / JPY",          value: "159.2",       status: "elevated", context: "near 160 intervention zone" },
  { id: "jgb10",    group: "dollar", label: "Japan 10Y JGB",      value: "~2.8%",       status: "elevated", context: "highest since ~1997" },
  { id: "bojrate",  group: "dollar", label: "BOJ policy rate",    value: "0.75%",       status: "elevated", context: "→ 1.0% expected in June" },
  // Risk appetite & havens
  { id: "vix",      group: "risk",   label: "VIX (volatility)",   value: "16.7",        status: "calm",     context: "calm — diverges from bonds" },
  { id: "sp500",    group: "risk",   label: "S&P 500",            value: "7,473",       status: "calm",     context: "near record highs" },
  { id: "gold",     group: "risk",   label: "Gold",               value: "$4,523",      status: "elevated", context: "haven / debasement bid" },
  { id: "bitcoin",  group: "risk",   label: "Bitcoin",            value: "$74.6k",      status: "neutral",  context: "off recent highs" },
  { id: "brent",    group: "risk",   label: "Brent crude",        value: "~$100",       status: "elevated", context: "Iran-war risk premium" },
];

export const YIELD_CURVE = [
  { m: "2Y",  y: 4.13 },
  { m: "10Y", y: 4.56 },
  { m: "30Y", y: 5.06 },
];
