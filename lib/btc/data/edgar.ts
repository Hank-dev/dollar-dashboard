// SEC EDGAR fetcher for MicroStrategy / Strategy 8-K capital-markets filings.
// EDGAR requires a User-Agent identifying the requester and rate-limits at
// 10 req/s. We stay well below.
import type {
  StrategyAtmRow,
  StrategyFiling,
  StrategyTicker,
} from "@/lib/btc/types";

const CIK = "0001050446"; // MicroStrategy / Strategy
const UA =
  "btc-terminal/1.0 johanneshankoe@gmail.com (single-user dashboard, contact via email)";
const COMMON_HEADERS = {
  "User-Agent": UA,
  Accept: "application/atom+xml, text/html, */*",
};

const TICKERS: StrategyTicker[] = ["STRC", "STRF", "STRK", "STRD", "MSTR"];

const ATOM_URL = `https://www.sec.gov/cgi-bin/browse-edgar?action=getcompany&CIK=${CIK}&type=8-K&dateb=&owner=include&count=20&output=atom`;

type FilingHandle = {
  accession: string; // dashed form
  filedAt: number;
  indexUrl: string;
};

function parseAtom(xml: string): FilingHandle[] {
  const out: FilingHandle[] = [];
  const entryRe = /<entry>([\s\S]*?)<\/entry>/g;
  let m: RegExpExecArray | null;
  while ((m = entryRe.exec(xml)) !== null) {
    const entry = m[1];
    const idMatch = entry.match(/<id>[^<]*?accession-number=([\d-]+)/);
    const linkMatch = entry.match(/<link[^>]+href="([^"]+)"/);
    const updMatch = entry.match(/<updated>([^<]+)<\/updated>/);
    if (!idMatch || !linkMatch || !updMatch) continue;
    const filedAt = Math.floor(new Date(updMatch[1]).getTime() / 1000);
    if (!Number.isFinite(filedAt)) continue;
    out.push({
      accession: idMatch[1],
      filedAt,
      indexUrl: linkMatch[1],
    });
  }
  return out;
}

async function findPrimaryDoc(indexUrl: string): Promise<string | null> {
  const res = await fetch(indexUrl, {
    next: { revalidate: 86400 },
    headers: COMMON_HEADERS,
  });
  if (!res.ok) return null;
  const html = await res.text();
  // Primary doc is typically named mstr-YYYYMMDD.htm in the same archive dir.
  // Fallback: any non-index .htm under /Archives/edgar/data/.../*.htm
  const m =
    html.match(
      /href="(\/Archives\/edgar\/data\/[^"]+?\/mstr-\d{8}\.htm)"/i,
    ) ??
    html.match(
      /href="(\/ix\?doc=\/Archives\/edgar\/data\/[^"]+?\/mstr-\d{8}\.htm)"/i,
    );
  if (!m) return null;
  let path = m[1];
  if (path.startsWith("/ix?doc=")) path = path.slice("/ix?doc=".length);
  return `https://www.sec.gov${path}`;
}

function stripTags(s: string): string {
  return s
    .replace(/<[^>]+>/g, " ")
    .replace(/&nbsp;/gi, " ")
    .replace(/&amp;/g, "&")
    .replace(/&#\d+;/g, " ")
    .replace(/\s+/g, " ")
    .trim();
}

function parseNumberLoose(raw: string): number {
  // "1,412" or "$ 19,462.8" or "- " or "(1.2)"
  const s = raw.replace(/[$,\s]/g, "");
  if (s === "" || s === "-" || s === "—") return 0;
  const neg = s.startsWith("(") && s.endsWith(")");
  const cleaned = s.replace(/[()]/g, "");
  const v = parseFloat(cleaned);
  if (!Number.isFinite(v)) return 0;
  return neg ? -v : v;
}

function periodEndFromUrl(url: string): number | null {
  const m = url.match(/mstr-(\d{4})(\d{2})(\d{2})\.htm/i);
  if (!m) return null;
  const y = parseInt(m[1], 10);
  const mo = parseInt(m[2], 10) - 1;
  const d = parseInt(m[3], 10);
  return Math.floor(Date.UTC(y, mo, d) / 1000);
}

// Parse the ATM activity table from a filing's primary doc.
// The table has rows of the form:
//   {TICKER} Stock  {shares}  $ {notional}  $ {netProceeds}  $ {available}
// followed by the descriptive name of the security.
export function parseAtmTable(doc: string): StrategyAtmRow[] {
  const text = stripTags(doc);
  const rows: StrategyAtmRow[] = [];
  // Anchor: each ticker followed by "Stock" then 4 dollar-ish columns. Numbers
  // can be "-" for no activity. We capture greedily up to either the next
  // ticker, "Total", or end-of-block.
  const numCol = String.raw`(?:\$?\s*(?:[\d,]+(?:\.\d+)?|-|—))`;
  const re = new RegExp(
    String.raw`\b(STRC|STRF|STRK|STRD|MSTR)\s+(?:Stock|Common\s+Stock)\s+(${numCol})\s+\$?\s*(${numCol})\s+\$?\s*(${numCol})\s+\$?\s*(${numCol})`,
    "gi",
  );
  const seen = new Set<string>();
  let m: RegExpExecArray | null;
  while ((m = re.exec(text)) !== null) {
    const ticker = m[1].toUpperCase() as StrategyTicker;
    if (seen.has(ticker)) continue; // first match per ticker
    seen.add(ticker);
    rows.push({
      ticker,
      sharesSold: parseNumberLoose(m[2]),
      notionalM: parseNumberLoose(m[3]),
      netProceedsM: parseNumberLoose(m[4]),
      availableM: parseNumberLoose(m[5]),
    });
  }
  return rows;
}

async function fetchAndParse(handle: FilingHandle): Promise<StrategyFiling> {
  const docUrl = await findPrimaryDoc(handle.indexUrl);
  if (!docUrl) {
    return {
      accession: handle.accession,
      filedAt: handle.filedAt,
      periodEnd: null,
      rows: [],
    };
  }
  const periodEnd = periodEndFromUrl(docUrl);
  const res = await fetch(docUrl, {
    next: { revalidate: 86400 },
    headers: COMMON_HEADERS,
  });
  if (!res.ok) {
    return {
      accession: handle.accession,
      filedAt: handle.filedAt,
      periodEnd,
      rows: [],
    };
  }
  const html = await res.text();
  const rows = parseAtmTable(html);
  return {
    accession: handle.accession,
    filedAt: handle.filedAt,
    periodEnd,
    rows,
  };
}

// Returns filings with ATM activity, most recent first. Caps EDGAR fan-out at
// `maxFilings` to keep cache-warming bounded.
export async function fetchMstrAtmFilings(
  maxFilings = 16,
): Promise<StrategyFiling[]> {
  const atomRes = await fetch(ATOM_URL, {
    next: { revalidate: 86400 },
    headers: COMMON_HEADERS,
  });
  if (!atomRes.ok) throw new Error(`EDGAR atom ${atomRes.status}`);
  const atom = await atomRes.text();
  const handles = parseAtom(atom).slice(0, maxFilings);

  // Sequential with a tiny gap to stay polite (EDGAR limit is 10 req/s).
  const filings: StrategyFiling[] = [];
  for (const h of handles) {
    try {
      filings.push(await fetchAndParse(h));
    } catch {
      // skip individual failures
    }
  }
  // Keep only filings whose ATM table actually parsed.
  return filings.filter((f) => f.rows.length > 0);
}

export const STRATEGY_TICKERS = TICKERS;
