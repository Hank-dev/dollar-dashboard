import type { RegimeState } from "@/lib/btc/calc/marketEmotion";

export type DailyPoint = {
  t: number; // unix seconds, UTC midnight
  v: number;
};

export type PriceHistoryResponse = {
  series: DailyPoint[]; // daily close, full history
  fit: DailyPoint[]; // power-law fit value at each day
  upper1: DailyPoint[];
  lower1: DailyPoint[];
  upper2: DailyPoint[];
  lower2: DailyPoint[];
  sma200: DailyPoint[];
  volume: DailyPoint[]; // daily USD volume (Binance USDT proxy), ~last 1000d
  ath: { t: number; v: number };
  daysSinceAth: number;
  daysSinceLastHalving: number;
  powerLaw: { a: number; b: number; sigma: number };
  generatedAt: number;
};

export type RegimeResponse = {
  spot: number;
  mvrvZ: number | null;
  nupl: number | null;
  realizedPrice: number | null;
  mayer: number | null;
  powerLawZ: number | null;
  asOf: number;
};

export type NetLiquidityResponse = {
  series: DailyPoint[]; // billions USD
  latest: number;
  delta30d: number;
  generatedAt: number;
};

export type FngPoint = { t: number; v: number; label: string };
export type FngResponse = {
  current: FngPoint;
  history: FngPoint[];
  generatedAt: number;
};

export type FundingExchange = "binance" | "bybit" | "okx";
export type FundingPoint = { t: number; rate: number }; // rate per 8h interval, decimal
export type FundingExchangeData = {
  exchange: FundingExchange;
  current: number;
  avg7d: number;
  history: FundingPoint[];
  error?: string;
};
export type FundingResponse = {
  exchanges: FundingExchangeData[];
  combinedCurrent: number | null;
  combinedAvg7d: number | null;
  combinedAnnualized7d: number | null;
  generatedAt: number;
};

export type EtfTicker = "ibit" | "fbtc" | "arkb";
export type EtfFlowDay = {
  t: number; // unix sec for date (UTC midnight)
  ibit: number | null;
  fbtc: number | null;
  arkb: number | null;
  total: number | null; // $M, all 11 ETFs combined
};
export type EtfFlowsResponse = {
  days: EtfFlowDay[];
  latestDate: number;
  cumulativeTotal: number; // $M, cumulative since Jan 2024
  latestTotal: number | null;
  flow7dAvg: number; // $M/day
  flow30dAvg: number; // $M/day
  generatedAt: number;
};

export type MacroKey = "dxy" | "dfii10" | "hyspread" | "vix" | "move";
export type MacroIndicator = {
  key: MacroKey;
  label: string;
  source: string;
  unit: string; // "" or "%" or "bp"
  current: number | null;
  prev30d: number | null;
  delta30d: number | null;
  asOf: number | null;
  error?: string;
};
export type MacroResponse = {
  indicators: MacroIndicator[];
  generatedAt: number;
};

export type CorrelationKey =
  | "sp500"
  | "ndx"
  | "gold"
  | "dxy"
  | "vix"
  | "dfii10";
export type CorrelationAsset = {
  key: CorrelationKey;
  label: string;
  source: string;
  r30: number | null; // current 30d rolling Pearson
  r90: number | null; // current 90d rolling Pearson
  error?: string;
};
export type CorrelationsResponse = {
  assets: CorrelationAsset[];
  generatedAt: number;
};

export type RegimeScoreInputKey = "fed" | "m2" | "mvrv" | "btc" | "dxy";
export type RegimeScoreInputDetail = {
  key: RegimeScoreInputKey;
  label: string;
  source: string;
  rawValue: number | null; // most recent observation
  rawAuxValue: number | null; // 6m-ago / 1y-ago / SMA value, for context
  unit: string;
  score: -1 | 0 | 1;
  rationale: string; // one-line "rate falling 0.4 pp over 6m" type description
};
export type RegimeScoreHistoryPoint = {
  t: number;
  score: number; // -5..+5
  alloc: number; // 0..1
};
export type RegimeScoreResponse = {
  asOf: number;
  currentScore: number; // -5..+5
  targetAllocPct: number; // 0..100
  inputs: RegimeScoreInputDetail[];
  history: RegimeScoreHistoryPoint[]; // ~2y weekly
  generatedAt: number;
  error?: string;
};

export type StrategyTicker = "STRC" | "STRF" | "STRK" | "STRD" | "MSTR";
export type StrategyAtmRow = {
  ticker: StrategyTicker;
  sharesSold: number;
  notionalM: number; // $M
  netProceedsM: number; // $M
  availableM: number; // $M, remaining ATM capacity
};
export type StrategyFiling = {
  accession: string;
  filedAt: number; // unix seconds, filing date
  periodEnd: number | null; // unix seconds, the "as of" date in the doc filename
  rows: StrategyAtmRow[]; // empty array if filing has no ATM table
};
export type StrcWeekPoint = {
  t: number; // unix sec for the filing date
  netProceedsM: number;       // STRC only
  totalProceedsM: number;     // all preferreds + MSTR combined
  impliedBtc: number | null;  // STRC proceeds / avg btc price that week
};
export type CbPremiumPoint = {
  t: number;
  cb: number;
  bn: number;
  premiumBps: number;
};
export type CbPremiumResponse = {
  series: CbPremiumPoint[]; // last 48 hourly points
  current: CbPremiumPoint | null;
  avg24hBps: number | null;
  generatedAt: number;
};

export type StrategyFlowsResponse = {
  // Spot from Yahoo
  strcPrice: number | null;
  strcChange24h: number | null;
  // From latest ATM-bearing 8-K
  latestFiledAt: number | null;
  strcLastWeekM: number | null;
  strcLastWeekImpliedBtc: number | null;
  strcRemainingCapacityM: number | null;
  totalAtmCapacityB: number | null; // sum across all preferreds + MSTR, in $B
  // Derived
  strcYtdM: number;
  weekly: StrcWeekPoint[]; // last 12 weeks, chronological
  generatedAt: number;
};

// --- Interpretation (AI market summary) ---

export type InterpretationResponse =
  | { disabled: true; reason: string }
  | {
      text: string;
      generatedAt: number;
      inputs: {
        spot: number;
        daysSinceAth: number;
        daysSinceLastHalving: number;
        powerLawZ: number | null;
        powerLawSlope: number;
        mvrvZ: number | null;
        nupl: number | null;
        mayer: number | null;
        realizedPrice: number | null;
        netLiquidityB: number | null;
        netLiquidityDelta30dB: number | null;
        fng: number | null;
        fngLabel: string | null;
      };
    };

// --- MEI (Market Emotion Indicator) ---

export type MeiPoint = {
  t: number;
  close: number;
  v: number | null;
  a: number | null;
  s: RegimeState;
  i: number | null;
  w: boolean;
};

export type MeiResponse = {
  series: MeiPoint[];
  current: {
    t: number;
    state: RegimeState;
    valence: number | null;
    arousal: number | null;
    intensity: number | null;
  };
  distribution: { state: RegimeState; count: number; pct: number }[];
  config: {
    normWindow: number;
    trendWindow: number;
    momWindow: number;
    volWindow: number;
    smoothSpan: number;
    bands: { valenceBear: number; valenceBull: number; arousalLow: number; arousalHigh: number };
  };
  generatedAt: number;
  volumeCoverage: number;
};
