import type { Status } from "./metrics";
import snapshot from "./aiSnapshot.json" with { type: "json" };
import { validateAiSnapshot, type AiSnapshot } from "./aiSnapshot.schema";
import { fetchEquityQuotes, formatMarketCapUsd } from "./quotes";

export type AiPlayerKind = "public" | "private";
export type AiMetricGroup = "capital" | "adoption" | "technology" | "risk";
export type AiSignalTrack = "Models" | "Agents" | "Tools" | "Infra";
export type Confidence = "high" | "medium" | "low";

export interface AiSource {
  name: string;
  url: string;
  asOf: string;
  confidence: Confidence;
}

export interface AiPlayer {
  id: string;
  kind: AiPlayerKind;
  name: string;
  ticker?: string;
  category: string;
  marketCap?: string;
  valuationEstimate?: string;
  adoptionSignal: string;
  aiExposure: string;
  status: Status;
  source: AiSource;
}

export interface AiMarketMetric {
  id: string;
  group: AiMetricGroup;
  label: string;
  value: string;
  status: Status;
  context: string;
  detail: string;
  source: AiSource;
}

export interface AiTechSignal {
  id: string;
  track: AiSignalTrack;
  label: string;
  status: Status;
  summary: string;
  watchNext: string;
  source: AiSource;
}

export interface AiDashboardData {
  snapshotDate: string;
  verdict: string;
  players: AiPlayer[];
  marketMetrics: AiMarketMetric[];
  techSignals: AiTechSignal[];
}

export type AiExplainable =
  | { type: "player"; item: AiPlayer }
  | { type: "metric"; item: AiMarketMetric }
  | { type: "signal"; item: AiTechSignal };

export const AI_GROUPS: Record<AiMetricGroup, { title: string; icon: string }> = {
  capital: { title: "Capital & market structure", icon: "building-bank" },
  adoption: { title: "Adoption & users", icon: "activity" },
  technology: { title: "Technology state", icon: "trending-up" },
  risk: { title: "Risks & constraints", icon: "message" },
};

type PlayerRoster = Pick<AiPlayer, "id" | "kind" | "name" | "category"> & { ticker?: string };
type MetricRoster = Pick<AiMarketMetric, "id" | "group" | "label">;
type SignalRoster = Pick<AiTechSignal, "id" | "track">;

const PLAYER_ROSTER: PlayerRoster[] = [
  { id: "nvidia", kind: "public", name: "NVIDIA", ticker: "NVDA", category: "AI compute" },
  { id: "alphabet", kind: "public", name: "Alphabet", ticker: "GOOGL", category: "Models, search, cloud" },
  { id: "microsoft", kind: "public", name: "Microsoft", ticker: "MSFT", category: "Enterprise agents" },
  { id: "amazon", kind: "public", name: "Amazon", ticker: "AMZN", category: "Cloud and retail agents" },
  { id: "broadcom", kind: "public", name: "Broadcom", ticker: "AVGO", category: "AI networking and ASICs" },
  { id: "meta", kind: "public", name: "Meta", ticker: "META", category: "Consumer AI and open models" },
  { id: "openai", kind: "private", name: "OpenAI", category: "Frontier lab and agent platform" },
  { id: "anthropic", kind: "private", name: "Anthropic", category: "Frontier lab" },
  { id: "databricks", kind: "private", name: "Databricks", category: "Data and AI platform" },
  { id: "cursor", kind: "private", name: "Cursor / Anysphere", category: "Coding agent" },
  { id: "perplexity", kind: "private", name: "Perplexity", category: "Answer engine and agents" },
];

const METRIC_ROSTER: MetricRoster[] = [
  { id: "capex-race", group: "capital", label: "Hyperscaler AI capex" },
  { id: "private-valuation", group: "capital", label: "Private lab valuations" },
  { id: "consumer-scale", group: "adoption", label: "ChatGPT reach" },
  { id: "coding-agent", group: "adoption", label: "Coding assistants" },
  { id: "open-pressure", group: "technology", label: "Open-model pressure" },
  { id: "agent-reliability", group: "risk", label: "Agent reliability" },
];

const SIGNAL_ROSTER: SignalRoster[] = [
  { id: "frontier-models", track: "Models" },
  { id: "agents", track: "Agents" },
  { id: "ai-ides", track: "Tools" },
  { id: "compute-stack", track: "Infra" },
];

const ROSTER_IDS = {
  players: PLAYER_ROSTER.map((p) => p.id),
  marketMetrics: METRIC_ROSTER.map((m) => m.id),
  techSignals: SIGNAL_ROSTER.map((s) => s.id),
};

const SNAPSHOT = snapshot as AiSnapshot;

function freshestAsOf(): string {
  const dates = [
    SNAPSHOT.verdict.asOf,
    ...Object.values(SNAPSHOT.players).map((p) => p.source.asOf),
    ...Object.values(SNAPSHOT.marketMetrics).map((m) => m.source.asOf),
    ...Object.values(SNAPSHOT.techSignals).map((t) => t.source.asOf),
  ].filter(Boolean);
  return dates.sort().at(-1) ?? SNAPSHOT.verdict.asOf;
}

export function getAiDashboardData(): AiDashboardData {
  if (process.env.NODE_ENV !== "production") {
    const errors = validateAiSnapshot(SNAPSHOT, ROSTER_IDS);
    if (errors.length) throw new Error(`aiSnapshot.json invalid:\n${errors.join("\n")}`);
  }

  const players: AiPlayer[] = PLAYER_ROSTER.flatMap((r) => {
    const c = SNAPSHOT.players[r.id];
    if (!c) {
      console.warn(`aiSnapshot.json: missing player "${r.id}" — omitting`);
      return [];
    }
    return [{
      ...r,
      marketCap: c.marketCap,
      valuationEstimate: c.valuationEstimate,
      adoptionSignal: c.adoptionSignal,
      aiExposure: c.aiExposure,
      status: c.status,
      source: c.source,
    }];
  });

  const marketMetrics: AiMarketMetric[] = METRIC_ROSTER.flatMap((r) => {
    const c = SNAPSHOT.marketMetrics[r.id];
    if (!c) {
      console.warn(`aiSnapshot.json: missing metric "${r.id}" — omitting`);
      return [];
    }
    return [{ ...r, value: c.value, context: c.context, detail: c.detail, status: c.status, source: c.source }];
  });

  const techSignals: AiTechSignal[] = SIGNAL_ROSTER.flatMap((r) => {
    const c = SNAPSHOT.techSignals[r.id];
    if (!c) {
      console.warn(`aiSnapshot.json: missing signal "${r.id}" — omitting`);
      return [];
    }
    return [{ ...r, label: c.label, status: c.status, summary: c.summary, watchNext: c.watchNext, source: c.source }];
  });

  return { snapshotDate: freshestAsOf(), verdict: SNAPSHOT.verdict.text, players, marketMetrics, techSignals };
}

const PUBLIC_TICKERS: Record<string, string> = Object.fromEntries(
  PLAYER_ROSTER.filter((p) => p.kind === "public" && p.ticker).map((p) => [p.id, p.ticker as string]),
);

export async function getAiDashboardDataLive(): Promise<AiDashboardData> {
  const data = getAiDashboardData();
  try {
    const symbols = Object.values(PUBLIC_TICKERS);
    const { quotes } = await fetchEquityQuotes(symbols);
    const capBySymbol = new Map(
      quotes.filter((q) => q.marketCapSource === "live").map((q) => [q.symbol, q.marketCapUsd]),
    );
    const players = data.players.map((p) => {
      const ticker = PUBLIC_TICKERS[p.id];
      const cap = ticker ? capBySymbol.get(ticker) : null;
      const formatted = formatMarketCapUsd(cap ?? null);
      return formatted ? { ...p, marketCap: formatted } : p;
    });
    return { ...data, players };
  } catch {
    return data;
  }
}
