// One implementation behind /api/ai/x-summary and /api/nuclear/x-summary.
// Each xAI search is billed, so a result is kept for twelve hours, concurrent
// requests share one call, and a failed refresh serves the last good summary.
import { NextResponse } from "next/server";
import { cached } from "./routeCache";
import type { XFeedItem, XSummaryCitation, XSummaryResponse } from "./xSummary";

const XAI_RESPONSES_URL = "https://api.x.ai/v1/responses";
const DEFAULT_MODEL = "grok-4.3";
const REFRESH_CADENCE_HOURS = 12;
const REFRESH_CADENCE_MS = REFRESH_CADENCE_HOURS * 60 * 60 * 1000;
const REQUEST_TIMEOUT_MS = 90_000;

export interface XSummaryTopic {
  /** Cache key and log label, e.g. "ai". */
  key: string;
  /** Completes "market-relevant discussion about …". */
  subject: string;
  /** Completes "Focus on …". */
  focus: string;
  /** Heading for the four narrative bullets. */
  narrativesHeading: string;
  /** Completes "Prefer recent primary-source posts, …". */
  preferredSources: string;
}

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

class UpstreamStatus extends Error {
  constructor(readonly status: number) {
    super(`xAI responded ${status}`);
  }
}

export function createXSummaryRoute(topic: XSummaryTopic): () => Promise<Response> {
  return async function GET() {
    const apiKey = process.env.XAI_API_KEY?.trim();
    if (!apiKey || apiKey === "insert_key") {
      return NextResponse.json(
        {
          error: "Missing XAI_API_KEY.",
          setup: "Add XAI_API_KEY to .env.local to enable live X summaries.",
        },
        { status: 503 },
      );
    }

    try {
      const hit = await cached<XSummaryResponse>(
        `x-summary:${topic.key}`,
        { ttlSeconds: REFRESH_CADENCE_MS / 1000 },
        () => searchX(topic, apiKey),
      );
      return NextResponse.json(
        { ...hit.value, cacheStatus: hit.status },
        {
          headers: {
            "Cache-Control": `public, max-age=300, s-maxage=${REFRESH_CADENCE_MS / 1000}, stale-while-revalidate=3600`,
          },
        },
      );
    } catch (err) {
      console.error(`xAI ${topic.key} summary failed:`, err instanceof Error ? err.message : err);
      const detail =
        err instanceof UpstreamStatus ? ` (upstream status ${err.status})` : "";
      return NextResponse.json(
        { error: `Live X summary is temporarily unavailable${detail}.` },
        { status: 502 },
      );
    }
  };
}

async function searchX(topic: XSummaryTopic, apiKey: string): Promise<XSummaryResponse> {
  const now = new Date();
  const from = new Date(now.getTime() - 24 * 60 * 60 * 1000);
  const model = process.env.XAI_SUMMARY_MODEL ?? DEFAULT_MODEL;

  const res = await fetch(XAI_RESPONSES_URL, {
    method: "POST",
    cache: "no-store",
    signal: AbortSignal.timeout(REQUEST_TIMEOUT_MS),
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${apiKey}`,
    },
    body: JSON.stringify({
      model,
      input: [{ role: "user", content: buildPrompt(topic, from, now) }],
      tools: [
        {
          type: "x_search",
          from_date: from.toISOString(),
          to_date: now.toISOString(),
        },
      ],
    }),
  });
  if (!res.ok) throw new UpstreamStatus(res.status);

  const body = (await res.json()) as XAiResponse;
  const summary = extractText(body).trim();
  // An empty answer is a failed search, not a result worth keeping for 12 hours.
  if (!summary) throw new Error("xAI returned no summary text");
  const citations = extractCitations(body).slice(0, 8);

  return {
    checkedAt: now.toISOString(),
    nextRefreshAt: new Date(now.getTime() + REFRESH_CADENCE_MS).toISOString(),
    refreshCadenceHours: REFRESH_CADENCE_HOURS,
    cacheStatus: "fresh",
    fromDate: from.toISOString(),
    toDate: now.toISOString(),
    model,
    summary,
    citations,
    feed: extractFeedItems(summary, citations).slice(0, 8),
  };
}

function buildPrompt(topic: XSummaryTopic, from: Date, to: Date): string {
  return `Search X posts from ${from.toISOString()} to ${to.toISOString()} for market-relevant discussion about ${topic.subject}.

Focus on ${topic.focus}.

Write a concise dashboard update for a developer and investor audience:
- 4 short bullets under "${topic.narrativesHeading}"
- 2 short bullets under "What changed"
- 2 short bullets under "What to watch"

Rules:
- Use only information supported by X search results.
- Prefer recent primary-source posts, ${topic.preferredSources}.
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
  const rawUrl = firstString(record.url, record.uri, record.link);
  const url = rawUrl && /^https?:\/\//i.test(rawUrl) ? rawUrl : undefined;
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
