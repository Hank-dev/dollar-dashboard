import { NextResponse } from "next/server";
import type {
  XFeedItem,
  XSummaryCitation,
  XSummaryResponse,
} from "@/lib/xSummary";

const XAI_RESPONSES_URL = "https://api.x.ai/v1/responses";
const DEFAULT_MODEL = "grok-4.3";
const REFRESH_CADENCE_HOURS = 12;
const REFRESH_CADENCE_MS = REFRESH_CADENCE_HOURS * 60 * 60 * 1000;

export const dynamic = "force-dynamic";

let xSummaryCache:
  | {
      data: XSummaryResponse;
      expiresAt: number;
    }
  | null = null;

type XAiResponse = {
  output_text?: string;
  citations?: unknown[];
  output?: {
    content?: {
      text?: string;
      annotations?: unknown[];
    }[];
  }[];
};

export async function GET() {
  const apiKey = process.env.XAI_API_KEY;
  if (!apiKey) {
    return NextResponse.json(
      {
        error: "Missing XAI_API_KEY.",
        setup: "Add XAI_API_KEY to .env.local to enable live nuclear X summaries.",
      },
      { status: 503 },
    );
  }

  const nowMs = Date.now();
  if (xSummaryCache && xSummaryCache.expiresAt > nowMs) {
    return summaryJson({ ...xSummaryCache.data, cacheStatus: "cached" });
  }

  const now = new Date(nowMs);
  const from = new Date(now.getTime() - 24 * 60 * 60 * 1000);
  const model = process.env.XAI_SUMMARY_MODEL ?? DEFAULT_MODEL;

  const res = await fetch(XAI_RESPONSES_URL, {
    method: "POST",
    cache: "no-store",
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${apiKey}`,
    },
    body: JSON.stringify({
      model,
      input: [
        {
          role: "user",
          content: buildPrompt(from, now),
        },
      ],
      tools: [
        {
          type: "x_search",
          from_date: from.toISOString(),
          to_date: now.toISOString(),
        },
      ],
    }),
  });

  if (!res.ok) {
    const text = await res.text().catch(() => "");
    if (xSummaryCache) {
      return summaryJson({ ...xSummaryCache.data, cacheStatus: "stale" });
    }
    return NextResponse.json(
      {
        error: text || `xAI request failed (${res.status}).`,
      },
      { status: 502 },
    );
  }

  const body = (await res.json()) as XAiResponse;
  const summary = extractText(body).trim();
  const citations = extractCitations(body).slice(0, 8);
  const feed = extractFeedItems(summary, citations).slice(0, 8);
  const nextRefreshAt = new Date(nowMs + REFRESH_CADENCE_MS).toISOString();

  const data: XSummaryResponse = {
    checkedAt: now.toISOString(),
    nextRefreshAt,
    refreshCadenceHours: REFRESH_CADENCE_HOURS,
    cacheStatus: "fresh",
    fromDate: from.toISOString(),
    toDate: now.toISOString(),
    model,
    summary: summary || "No summary was returned by xAI.",
    citations,
    feed,
  };

  xSummaryCache = {
    data,
    expiresAt: nowMs + REFRESH_CADENCE_MS,
  };

  return summaryJson(data);
}

function summaryJson(data: XSummaryResponse) {
  return NextResponse.json(data, {
    headers: {
      "Cache-Control": `public, max-age=300, s-maxage=${REFRESH_CADENCE_MS / 1000}, stale-while-revalidate=3600`,
    },
  });
}

function buildPrompt(from: Date, to: Date): string {
  return `Search X posts from ${from.toISOString()} to ${to.toISOString()} for market-relevant discussion about nuclear energy.

Focus on nuclear power, uranium, enrichment, HALEU, SMRs, Constellation, Cameco, Centrus, Oklo, NuScale, BWXT, Kairos Power, X-energy, TerraPower, data-center power demand, hyperscaler nuclear PPAs, plant restarts, NRC licensing, DOE policy, IAEA/WNA data, and nuclear fuel-cycle supply chains.

Write a concise dashboard update for a developer and investor audience:
- 4 short bullets under "Top nuclear narratives"
- 2 short bullets under "What changed"
- 2 short bullets under "What to watch"

Rules:
- Use only information supported by X search results.
- Prefer recent primary-source posts, company accounts, researchers, builders, investors, credible reporters, regulators, and industry groups.
- Mention uncertainty where the conversation is rumor-driven.
- Do not give investment advice.
- Include citations in the response metadata when available.`;
}

function extractText(body: XAiResponse): string {
  if (typeof body.output_text === "string") return body.output_text;

  const parts: string[] = [];
  for (const output of body.output ?? []) {
    for (const content of output.content ?? []) {
      if (typeof content.text === "string") parts.push(content.text);
    }
  }
  return parts.join("\n");
}

function extractCitations(body: XAiResponse): XSummaryCitation[] {
  const citations: XSummaryCitation[] = [];
  const seen = new Set<string>();

  for (const item of body.citations ?? []) {
    addCitation(citations, seen, item);
  }

  for (const output of body.output ?? []) {
    for (const content of output.content ?? []) {
      for (const annotation of content.annotations ?? []) {
        addCitation(citations, seen, annotation);
      }
    }
  }

  return citations;
}

function addCitation(
  citations: XSummaryCitation[],
  seen: Set<string>,
  item: unknown,
) {
  if (typeof item === "string") {
    if (seen.has(item)) return;
    seen.add(item);
    citations.push({ title: item, url: item.startsWith("http") ? item : undefined });
    return;
  }

  if (!item || typeof item !== "object") return;
  const record = item as Record<string, unknown>;
  const url = firstString(record.url, record.uri, record.link);
  const title =
    firstString(record.title, record.text, record.display_name, record.username) ??
    url ??
    "X source";
  const key = url ?? title;
  if (seen.has(key)) return;
  seen.add(key);
  citations.push({ title, url });
}

function firstString(...values: unknown[]): string | undefined {
  return values.find((value): value is string => typeof value === "string");
}

function extractFeedItems(
  summary: string,
  citations: XSummaryCitation[],
): XFeedItem[] {
  const feed: XFeedItem[] = [];
  const seen = new Set<string>();

  for (const citation of citations) {
    if (!citation.url || !isXPostUrl(citation.url)) continue;
    addFeedItem(feed, seen, {
      title: citation.title,
      url: citation.url,
      handle: extractHandle(citation.url),
    });
  }

  for (const url of summary.match(/https:\/\/(?:x|twitter)\.com\/[^\s)\]]+/g) ?? []) {
    if (!isXPostUrl(url)) continue;
    addFeedItem(feed, seen, {
      title: "Referenced X post",
      url,
      handle: extractHandle(url),
    });
  }

  return feed;
}

function addFeedItem(
  feed: XFeedItem[],
  seen: Set<string>,
  item: XFeedItem,
) {
  if (seen.has(item.url)) return;
  seen.add(item.url);
  feed.push(item);
}

function isXPostUrl(url: string): boolean {
  return /^https:\/\/(?:x|twitter)\.com\/[^/]+\/status\/\d+/.test(url);
}

function extractHandle(url: string): string | undefined {
  return url.match(/^https:\/\/(?:x|twitter)\.com\/([^/]+)/)?.[1];
}
