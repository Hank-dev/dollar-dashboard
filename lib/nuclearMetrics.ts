import type { Status } from "./metrics";
import { MARKET_CAP_USD_BY_SYMBOL } from "./nuclearLive";

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
    "2026-08-03",
    "high",
  ),
  wnaReactors: source(
    "World Nuclear Association - nuclear power in the world today",
    "https://world-nuclear.org/information-library/current-and-future-generation/nuclear-power-in-the-world-today",
    "2026-08-31",
    "high",
  ),
  wnaUranium: source(
    "World Nuclear Association - supply of uranium",
    "https://world-nuclear.org/information-library/nuclear-fuel-cycle/uranium-resources/supply-of-uranium",
    "2026-08-03",
    "high",
  ),
  haleu: source(
    "World Nuclear News - Centrus, Oklo HALEU agreement",
    "https://www.world-nuclear-news.org/articles/centrus-oklo-haleu-agreement-to-support-reactor-deployment",
    "2026-08-03",
    "high",
  ),
  doeHaleu: source(
    "U.S. Department of Energy - domestic enrichment award",
    "https://www.energy.gov/articles/us-department-energy-awards-27-billion-restore-american-uranium-enrichment",
    "2026-08-03",
    "high",
  ),
  uraniumSpot: source(
    "Trading Economics - uranium spot price",
    "https://tradingeconomics.com/commodity/uranium",
    "2026-08-31",
    "high",
  ),
  iaeaDataCenters: source(
    "IEA - data-centre electricity demand and AI",
    "https://www.iea.org/reports/key-questions-on-energy-and-ai/executive-summary",
    "2026-08-03",
    "high",
  ),
  ceg: source(
    "Stock Analysis - Constellation Energy market cap",
    "https://stockanalysis.com/stocks/ceg/market-cap/",
    "2026-08-31",
    "high",
  ),
  vst: source(
    "Stock Analysis - Vistra market cap",
    "https://stockanalysis.com/stocks/vst/market-cap/",
    "2026-08-31",
    "high",
  ),
  ccj: source(
    "Stock Analysis - Cameco market cap",
    "https://stockanalysis.com/stocks/ccj/market-cap/",
    "2026-08-31",
    "high",
  ),
  bwxt: source(
    "Stock Analysis - BWX Technologies market cap",
    "https://stockanalysis.com/stocks/bwxt/market-cap/",
    "2026-08-31",
    "high",
  ),
  oklo: source(
    "Stock Analysis - Oklo market cap",
    "https://stockanalysis.com/stocks/oklo/market-cap/",
    "2026-08-31",
    "high",
  ),
  uec: source(
    "Stock Analysis - Uranium Energy market cap",
    "https://stockanalysis.com/stocks/uec/market-cap/",
    "2026-08-31",
    "high",
  ),
  smr: source(
    "Stock Analysis - NuScale Power market cap",
    "https://stockanalysis.com/stocks/smr/market-cap/",
    "2026-08-31",
    "high",
  ),
  leu: source(
    "Stock Analysis - Centrus Energy market cap",
    "https://stockanalysis.com/stocks/leu/market-cap/",
    "2026-08-31",
    "high",
  ),
  xEnergyCentrus: source(
    "World Nuclear News - X-Energy and Centrus sign HALEU supply agreement",
    "https://www.world-nuclear-news.org/articles/x-energy-and-centrus-sign-haleu-supply-agreement",
    "2026-08-10",
    "high",
  ),
  okloGroves: source(
    "World Nuclear News - US test reactors achieve milestones",
    "https://www.world-nuclear-news.org/articles/us-test-reactors-achieve-milestones",
    "2026-08-10",
    "high",
  ),
  microsoft: source(
    "Constellation - Microsoft Crane Clean Energy Center agreement",
    "https://investors.constellationenergy.com/news-releases/news-release-details/constellation-launch-crane-clean-energy-center-restoring-jobs/",
    "2026-08-03",
    "high",
  ),
  google: source(
    "Google - Kairos Power nuclear agreement",
    "https://blog.google/outreach-initiatives/sustainability/google-kairos-power-nuclear-energy-agreement/",
    "2026-08-03",
    "high",
  ),
  amazon: source(
    "Amazon - SMR nuclear energy",
    "https://www.aboutamazon.com/news/sustainability/amazon-smr-nuclear-energy",
    "2026-08-03",
    "high",
  ),
  meta: source(
    "Meta - nuclear energy projects",
    "https://about.fb.com/news/2026/01/meta-nuclear-energy-projects-power-american-ai-leadership/",
    "2026-08-03",
    "high",
  ),
  smrIaea: source(
    "IAEA - small modular reactors",
    "https://www.iaea.org/topics/small-modular-reactors",
    "2026-08-03",
    "high",
  ),
  palisades: source(
    "Holtec - Palisades major restart projects complete",
    "https://holtecinternational.com/hh-41-10/",
    "2026-08-03",
    "medium",
  ),
  doeCriticality: source(
    "U.S. Department of Energy - fourth advanced reactor criticality",
    "https://www.energy.gov/articles/department-energy-celebrates-fourth-criticality-ahead-july-4th-goal",
    "2026-08-03",
    "high",
  ),
  doeLoans: source(
    "U.S. Department of Energy - American Nuclear Supply Chain Loans",
    "https://www.energy.gov/articles/department-energy-announces-american-nuclear-supply-chain-loans",
    "2026-08-03",
    "high",
  ),
  ansJuly: source(
    "American Nuclear Society - Industry Update, July 2026",
    "https://www.ans.org/news/article-8165/industry-updatejuly-2026/",
    "2026-08-03",
    "medium",
  ),
  nrcRadiation: source(
    "U.S. NRC - proposed radiation protection rule modernization",
    "https://www.nrc.gov/sites/default/files/cdn/doc-collection-news/2026/26-070.pdf",
    "2026-08-03",
    "high",
  ),
  ansFuelJuly24: source(
    "American Nuclear Society - Standard Nuclear TRISO facilities",
    "https://www.ans.org/news/2026-07-24/article-8241/standard-nuclear-plans-triso-production-at-two-new-facilities-in-2026/",
    "2026-08-03",
    "medium",
  ),
  ansTerraPowerJuly24: source(
    "American Nuclear Society - TerraPower joins INPO",
    "https://www.ans.org/news/2026-07-24/article-8243/terrapower-becomes-first-advanced-reactor-company-to-join-inpo/",
    "2026-08-03",
    "high",
  ),
  ansLicensingJuly2: source(
    "American Nuclear Society - NRC licensing and ALARA proposals",
    "https://www.ans.org/news/2026-07-02/article-8177/proposed-rules-on-alara-reactor-licensing-revamp-introduced-by-nrc/",
    "2026-08-24",
    "high",
  ),
  doosanNatrium: source(
    "World Nuclear News - Doosan Enerbility contracted for Natrium components",
    "https://www.world-nuclear-news.org/articles/doosan-enerbility-contracted-for-natrium-components",
    "2026-08-24",
    "high",
  ),
  carnegieHyperscalers: source(
    "Carnegie Endowment - hyperscaler nuclear commitments",
    "https://carnegieendowment.org/research/2026/06/beyond-the-hype-assessing-hyperscaler-nuclear-commitments-against-us-energy-realities",
    "2026-08-03",
    "medium",
  ),
  evinciCriticality: source(
    "American Nuclear Society - eVinci completes cold criticality test at NCERC",
    "https://www.ans.org/news/2026-08-26/article-8341/evinci-completes-cold-criticality-test-at-ncerc/",
    "2026-08-31",
    "high",
  ),
  doeWipp: source(
    "American Nuclear Society - DOE plans to expand WIPP capacity and operational life",
    "https://www.ans.org/news/2026-08-27/article-8343/doe-plans-to-expand-wipps-capacity-and-operational-life/",
    "2026-08-31",
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
    marketCap: "$98.1B",
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
    marketCap: "$46.0B",
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
    marketCap: "$43.6B",
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
    marketCap: "$14.0B",
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
    marketCap: "$7.5B",
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
    marketCap: "$6.1B",
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
    marketCap: "$4.0B",
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
    value: "400 GWe",
    status: "neutral",
    context: "440 reactors; ~400 GWe operable",
    detail:
      "The World Nuclear Association counts 440 operable power reactors totaling about 400 GWe across more than 30 countries; nuclear supplies about 9% of global electricity.",
    source: s.wnaReactors,
  },
  {
    id: "under-construction",
    group: "technology",
    label: "Under construction",
    value: "75 reactors",
    status: "elevated",
    context: "~400 GWe fleet; units concentrated in Asia",
    detail:
      "The World Nuclear Association counts about 75 reactors under construction worldwide alongside the ~400 GWe operable fleet; the pipeline is concentrated in Asia and remains a long-duration buildout rather than near-term generating capacity.",
    source: s.wnaReactors,
  },
  {
    id: "new-starts",
    group: "policy",
    label: "Planned U.S. builds",
    value: "10 × AP1000",
    status: "elevated",
    context: "$17.5B conditional loans; 11 GW total",
    detail:
      "DOE conditionally committed financing for long-lead items at five two-reactor sites, but each site still needs partner equity and final loan conditions before funding.",
    source: s.doeLoans,
  },
  {
    id: "uranium-demand",
    group: "fuel",
    label: "Uranium price",
    value: "$89.85/lb",
    status: "elevated",
    context: "Aug 28; +3.75% MoM; +17.22% YoY",
    detail:
      "Trading Economics reported uranium at $89.85/lb on August 28; the benchmark fell 0.61% on the day, gained 3.75% over the month, and is up 17.22% year over year.",
    source: s.uraniumSpot,
  },
  {
    id: "haleu",
    group: "fuel",
    label: "HALEU availability",
    value: "Constrained",
    status: "stressed",
    context: "Centrus-X-energy prepayments; no US commercial output yet",
    detail:
      "A new Centrus-X-Energy enrichment and prepayment agreement eases the chicken-and-egg funding problem and backs Centrus's $3B contingent LEU/HALEU backlog, but there is still no commercial domestic HALEU source in production.",
    source: s.xEnergyCentrus,
  },
  {
    id: "data-center-pull",
    group: "demand",
    label: "Hyperscaler pull",
    value: "485 → 950 TWh",
    status: "elevated",
    context: "IEA data-centre demand: 2025 → 2030",
    detail:
      "The IEA expects data-centre electricity consumption to roughly double from 485 TWh in 2025 to 950 TWh in 2030, strengthening the case for firm clean power.",
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
      "Holtec reports that Palisades has completed its major restart projects and shifted to maintenance, testing, inspection, and operational-readiness work; more than 5,000 activities remain before fuel load and startup.",
    watchNext: "Watch fuel load, remaining NRC approvals, the startup milestone, and Crane's 2027 restart schedule.",
    source: s.palisades,
  },
  {
    id: "smr-commercial",
    track: "Reactors",
    label: "SMRs are demand-backed but not yet routine infrastructure",
    status: "elevated",
    summary:
      "TerraPower joined INPO in July, began Natrium construction in April after NRC approval, and on August 21 contracted Doosan Enerbility for key equipment for the first Kemmerer plant; Westinghouse completed zero-power criticality testing of its one-fifth-scale eVinci test reactor at NCERC on August 25, validating core design models. The progress is real, but commercial economics remain unproven.",
    watchNext: "Watch Doosan equipment deliveries to Wyoming, TerraPower's planned March 2028 Part 50 filing, eVinci prototyping and DOME test-bed access, and whether test-reactor wins convert into licensed commercial plants.",
    source: s.evinciCriticality,
  },
  {
    id: "haleu-bottleneck",
    track: "Fuel",
    label: "Advanced fuel is a gating item",
    status: "stressed",
    summary:
      "Fuel-cycle investment is advancing on both fronts: Standard Nuclear reports construction substantially complete at its Tennessee and Idaho TRISO sites (each up to 1 MTU/yr initially, scaling toward 5 MTU combined), while on August 6 Centrus and X-Energy signed an enrichment and prepayment agreement that eases the chicken-and-egg financing of domestic HALEU output. Supply still must scale and start producing at commercial volume.",
    watchNext: "Watch TRISO startup and licensing at both sites, initial MTU output, and whether Centrus's Piketon expansion delivers contracted LEU/HALEU volumes on schedule.",
    source: s.xEnergyCentrus,
  },
  {
    id: "corporate-ppa",
    track: "Customers",
    label: "Hyperscalers are becoming anchor customers",
    status: "elevated",
    summary:
      "Carnegie reports that Alphabet, Amazon, Meta, and Microsoft have all signed nuclear PPAs that could provide about 6.9 GW by the early 2030s, but projects still face regulatory and construction dependencies.",
    watchNext: "Watch named sites, binding offtake, NRC and interconnection progress, and whether 2027 restart targets become firm milestones.",
    source: s.carnegieHyperscalers,
  },
  {
    id: "geopolitics",
    track: "Risk",
    label: "Supply chain geopolitics define the investable map",
    status: "elevated",
    summary:
      "DOE's June 23 conditional loan package targets ten new large reactors through up to five loans for five two-reactor sites. The financing supports long-lead items, not funded builds without partners, equity, and final conditions.",
    watchNext: "Watch partner names, final loan conditions, roughly $1B-per-site equity commitments, and long-lead purchase orders.",
    source: s.doeLoans,
  },
  {
    id: "waste-licensing",
    track: "Risk",
    label: "Licensing and waste remain public-trust constraints",
    status: "neutral",
    summary:
      "The NRC's July proposal would revise ALARA and reactor-licensing requirements, but it is not final; meanwhile DOE amended WIPP's record of decision on August 18, adding seven disposal panels and extending the defense-waste repository's operating horizon about 50 years to roughly 2083 without raising the total TRU volume cap. Siting, waste, and public trust remain separate constraints on deployment.",
    watchNext: "Watch the 45-day comment process, final NRC rules, court challenges, WIPP panel excavation, and New Mexico's push to prioritize in-state waste shipments.",
    source: s.doeWipp,
  },
];

export function getNuclearDashboardData(): NuclearDashboardData {
  // Derive market caps from the single source of truth in nuclearLive.ts
  const playersWithCaps = players.map((p) => {
    const cap = p.ticker ? MARKET_CAP_USD_BY_SYMBOL[p.ticker] : undefined;
    return {
      ...p,
      marketCap: cap !== undefined ? formatMarketCap(cap) : p.marketCap,
    };
  });

  return {
    snapshotDate: "2026-08-31",
    verdict:
      "Nuclear's demand case remains strong, but the investable signal is still execution: the global fleet is about 400 GWe with about 75 reactors under construction, at least five DOE-authorised test reactors have now reached criticality (Oklo's Groves on August 6, with Westinghouse's eVinci completing cold criticality testing at NCERC on August 25), and DOE conditionally committed $17.5B for ten 1.1 GW AP1000s. None of those advanced-build milestones is commercial capacity yet—the pilots need full-power and licensing proof, and the loans need partners, equity, and final conditions. Uranium firmed further to $89.85/lb, while HALEU remains constrained despite new Centrus-X-Energy prepayment contracting. Existing fleets, restarts, and fuel/supply-chain bottlenecks still offer the clearest risk-adjusted exposure; advanced developers remain execution-heavy. Sources as of 2026-08-31: [WNA reactors](https://world-nuclear.org/information-library/current-and-future-generation/nuclear-power-in-the-world-today), [ANS eVinci criticality](https://www.ans.org/news/2026-08-26/article-8341/evinci-completes-cold-criticality-test-at-ncerc/), [DOE builds](https://www.energy.gov/articles/department-energy-announces-american-nuclear-supply-chain-loans), [Trading Economics](https://tradingeconomics.com/commodity/uranium), [WNN HALEU](https://www.world-nuclear-news.org/articles/x-energy-and-centrus-sign-haleu-supply-agreement), [ANS WIPP](https://www.ans.org/news/2026-08-27/article-8343/doe-plans-to-expand-wipps-capacity-and-operational-life/).",
    players: playersWithCaps,
    marketMetrics,
    techSignals,
  };
}

function formatMarketCap(usd: number): string {
  if (usd >= 1e12) return `$${(usd / 1e12).toFixed(1)}T`;
  if (usd >= 1e9) return `$${(usd / 1e9).toFixed(1)}B`;
  if (usd >= 1e6) return `$${(usd / 1e6).toFixed(0)}M`;
  return `$${usd}`;
}
