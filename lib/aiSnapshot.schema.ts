// Shape + runtime validation for lib/aiSnapshot.json. Used by aiMetrics.ts
// (validate on read) and scripts/refresh-snapshots.ts (validate before write).
import type { Status } from "./metrics";
import type { Confidence } from "./aiMetrics";

export interface SnapshotSource {
  name: string;
  url: string;
  asOf: string; // YYYY-MM-DD
  confidence: Confidence;
}

export interface SnapshotPlayer {
  marketCap?: string;
  valuationEstimate?: string;
  adoptionSignal: string;
  aiExposure: string;
  status: Status;
  source: SnapshotSource;
}

export interface SnapshotMetric {
  value: string;
  context: string;
  detail: string;
  status: Status;
  source: SnapshotSource;
}

export interface SnapshotSignal {
  label: string;
  summary: string;
  watchNext: string;
  status: Status;
  source: SnapshotSource;
}

export interface AiSnapshot {
  verdict: { text: string; asOf: string };
  players: Record<string, SnapshotPlayer>;
  marketMetrics: Record<string, SnapshotMetric>;
  techSignals: Record<string, SnapshotSignal>;
}

const STATUSES: Status[] = ["calm", "neutral", "elevated", "stressed"];
const CONFIDENCES: Confidence[] = ["high", "medium", "low"];
const ISO_DATE = /^\d{4}-\d{2}-\d{2}$/;

function str(v: unknown, max = 600): v is string {
  return typeof v === "string" && v.trim().length > 0 && v.length <= max;
}

function isSource(v: unknown): v is SnapshotSource {
  if (typeof v !== "object" || v === null) return false;
  const s = v as SnapshotSource;
  return (
    str(s.name, 200) &&
    str(s.url, 500) &&
    typeof s.asOf === "string" &&
    ISO_DATE.test(s.asOf) &&
    CONFIDENCES.includes(s.confidence)
  );
}

// Returns [] when valid, otherwise a list of human-readable problems.
export function validateAiSnapshot(
  data: unknown,
  rosterIds: { players: string[]; marketMetrics: string[]; techSignals: string[] },
): string[] {
  const errors: string[] = [];
  if (typeof data !== "object" || data === null) return ["snapshot is not an object"];
  const snap = data as AiSnapshot;

  if (!snap.verdict || !str(snap.verdict.text, 1200) || !ISO_DATE.test(snap.verdict.asOf ?? "")) {
    errors.push("verdict: missing text or invalid asOf");
  }

  for (const id of rosterIds.players) {
    const p = snap.players?.[id];
    if (!p) { errors.push(`players.${id}: missing`); continue; }
    if (!p.marketCap && !p.valuationEstimate)
      errors.push(`players.${id}: needs marketCap or valuationEstimate`);
    if (!str(p.adoptionSignal) || !str(p.aiExposure)) errors.push(`players.${id}: prose invalid`);
    if (!STATUSES.includes(p.status)) errors.push(`players.${id}: bad status`);
    if (!isSource(p.source)) errors.push(`players.${id}: bad source`);
  }

  for (const id of rosterIds.marketMetrics) {
    const m = snap.marketMetrics?.[id];
    if (!m) { errors.push(`marketMetrics.${id}: missing`); continue; }
    if (!str(m.value, 60) || !str(m.context, 120) || !str(m.detail)) errors.push(`marketMetrics.${id}: prose invalid`);
    if (!STATUSES.includes(m.status)) errors.push(`marketMetrics.${id}: bad status`);
    if (!isSource(m.source)) errors.push(`marketMetrics.${id}: bad source`);
  }

  for (const id of rosterIds.techSignals) {
    const t = snap.techSignals?.[id];
    if (!t) { errors.push(`techSignals.${id}: missing`); continue; }
    if (!str(t.label, 120) || !str(t.summary) || !str(t.watchNext)) errors.push(`techSignals.${id}: prose invalid`);
    if (!STATUSES.includes(t.status)) errors.push(`techSignals.${id}: bad status`);
    if (!isSource(t.source)) errors.push(`techSignals.${id}: bad source`);
  }

  return errors;
}
