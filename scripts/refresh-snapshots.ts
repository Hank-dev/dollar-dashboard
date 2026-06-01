/**
 * Refreshes lib/aiSnapshot.json using Claude + web search.
 * Run: ENABLE_WEB_SEARCH=true ANTHROPIC_API_KEY=... npm run refresh:snapshots [-- --dry-run]
 * Guardrails: every changed field must cite a source; unverifiable fields keep
 * their prior value; the document is schema-validated before any write.
 */
import { readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import Anthropic from "@anthropic-ai/sdk";
import { validateAiSnapshot, type AiSnapshot } from "../lib/aiSnapshot.schema";

const ROSTER_IDS = {
  players: ["nvidia", "alphabet", "microsoft", "amazon", "broadcom", "meta", "openai", "anthropic", "databricks", "cursor", "perplexity"],
  marketMetrics: ["capex-race", "private-valuation", "consumer-scale", "coding-agent", "open-pressure", "agent-reliability"],
  techSignals: ["frontier-models", "agents", "ai-ides", "compute-stack"],
};

const SNAPSHOT_PATH = resolve(process.cwd(), "lib/aiSnapshot.json");
const MODEL = process.env.REFRESH_MODEL ?? "claude-sonnet-4-6";
const DRY_RUN = process.argv.includes("--dry-run");

function die(msg: string): never {
  console.error(`refresh-snapshots: ${msg}`);
  process.exit(1);
}

async function main() {
  if (process.env.ENABLE_WEB_SEARCH !== "true") die("set ENABLE_WEB_SEARCH=true to run the refresh");
  if (!process.env.ANTHROPIC_API_KEY) die("ANTHROPIC_API_KEY is required");

  const prior = JSON.parse(readFileSync(SNAPSHOT_PATH, "utf8")) as AiSnapshot;
  const anthropic = new Anthropic({ apiKey: process.env.ANTHROPIC_API_KEY });

  const system = [
    "You update a JSON snapshot for an AI-market dashboard. You are given the CURRENT snapshot.",
    "Use web search to verify each value. Return ONLY a JSON object with the SAME keys and structure as the input.",
    "Rules:",
    "- Keep the exact same top-level keys and the exact same ids under players/marketMetrics/techSignals. Do not add, remove, or rename ids.",
    "- For every field you change, set source.url to a web page you actually consulted and source.asOf to the publication/observation date (YYYY-MM-DD), and set source.confidence (high|medium|low).",
    "- If you cannot verify a field from a credible source, return its PRIOR value unchanged. Never guess, never blank.",
    "- status is one of: calm, neutral, elevated, stressed.",
    "- Keep prose fields roughly the same length and neutral analyst tone (adoptionSignal/aiExposure <= ~160 chars; detail/summary/watchNext <= ~220 chars; verdict.text <= ~600 chars).",
    "- Output strict JSON only. No markdown, no commentary.",
  ].join("\n");

  const response = await anthropic.messages.create({
    model: MODEL,
    max_tokens: 8000,
    tools: [{ type: "web_search_20250305", name: "web_search", max_uses: 8 }],
    system: [{ type: "text", text: system, cache_control: { type: "ephemeral" } }],
    messages: [
      { role: "user", content: `CURRENT snapshot:\n${JSON.stringify(prior, null, 2)}\n\nReturn the updated snapshot as strict JSON.` },
    ],
  });

  const text = response.content
    .filter((b): b is Anthropic.TextBlock => b.type === "text")
    .map((b) => b.text)
    .join("");
  const next = extractJson(text);
  if (!next) die(`model did not return parseable JSON (stop_reason: ${response.stop_reason ?? "unknown"})`);

  const merged = mergeKnownKeys(prior, next as AiSnapshot);
  const errors = validateAiSnapshot(merged, ROSTER_IDS);
  if (errors.length) die(`refreshed snapshot failed validation:\n${errors.join("\n")}`);

  const changed = diffSummary(prior, merged);
  if (DRY_RUN) {
    console.log("DRY RUN — changes that would be written:\n" + (changed || "(none)"));
    return;
  }
  writeFileSync(SNAPSHOT_PATH, JSON.stringify(merged, null, 2) + "\n", "utf8");
  console.log("Wrote lib/aiSnapshot.json\nChanges:\n" + (changed || "(none)"));
}

function extractJson(text: string): unknown {
  const trimmed = text.trim();
  try { return JSON.parse(trimmed); } catch { /* fall back to brace slice below */ }
  const start = trimmed.indexOf("{");
  const end = trimmed.lastIndexOf("}");
  if (start === -1 || end === -1 || end <= start) return null;
  try { return JSON.parse(trimmed.slice(start, end + 1)); } catch { return null; }
}

// Only copy keys that already exist in `prior` — prevents id drift / injection.
function mergeKnownKeys(prior: AiSnapshot, next: AiSnapshot): AiSnapshot {
  const out: AiSnapshot = JSON.parse(JSON.stringify(prior));
  out.verdict = { text: next.verdict?.text || prior.verdict.text, asOf: next.verdict?.asOf || prior.verdict.asOf };
  for (const id of Object.keys(prior.players)) if (next.players?.[id]) out.players[id] = next.players[id];
  for (const id of Object.keys(prior.marketMetrics)) if (next.marketMetrics?.[id]) out.marketMetrics[id] = next.marketMetrics[id];
  for (const id of Object.keys(prior.techSignals)) if (next.techSignals?.[id]) out.techSignals[id] = next.techSignals[id];
  return out;
}

function diffSummary(prior: AiSnapshot, next: AiSnapshot): string {
  const lines: string[] = [];
  const a = JSON.stringify(prior), b = JSON.stringify(next);
  if (a === b) return "";
  const walk = (pa: Record<string, unknown>, pb: Record<string, unknown>, path: string) => {
    for (const k of Object.keys(pb)) {
      const va = (pa as Record<string, unknown>)?.[k], vb = pb[k];
      if (typeof vb === "object" && vb) walk((va ?? {}) as Record<string, unknown>, vb as Record<string, unknown>, `${path}${k}.`);
      else if (JSON.stringify(va) !== JSON.stringify(vb)) lines.push(`${path}${k}: ${JSON.stringify(va)} → ${JSON.stringify(vb)}`);
    }
  };
  walk(prior as unknown as Record<string, unknown>, next as unknown as Record<string, unknown>, "");
  return lines.join("\n");
}

main().catch((e) => die(e instanceof Error ? e.message : String(e)));
