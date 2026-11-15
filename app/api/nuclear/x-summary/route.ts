import { createXSummaryRoute } from "@/lib/xSummaryRoute";

export const dynamic = "force-dynamic";

export const GET = createXSummaryRoute({
  key: "nuclear",
  subject: "nuclear energy",
  focus:
    "nuclear power, uranium, enrichment, HALEU, SMRs, Constellation, Cameco, Centrus, Oklo, NuScale, BWXT, Kairos Power, X-energy, TerraPower, data-center power demand, hyperscaler nuclear PPAs, plant restarts, NRC licensing, DOE policy, IAEA/WNA data, and nuclear fuel-cycle supply chains",
  narrativesHeading: "Top nuclear narratives",
  preferredSources:
    "company accounts, researchers, builders, investors, credible reporters, regulators, and industry groups",
});
