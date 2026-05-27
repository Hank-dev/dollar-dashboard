import type { Status } from "./metrics";

export type NuclearPlayerKind = "public" | "project";
export type NuclearMetricGroup = "demand" | "fuel" | "policy" | "technology";
export type NuclearSignalTrack = "Reactors" | "Fuel" | "Customers" | "Risk";
export type NuclearConfidence = "high" | "medium" | "low";

export interface NuclearSource {
  name: string;
  url: string;
  asOf: string;
  confidence: NuclearConfidence;
}

export interface NuclearPlayer {
  id: string;
  kind: NuclearPlayerKind;
  name: string;
  ticker?: string;
  category: string;
  marketCap?: string;
  projectScale?: string;
  role: string;
  nuclearExposure: string;
  status: Status;
  source: NuclearSource;
}

export interface NuclearMarketMetric {
  id: string;
  group: NuclearMetricGroup;
  label: string;
  value: string;
  status: Status;
  context: string;
  detail: string;
  source: NuclearSource;
}

export interface NuclearTechSignal {
  id: string;
  track: NuclearSignalTrack;
  label: string;
  status: Status;
  summary: string;
  watchNext: string;
  source: NuclearSource;
}

export interface NuclearDashboardData {
  snapshotDate: string;
  verdict: string;
  players: NuclearPlayer[];
  marketMetrics: NuclearMarketMetric[];
  techSignals: NuclearTechSignal[];
}

export type NuclearExplainable =
  | { type: "player"; item: NuclearPlayer }
  | { type: "metric"; item: NuclearMarketMetric }
  | { type: "signal"; item: NuclearTechSignal };

export const NUCLEAR_GROUPS: Record<
  NuclearMetricGroup,
  { title: string; icon: string }
> = {
  demand: { title: "Power demand", icon: "activity" },
  fuel: { title: "Fuel cycle", icon: "currency-dollar" },
  policy: { title: "Policy & contracting", icon: "building-bank" },
  technology: { title: "Technology readiness", icon: "trending-up" },
};

const source = (
  name: string,
  url: string,
  asOf: string,
  confidence: NuclearConfidence,
): NuclearSource => ({ name, url, asOf, confidence });

const s = {
  ieaNuclear: source(
    "IEA Global Energy Review 2026 - nuclear",
    "https://www.iea.org/reports/global-energy-review-2026/technology-nuclear",
    "2026-03-25",
    "high",
  ),
  wnaReactors: source(
    "World Nuclear Association - nuclear power in the world today",
    "https://world-nuclear.org/information-library/current-and-future-generation/nuclear-power-in-the-world-today",
    "2026-04-21",
    "high",
  ),
  wnaUranium: source(
    "World Nuclear Association - supply of uranium",
    "https://world-nuclear.org/information-library/nuclear-fuel-cycle/uranium-resources/supply-of-uranium",
    "2025-12-01",
    "high",
  ),
  haleu: source(
    "World Nuclear Association - HALEU",
    "https://world-nuclear.org/information-library/nuclear-fuel-cycle/conversion-enrichment-and-fabrication/high-assay-low-enriched-uranium-haleu",
    "2026-02-01",
    "high",
  ),
  doeHaleu: source(
    "U.S. Department of Energy - HALEU shipment",
    "https://www.energy.gov/nnsa/articles/us-secures-largest-ever-haleu-shipment-power-american-nuclear-industry",
    "2026-05-07",
    "high",
  ),
  iaeaDataCenters: source(
    "IAEA - data centres, AI and advanced nuclear",
    "https://www.iaea.org/bulletin/data-centres-artificial-intelligence-and-cryptocurrencies-eye-advanced-nuclear-to-meet-growing-power-needs",
    "2024-09-01",
    "high",
  ),
  ceg: source(
    "StockAnalysis - Constellation valuation",
    "https://stockanalysis.com/stocks/ceg/statistics/",
    "2026-05-26",
    "medium",
  ),
  vst: source(
    "StockAnalysis - Vistra valuation",
    "https://stockanalysis.com/stocks/vst/statistics/",
    "2026-05-26",
    "medium",
  ),
  ccj: source(
    "StockAnalysis - Cameco market cap",
    "https://stockanalysis.com/stocks/ccj/market-cap/",
    "2026-05-26",
    "medium",
  ),
  bwxt: source(
    "StockAnalysis - BWX Technologies valuation",
    "https://stockanalysis.com/stocks/bwxt/statistics/",
    "2026-05-26",
    "medium",
  ),
  oklo: source(
    "StockAnalysis - Oklo valuation",
    "https://stockanalysis.com/stocks/oklo/statistics/",
    "2026-05-26",
    "medium",
  ),
  uec: source(
    "StockAnalysis - Uranium Energy market cap",
    "https://stockanalysis.com/stocks/uec/market-cap/",
    "2026-05-26",
    "medium",
  ),
  smr: source(
    "StockAnalysis - NuScale valuation",
    "https://stockanalysis.com/stocks/smr/statistics/",
    "2026-05-26",
    "medium",
  ),
  leu: source(
    "StockAnalysis - Centrus valuation",
    "https://stockanalysis.com/stocks/leu/statistics/",
    "2026-05-22",
    "medium",
  ),
  microsoft: source(
    "Constellation - Microsoft Crane Clean Energy Center agreement",
    "https://investors.constellationenergy.com/news-releases/news-release-details/constellation-launch-crane-clean-energy-center-restoring-jobs/",
    "2024-09-20",
    "high",
  ),
  google: source(
    "Google - Kairos Power nuclear agreement",
    "https://blog.google/outreach-initiatives/sustainability/google-kairos-power-nuclear-energy-agreement/",
    "2024-10-14",
    "high",
  ),
  amazon: source(
    "Amazon - SMR nuclear energy",
    "https://www.aboutamazon.com/news/sustainability/amazon-smr-nuclear-energy",
    "2025-10-16",
    "high",
  ),
  meta: source(
    "Meta - nuclear energy projects",
    "https://about.fb.com/news/2026/01/meta-nuclear-energy-projects-power-american-ai-leadership/",
    "2026-01-09",
    "high",
  ),
  smrIaea: source(
    "IAEA - small modular reactors",
    "https://www.iaea.org/topics/small-modular-reactors",
    "2025-09-01",
    "high",
  ),
};

const players: NuclearPlayer[] = [
  {
    id: "constellation",
    kind: "public",
    name: "Constellation Energy",
    ticker: "CEG",
    category: "Nuclear fleet and clean power PPAs",
    marketCap: "$108.9B",
    role: "Largest pure-play U.S. nuclear generation platform.",
    nuclearExposure:
      "Existing fleet, Crane restart, and large corporate PPAs make it the clearest public-market nuclear utility proxy.",
    status: "elevated",
    source: s.ceg,
  },
  {
    id: "vistra",
    kind: "public",
    name: "Vistra",
    ticker: "VST",
    category: "Merchant power and nuclear PPAs",
    marketCap: "$55.5B",
    role: "Independent power producer with nuclear assets and data-center contracting exposure.",
    nuclearExposure:
      "Nuclear output is becoming more valuable as hyperscalers pay for firm clean power.",
    status: "elevated",
    source: s.vst,
  },
  {
    id: "cameco",
    kind: "public",
    name: "Cameco",
    ticker: "CCJ",
    category: "Uranium and Westinghouse exposure",
    marketCap: "$46.6B",
    role: "Major uranium producer and strategic nuclear-services owner.",
    nuclearExposure:
      "Levered to uranium contracting, mine restarts, and the Westinghouse reactor-services platform.",
    status: "elevated",
    source: s.ccj,
  },
  {
    id: "bwxt",
    kind: "public",
    name: "BWX Technologies",
    ticker: "BWXT",
    category: "Nuclear components and naval reactors",
    marketCap: "$18.7B",
    role: "Specialized nuclear manufacturing, services, and defense nuclear supplier.",
    nuclearExposure:
      "More industrial/defense nuclear than merchant power; lower concept risk than pre-revenue reactor developers.",
    status: "neutral",
    source: s.bwxt,
  },
  {
    id: "oklo",
    kind: "public",
    name: "Oklo",
    ticker: "OKLO",
    category: "Advanced reactor developer",
    marketCap: "$12.0B",
    role: "Fast-reactor and fuel-cycle development platform.",
    nuclearExposure:
      "High optionality but high execution and licensing risk; no commercial reactor fleet yet.",
    status: "stressed",
    source: s.oklo,
  },
  {
    id: "uranium-energy",
    kind: "public",
    name: "Uranium Energy",
    ticker: "UEC",
    category: "Uranium developer",
    marketCap: "$6.4B",
    role: "U.S.-oriented uranium production optionality.",
    nuclearExposure:
      "Tied to uranium price, contracting, and restart/development execution rather than reactor sales.",
    status: "elevated",
    source: s.uec,
  },
  {
    id: "nuscale",
    kind: "public",
    name: "NuScale Power",
    ticker: "SMR",
    category: "Light-water SMR developer",
    marketCap: "$4.5B",
    role: "Certified U.S. SMR technology with commercialization risk.",
    nuclearExposure:
      "Regulatory first-mover status matters, but project economics and customer conversion remain the proof points.",
    status: "stressed",
    source: s.smr,
  },
  {
    id: "centrus",
    kind: "public",
    name: "Centrus Energy",
    ticker: "LEU",
    category: "Enrichment and HALEU",
    marketCap: "$3.5B",
    role: "Nuclear fuel-cycle and enrichment bottleneck exposure.",
    nuclearExposure:
      "Strategically important if advanced reactors need domestic HALEU at commercial scale.",
    status: "elevated",
    source: s.leu,
  },
  {
    id: "microsoft-crane",
    kind: "project",
    name: "Microsoft / Constellation",
    category: "Nuclear restart",
    projectScale: "835 MW",
    role: "20-year PPA supporting the restart of the Crane Clean Energy Center.",
    nuclearExposure:
      "Shows that near-term nuclear growth may come from restarts and uprates before new SMRs arrive.",
    status: "elevated",
    source: s.microsoft,
  },
  {
    id: "google-kairos",
    kind: "project",
    name: "Google / Kairos Power",
    category: "Advanced reactors",
    projectScale: "Up to 500 MWe",
    role: "Corporate agreement for multiple Kairos deployments through 2035.",
    nuclearExposure:
      "A key signal that hyperscalers are willing to contract ahead of commercial advanced-reactor maturity.",
    status: "neutral",
    source: s.google,
  },
  {
    id: "amazon-xenergy",
    kind: "project",
    name: "Amazon / X-energy",
    category: "SMR campus",
    projectScale: "320-960 MW",
    role: "Cascade project with Energy Northwest and Xe-100 reactors.",
    nuclearExposure:
      "Large enough to matter for data centers, but still depends on licensing, financing, and construction execution.",
    status: "neutral",
    source: s.amazon,
  },
  {
    id: "meta-nuclear",
    kind: "project",
    name: "Meta nuclear portfolio",
    category: "Corporate nuclear procurement",
    projectScale: "Up to 6.6 GW",
    role: "Agreements across existing and new nuclear to support AI power demand.",
    nuclearExposure:
      "Shows the demand pull: nuclear is being bought as firm clean capacity, not just ESG branding.",
    status: "elevated",
    source: s.meta,
  },
];

const marketMetrics: NuclearMarketMetric[] = [
  {
    id: "global-capacity",
    group: "demand",
    label: "Operating capacity",
    value: "420 GW",
    status: "neutral",
    context: "Global fleet at end-2025",
    detail:
      "Nuclear remains a large but slow-moving source of firm power across more than 30 countries.",
    source: s.ieaNuclear,
  },
  {
    id: "under-construction",
    group: "technology",
    label: "Under construction",
    value: "78 GW",
    status: "elevated",
    context: "Highest levels in ~30 years",
    detail:
      "The buildout is real, but heavily concentrated in China and Russian-designed supply chains.",
    source: s.ieaNuclear,
  },
  {
    id: "new-starts",
    group: "policy",
    label: "Construction starts",
    value: "10",
    status: "neutral",
    context: "2025 starts: 9 China, 1 Russia",
    detail:
      "Western policy support is rising, but concrete poured is still mostly outside the U.S. and Europe.",
    source: s.ieaNuclear,
  },
  {
    id: "uranium-demand",
    group: "fuel",
    label: "Annual uranium need",
    value: "67k tU",
    status: "elevated",
    context: "Current reactor fleet",
    detail:
      "Fuel demand is steady and strategic; term contracting matters more than spot price headlines.",
    source: s.wnaUranium,
  },
  {
    id: "haleu",
    group: "fuel",
    label: "HALEU availability",
    value: "Constrained",
    status: "stressed",
    context: "Advanced reactor bottleneck",
    detail:
      "Many advanced reactor designs need HALEU, but commercial Western supply is not yet broadly available.",
    source: s.haleu,
  },
  {
    id: "data-center-pull",
    group: "demand",
    label: "Hyperscaler pull",
    value: "Rising",
    status: "elevated",
    context: "AI power demand",
    detail:
      "Tech buyers are now credible nuclear counterparties, especially for firm 24/7 clean power.",
    source: s.iaeaDataCenters,
  },
];

const techSignals: NuclearTechSignal[] = [
  {
    id: "restart-uprate",
    track: "Reactors",
    label: "Restarts and uprates are the near-term path",
    status: "elevated",
    summary:
      "Existing plants and dormant assets can add firm clean power before most new-build SMRs are licensed and financed.",
    watchNext: "Watch Crane, Clinton, Perry, Davis-Besse, and Beaver Valley milestones.",
    source: s.microsoft,
  },
  {
    id: "smr-commercial",
    track: "Reactors",
    label: "SMRs are demand-backed but not yet routine infrastructure",
    status: "neutral",
    summary:
      "Corporate demand is strong, but commercial deployment still needs repeatable licensing, supply chains, and cost evidence.",
    watchNext: "Watch first-of-a-kind construction decisions in the U.S., Canada, UK, and Korea.",
    source: s.smrIaea,
  },
  {
    id: "haleu-bottleneck",
    track: "Fuel",
    label: "Advanced fuel is a gating item",
    status: "stressed",
    summary:
      "HALEU supply, transport containers, conversion, enrichment, and deconversion capacity must scale together.",
    watchNext: "Watch DOE allocation, Centrus expansion, Urenco plans, and non-Russian supply commitments.",
    source: s.doeHaleu,
  },
  {
    id: "corporate-ppa",
    track: "Customers",
    label: "Hyperscalers are becoming anchor customers",
    status: "elevated",
    summary:
      "Microsoft, Google, Amazon, and Meta are turning nuclear into a procurement market for reliable clean megawatts.",
    watchNext: "Watch which agreements move from announcements to interconnection, licensing, and financing.",
    source: s.meta,
  },
  {
    id: "geopolitics",
    track: "Risk",
    label: "Supply chain geopolitics define the investable map",
    status: "elevated",
    summary:
      "Uranium, conversion, enrichment, heavy components, and reactor vendor nationality now matter commercially.",
    watchNext: "Watch Russian-fuel restrictions, China build rates, and Western industrial-policy funding.",
    source: s.ieaNuclear,
  },
  {
    id: "waste-licensing",
    track: "Risk",
    label: "Licensing and waste remain public-trust constraints",
    status: "neutral",
    summary:
      "Nuclear has strong energy-density advantages, but approvals, local acceptance, and waste pathways remain slow.",
    watchNext: "Watch NRC process reforms and whether standardized designs shorten deployment cycles.",
    source: s.smrIaea,
  },
];

export function getNuclearDashboardData(): NuclearDashboardData {
  return {
    snapshotDate: "2026-05-27",
    verdict:
      "Nuclear is shifting from climate-policy optionality to power-market infrastructure. The most bankable near-term value is in existing fleets, restarts, fuel-cycle bottlenecks, and corporate PPAs; SMRs and advanced reactors are strategically important but still need licensing, fuel, and first-project execution proof.",
    players,
    marketMetrics,
    techSignals,
  };
}
