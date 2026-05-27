import { NextResponse } from "next/server";
import {
  FRONTIER_MODEL_CANDIDATES,
  type FrontierModelCandidate,
  type FrontierModelPoint,
  type FrontierModelsResponse,
} from "@/lib/frontierModels";

const AI_API_COST_BASE = "https://www.aiapicost.com";
const OPENROUTER_MODELS_URL = "https://openrouter.ai/api/v1/models";

type OpenRouterModel = {
  id: string;
  pricing?: {
    prompt?: string;
    completion?: string;
  };
};

export async function GET() {
  const openRouterById = await fetchOpenRouterPrices();
  const settled = await Promise.allSettled(
    FRONTIER_MODEL_CANDIDATES.map((candidate) =>
      fetchFrontierPoint(candidate, openRouterById),
    ),
  );

  const points: FrontierModelPoint[] = [];
  const errors: FrontierModelsResponse["errors"] = [];

  settled.forEach((result, index) => {
    const candidate = FRONTIER_MODEL_CANDIDATES[index];
    if (result.status === "fulfilled") {
      points.push(result.value);
    } else {
      errors.push({
        id: candidate.id,
        message:
          result.reason instanceof Error
            ? result.reason.message
            : "Could not load model data.",
      });
    }
  });

  points.sort((a, b) => b.intelligenceIndex - a.intelligenceIndex);

  return NextResponse.json({
    capturedAt: new Date().toISOString(),
    points,
    errors,
  } satisfies FrontierModelsResponse);
}

async function fetchOpenRouterPrices(): Promise<Map<string, OpenRouterModel>> {
  try {
    const res = await fetch(OPENROUTER_MODELS_URL, {
      cache: "no-store",
      headers: { Accept: "application/json" },
    });
    if (!res.ok) return new Map();
    const body = (await res.json()) as { data?: OpenRouterModel[] };
    return new Map((body.data ?? []).map((model) => [model.id, model]));
  } catch {
    return new Map();
  }
}

async function fetchFrontierPoint(
  candidate: FrontierModelCandidate,
  openRouterById: Map<string, OpenRouterModel>,
): Promise<FrontierModelPoint> {
  const sourceUrl = `${AI_API_COST_BASE}${candidate.aiApiCostPath}`;
  const res = await fetch(sourceUrl, {
    cache: "no-store",
    headers: {
      Accept: "text/html,application/xhtml+xml",
      "User-Agent": "dollar-dashboard/0.1 frontier-model-monitor",
    },
  });

  if (!res.ok) {
    throw new Error(`AI API Cost returned ${res.status}`);
  }

  const html = await res.text();
  const intelligenceIndex = parseNumber(
    html,
    /Intelligence Index:\s*([0-9.]+)/,
    /Intelligence Index<\/a><span[^>]*>([0-9.]+)<\/span>/,
  );
  const pageInput = parseNumber(
    html,
    /Input:\s*\$([0-9.]+)\/M tokens/i,
    /:\s*\$([0-9.]+)\/M input/i,
  );
  const pageOutput = parseNumber(
    html,
    /Output:\s*\$([0-9.]+)\/M tokens/i,
    /\$[0-9.]+\/M input,\s*\$([0-9.]+)\/M output/i,
  );

  if (intelligenceIndex == null) {
    throw new Error("Missing Intelligence Index");
  }

  const openRouterModel = candidate.openRouterId
    ? openRouterById.get(candidate.openRouterId)
    : undefined;
  const openRouterInput = parsePricePerToken(openRouterModel?.pricing?.prompt);
  const openRouterOutput = parsePricePerToken(openRouterModel?.pricing?.completion);

  const inputUsdPerMillion = pageInput ?? openRouterInput;
  const outputUsdPerMillion = pageOutput ?? openRouterOutput;
  const pricingSource =
    pageInput != null && pageOutput != null ? "AI API Cost" : "OpenRouter";

  if (inputUsdPerMillion == null || outputUsdPerMillion == null) {
    throw new Error("Missing token pricing");
  }

  const blendedUsdPerMillion = (inputUsdPerMillion * 3 + outputUsdPerMillion) / 4;

  return {
    id: candidate.id,
    label: candidate.label,
    provider: candidate.provider,
    intelligenceIndex,
    inputUsdPerMillion,
    outputUsdPerMillion,
    blendedUsdPerMillion,
    blendedUsdPerToken: blendedUsdPerMillion / 1_000_000,
    sourceUrl,
    pricingSource,
  };
}

function parseNumber(source: string, ...patterns: RegExp[]): number | null {
  for (const pattern of patterns) {
    const match = source.match(pattern);
    if (!match) continue;
    const value = Number(match[1]);
    if (Number.isFinite(value)) return value;
  }
  return null;
}

function parsePricePerToken(value: string | undefined): number | null {
  if (!value) return null;
  const pricePerToken = Number(value);
  if (!Number.isFinite(pricePerToken) || pricePerToken <= 0) return null;
  return pricePerToken * 1_000_000;
}
