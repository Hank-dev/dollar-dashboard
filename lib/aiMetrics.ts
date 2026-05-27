import type { Status } from "./metrics";

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

const source = (
  name: string,
  url: string,
  asOf: string,
  confidence: Confidence,
): AiSource => ({ name, url, asOf, confidence });

const s = {
  publicMarketCaps: source(
    "StockAnalysis / public market-cap snapshots",
    "https://stockanalysis.com/stocks/nvda/market-cap/",
    "2026-05-22",
    "medium",
  ),
  openai: source(
    "TechCrunch, OpenAI funding and ChatGPT users",
    "https://techcrunch.com/2026/02/27/openai-raises-110b-in-one-of-the-largest-private-funding-rounds-in-history/",
    "2026-02-27",
    "medium",
  ),
  chatgptUsers: source(
    "TechCrunch, ChatGPT weekly active users",
    "https://techcrunch.com/2026/02/27/chatgpt-reaches-900m-weekly-active-users/",
    "2026-02-27",
    "medium",
  ),
  anthropic: source(
    "Axios, Anthropic funding round",
    "https://www.axios.com/2026/02/12/anthropic-raises-30b-at-380b-valuation",
    "2026-02-12",
    "medium",
  ),
  databricks: source(
    "Databricks valuation reporting",
    "https://ai2.work/blog/databricks-at-134b-the-ipo-that-will-define-ai-valuations-in-2026",
    "2026-02-09",
    "low",
  ),
  cursor: source(
    "TechCrunch, Anysphere funding",
    "https://techcrunch.com/2025/06/05/cursors-anysphere-nabs-9-9b-valuation-soars-past-500m-arr/",
    "2025-06-05",
    "medium",
  ),
  perplexity: source(
    "Sacra, Perplexity revenue and valuation",
    "https://sacra.com/research/perplexity/",
    "2026-04-30",
    "low",
  ),
  copilot: source(
    "TechCrunch, GitHub Copilot usage",
    "https://techcrunch.com/2025/07/30/github-copilot-crosses-20-million-all-time-users/",
    "2025-07-30",
    "medium",
  ),
  capex: source(
    "Epoch AI, hyperscaler capex trend",
    "https://epoch.ai/data-insights/hyperscaler-capex-trend",
    "2026-02-27",
    "high",
  ),
  nist: source(
    "NIST AI Risk Management Framework",
    "https://www.nist.gov/itl/ai-risk-management-framework",
    "2026-04-07",
    "high",
  ),
  openModels: source(
    "LLMCheck, state of open-source local LLMs",
    "https://llmcheck.net/blog/state-of-open-source-local-llms-may-2026/",
    "2026-05-09",
    "low",
  ),
};

const players: AiPlayer[] = [
  {
    id: "nvidia",
    kind: "public",
    name: "NVIDIA",
    ticker: "NVDA",
    category: "AI compute",
    marketCap: "$5.2T",
    adoptionSignal: "Default accelerator stack for training and inference clusters.",
    aiExposure: "Cleanest public-market proxy for frontier AI compute demand.",
    status: "stressed",
    source: s.publicMarketCaps,
  },
  {
    id: "alphabet",
    kind: "public",
    name: "Alphabet",
    ticker: "GOOGL",
    category: "Models, search, cloud",
    marketCap: "~$4.7T",
    adoptionSignal: "Gemini distribution spans Search, Android, Workspace, and Google Cloud.",
    aiExposure: "Owns TPUs, consumer distribution, ads surface, and enterprise cloud channel.",
    status: "elevated",
    source: s.publicMarketCaps,
  },
  {
    id: "microsoft",
    kind: "public",
    name: "Microsoft",
    ticker: "MSFT",
    category: "Enterprise agents",
    marketCap: "~$3.6T",
    adoptionSignal: "Copilot is the enterprise wedge; GitHub Copilot crossed 20M all-time users.",
    aiExposure: "Azure, OpenAI partnership, Office, Windows, GitHub, and security workflow reach.",
    status: "elevated",
    source: s.copilot,
  },
  {
    id: "amazon",
    kind: "public",
    name: "Amazon",
    ticker: "AMZN",
    category: "Cloud and retail agents",
    marketCap: "~$2.9T",
    adoptionSignal: "Bedrock and AWS infrastructure anchor enterprise model deployment.",
    aiExposure: "AI demand flows through AWS compute, custom silicon, logistics, and retail assistants.",
    status: "elevated",
    source: s.publicMarketCaps,
  },
  {
    id: "broadcom",
    kind: "public",
    name: "Broadcom",
    ticker: "AVGO",
    category: "AI networking and ASICs",
    marketCap: "~$2.0T",
    adoptionSignal: "Custom silicon and networking exposure rises with hyperscaler AI clusters.",
    aiExposure: "Picks-and-shovels beneficiary when frontier labs diversify beyond merchant GPUs.",
    status: "elevated",
    source: s.publicMarketCaps,
  },
  {
    id: "meta",
    kind: "public",
    name: "Meta",
    ticker: "META",
    category: "Consumer AI and open models",
    marketCap: "~$1.5T",
    adoptionSignal: "AI is distributed through feeds, ads, messaging, wearables, and open-weight models.",
    aiExposure: "Monetizes AI mostly through ad ranking, engagement, and developer ecosystem gravity.",
    status: "neutral",
    source: s.publicMarketCaps,
  },
  {
    id: "openai",
    kind: "private",
    name: "OpenAI",
    category: "Frontier lab and agent platform",
    valuationEstimate: "~$840B post-money",
    adoptionSignal: "ChatGPT reported 900M weekly active users and 50M paid subscribers.",
    aiExposure: "Consumer AI distribution leader with enterprise, API, coding, and agent ambitions.",
    status: "stressed",
    source: s.openai,
  },
  {
    id: "anthropic",
    kind: "private",
    name: "Anthropic",
    category: "Frontier lab",
    valuationEstimate: "~$380B post-money",
    adoptionSignal: "Enterprise adoption and revenue acceleration drive the valuation narrative.",
    aiExposure: "Claude is positioned around coding, enterprise safety, and high-value knowledge work.",
    status: "elevated",
    source: s.anthropic,
  },
  {
    id: "databricks",
    kind: "private",
    name: "Databricks",
    category: "Data and AI platform",
    valuationEstimate: "~$134B",
    adoptionSignal: "Lakehouse data estate is the enterprise control point for production AI.",
    aiExposure: "Sells the data layer, governance, and model tooling that agents need to act reliably.",
    status: "neutral",
    source: s.databricks,
  },
  {
    id: "cursor",
    kind: "private",
    name: "Cursor / Anysphere",
    category: "Coding agent",
    valuationEstimate: "$9.9B",
    adoptionSignal: "Reported above $500M ARR in 2025; strongest independent AI IDE signal.",
    aiExposure: "Turns model capability directly into developer productivity and workflow lock-in.",
    status: "elevated",
    source: s.cursor,
  },
  {
    id: "perplexity",
    kind: "private",
    name: "Perplexity",
    category: "Answer engine and agents",
    valuationEstimate: "~$18B prior round",
    adoptionSignal: "Sacra estimates $500M annualized revenue in April 2026.",
    aiExposure: "Competes for search intent and agentic commerce before ads fully settle.",
    status: "neutral",
    source: s.perplexity,
  },
];

const marketMetrics: AiMarketMetric[] = [
  {
    id: "capex-race",
    group: "capital",
    label: "Hyperscaler AI capex",
    value: "Quadrupled",
    status: "stressed",
    context: "Since GPT-4 release",
    detail:
      "The market is supply constrained: capital intensity is rising faster than most software revenue lines.",
    source: s.capex,
  },
  {
    id: "private-valuation",
    group: "capital",
    label: "Private lab valuations",
    value: "$100B+ tier",
    status: "elevated",
    context: "OpenAI, Anthropic, Databricks",
    detail:
      "Private marks are now large enough to influence public-company strategy, cloud commitments, and M&A math.",
    source: s.openai,
  },
  {
    id: "consumer-scale",
    group: "adoption",
    label: "ChatGPT reach",
    value: "900M WAU",
    status: "stressed",
    context: "Reported February 2026",
    detail:
      "Consumer AI has crossed mass-market scale; the open question is revenue per active user and retention by task.",
    source: s.chatgptUsers,
  },
  {
    id: "coding-agent",
    group: "adoption",
    label: "Coding assistants",
    value: "20M+",
    status: "elevated",
    context: "GitHub Copilot all-time users",
    detail:
      "Developer tools are the clearest paid-agent wedge because output quality is measurable and time savings are obvious.",
    source: s.copilot,
  },
  {
    id: "open-pressure",
    group: "technology",
    label: "Open-model pressure",
    value: "Rising",
    status: "elevated",
    context: "Qwen, DeepSeek, Llama, Mistral",
    detail:
      "Open and open-weight models pressure API pricing and push differentiation toward distribution, tooling, and data.",
    source: s.openModels,
  },
  {
    id: "agent-reliability",
    group: "risk",
    label: "Agent reliability",
    value: "Bottleneck",
    status: "stressed",
    context: "Autonomy needs guardrails",
    detail:
      "Agents are useful in bounded workflows, but planning errors, tool misuse, permissions, and auditability still cap autonomy.",
    source: s.nist,
  },
];

const techSignals: AiTechSignal[] = [
  {
    id: "frontier-models",
    track: "Models",
    label: "Frontier models are multimodal operating systems",
    status: "elevated",
    summary:
      "The leading labs are bundling text, code, vision, voice, memory, tools, and computer use into one platform surface.",
    watchNext: "Watch whether model gains translate into lower cost per completed task, not just higher benchmark scores.",
    source: s.openai,
  },
  {
    id: "agents",
    track: "Agents",
    label: "Agents work best where mistakes are reversible",
    status: "neutral",
    summary:
      "Coding, support triage, research, data cleanup, and sales ops are ahead of high-stakes autonomous execution.",
    watchNext: "Watch permissions, evals, and human approval loops become product primitives.",
    source: s.nist,
  },
  {
    id: "ai-ides",
    track: "Tools",
    label: "AI IDEs are the most legible agent business",
    status: "elevated",
    summary:
      "Cursor and Copilot show that workflow-native agents can monetize faster than broad horizontal assistants.",
    watchNext: "Watch whether coding agents expand from suggestions into repo-level planning, testing, and code review.",
    source: s.cursor,
  },
  {
    id: "compute-stack",
    track: "Infra",
    label: "Compute is still the choke point",
    status: "stressed",
    summary:
      "GPU supply, power, data centers, networking, and custom ASIC roadmaps are shaping product release cadence.",
    watchNext: "Watch capex efficiency, inference margins, and how much custom silicon shifts value away from NVIDIA.",
    source: s.capex,
  },
];

export function getAiDashboardData(): AiDashboardData {
  return {
    snapshotDate: "2026-05-27",
    verdict:
      "AI is no longer a feature cycle; it is a capital cycle. Public value is concentrated in compute and cloud, private value is concentrated in frontier labs and workflow agents, and the next proof point is whether agents can convert huge usage into durable, auditable revenue.",
    players,
    marketMetrics,
    techSignals,
  };
}
