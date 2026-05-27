// Scrapes the Farside spot-BTC-ETF daily flows table.
// Source: https://farside.co.uk/bitcoin-etf-flow-all-data/
import type { EtfFlowDay } from "@/lib/btc/types";
import { SECONDS_PER_DAY } from "@/lib/btc/calc/util";

const URL = "https://farside.co.uk/bitcoin-etf-flow-all-data/";

const MONTHS: Record<string, number> = {
  jan: 0, feb: 1, mar: 2, apr: 3, may: 4, jun: 5,
  jul: 6, aug: 7, sep: 8, oct: 9, nov: 10, dec: 11,
};

function parseDate(s: string): number | null {
  // "12 May 2026"
  const m = s.trim().match(/^(\d{1,2})\s+([A-Za-z]{3})\s+(\d{4})$/);
  if (!m) return null;
  const day = parseInt(m[1], 10);
  const mon = MONTHS[m[2].toLowerCase()];
  const year = parseInt(m[3], 10);
  if (mon == null || !Number.isFinite(day) || !Number.isFinite(year)) return null;
  return Math.floor(Date.UTC(year, mon, day) / 1000);
}

function parseValue(raw: string): number | null {
  // Farside uses parentheses for negatives sometimes and "-" for no data.
  const s = raw.replace(/<[^>]+>/g, "").trim();
  if (s === "" || s === "-" || s === "—") return null;
  const neg = s.startsWith("(") && s.endsWith(")");
  const cleaned = s
    .replace(/[()]/g, "")
    .replace(/,/g, "")
    .replace(/[^\d.\-]/g, "");
  if (cleaned === "" || cleaned === "-") return null;
  const v = parseFloat(cleaned);
  if (!Number.isFinite(v)) return null;
  return neg ? -v : v;
}

function stripTags(s: string): string {
  return s.replace(/<[^>]+>/g, "").replace(/&nbsp;/gi, " ").trim();
}

export async function fetchFarsideFlows(): Promise<EtfFlowDay[]> {
  const res = await fetch(URL, {
    next: { revalidate: 6 * 3600 }, // 6h
    headers: {
      "User-Agent":
        "Mozilla/5.0 (compatible; btc-terminal/1.0; +single-user dashboard)",
      Accept: "text/html,application/xhtml+xml",
    },
  });
  if (!res.ok) throw new Error(`Farside ${res.status}: ${await res.text()}`);
  const html = await res.text();

  // Target the flows table specifically. The page has several <table>s — the
  // first one is the site header, not the data. The data lives in
  // <table class="etf">.
  const tableMatch = html.match(/<table[^>]*class="etf"[^>]*>[\s\S]*?<\/table>/i);
  if (!tableMatch) throw new Error("Farside: no etf table found");
  const table = tableMatch[0];

  // Extract rows.
  const rowRe = /<tr[^>]*>([\s\S]*?)<\/tr>/gi;
  const rows: string[][] = [];
  let m: RegExpExecArray | null;
  while ((m = rowRe.exec(table)) !== null) {
    const cellRe = /<t[hd][^>]*>([\s\S]*?)<\/t[hd]>/gi;
    const cells: string[] = [];
    let c: RegExpExecArray | null;
    while ((c = cellRe.exec(m[1])) !== null) {
      cells.push(stripTags(c[1]));
    }
    if (cells.length > 0) rows.push(cells);
  }
  if (rows.length === 0) throw new Error("Farside: no rows parsed");

  // Locate header row: the first row whose first cell contains "Date".
  let headerIdx = -1;
  for (let i = 0; i < rows.length; i++) {
    const c0 = rows[i][0].toLowerCase();
    if (c0.includes("date")) {
      headerIdx = i;
      break;
    }
  }
  if (headerIdx < 0) throw new Error("Farside: no header row");
  const header = rows[headerIdx].map((h) => h.toLowerCase());

  const findCol = (needle: string): number =>
    header.findIndex((h) => h.includes(needle));
  const idxIbit = findCol("ibit");
  const idxFbtc = findCol("fbtc");
  const idxArkb = findCol("arkb");
  const idxTotal = findCol("total");

  const out: EtfFlowDay[] = [];
  for (let i = headerIdx + 1; i < rows.length; i++) {
    const r = rows[i];
    if (r.length < 2) continue;
    const t = parseDate(r[0]);
    if (t == null) continue; // skip summary rows like "Week", "Month", footers
    out.push({
      t,
      ibit: idxIbit >= 0 ? parseValue(r[idxIbit] ?? "") : null,
      fbtc: idxFbtc >= 0 ? parseValue(r[idxFbtc] ?? "") : null,
      arkb: idxArkb >= 0 ? parseValue(r[idxArkb] ?? "") : null,
      total: idxTotal >= 0 ? parseValue(r[idxTotal] ?? "") : null,
    });
  }

  // Sort ascending by date; dedupe by t (keep first occurrence).
  out.sort((a, b) => a.t - b.t);
  const seen = new Set<number>();
  return out.filter((d) => {
    if (seen.has(d.t)) return false;
    seen.add(d.t);
    return true;
  });
}

export function summariseFlows(days: EtfFlowDay[]): {
  cumulativeTotal: number;
  flow7dAvg: number;
  flow30dAvg: number;
  latestTotal: number | null;
  latestDate: number;
} {
  if (days.length === 0) {
    return {
      cumulativeTotal: 0,
      flow7dAvg: 0,
      flow30dAvg: 0,
      latestTotal: null,
      latestDate: 0,
    };
  }
  let cum = 0;
  for (const d of days) if (d.total != null) cum += d.total;

  const latest = days[days.length - 1];
  const cutoff7 = latest.t - 7 * SECONDS_PER_DAY;
  const cutoff30 = latest.t - 30 * SECONDS_PER_DAY;
  let sum7 = 0;
  let n7 = 0;
  let sum30 = 0;
  let n30 = 0;
  for (const d of days) {
    if (d.total == null) continue;
    if (d.t > cutoff7) {
      sum7 += d.total;
      n7++;
    }
    if (d.t > cutoff30) {
      sum30 += d.total;
      n30++;
    }
  }
  return {
    cumulativeTotal: cum,
    flow7dAvg: n7 > 0 ? sum7 / n7 : 0,
    flow30dAvg: n30 > 0 ? sum30 / n30 : 0,
    latestTotal: latest.total,
    latestDate: latest.t,
  };
}
