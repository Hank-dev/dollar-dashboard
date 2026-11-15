use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const GENESIS_UNIX: i64 = 1_230_940_800;
pub const POWER_LAW_CUTOFF: i64 = 1_279_324_800;
/// Zcash genesis block, 28 Oct 2016 (UTC midnight).
pub const ZEC_GENESIS_UNIX: i64 = 1_477_612_800;
/// Skip the launch-week auction prices (ZEC traded above $2,000 for days); fit from 1 Jan 2017.
pub const ZEC_POWER_LAW_CUTOFF: i64 = 1_483_228_800;
pub const LAST_HALVING_UNIX: i64 = 1_713_571_200;
pub const SECONDS_PER_DAY: i64 = 86_400;

pub const STANDING_NOTE: &str = "Equities and volatility can look calm while the long end and dollar funding look tight. The open question is which side converges. This note is a standing frame, not a live model call.";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Status {
    Calm,
    Neutral,
    Elevated,
    Stressed,
}

impl Status {
    pub fn label(self) -> &'static str {
        match self {
            Self::Calm => "Calm",
            Self::Neutral => "Neutral",
            Self::Elevated => "Elevated",
            Self::Stressed => "Stressed",
        }
    }

    pub fn from_snapshot(value: &str) -> Self {
        match value {
            "calm" => Self::Calm,
            "elevated" => Self::Elevated,
            "stressed" => Self::Stressed,
            _ => Self::Neutral,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Group {
    Rates,
    Dollar,
    Risk,
}

impl Group {
    pub fn title(self) -> &'static str {
        match self {
            Self::Rates => "US rates and fiscal stress",
            Self::Dollar => "Dollar and the yen carry",
            Self::Risk => "Risk appetite and havens",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Obs {
    pub date: String,
    pub value: f64,
    pub source: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CurvePoint {
    pub label: String,
    pub yield_pct: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CurveDay {
    pub date: String,
    pub points: Vec<CurvePoint>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub t: i64,
    pub v: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Quote {
    pub symbol: String,
    pub price: f64,
    pub change_pct: f64,
    pub market_cap: Option<f64>,
    /// Session change in price units; yields use it for basis points.
    #[serde(default)]
    pub change_abs: Option<f64>,
}

/// Timing and provenance travel with an individual price, never with an entire
/// partially successful refresh. A receipt time is not necessarily a trade time.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PriceStamp {
    pub acquired_unix: i64,
    #[serde(default)]
    pub observed_unix: Option<i64>,
    pub source: String,
}

impl PriceStamp {
    pub fn live_spot(&self, now: i64) -> bool {
        // The exchange clock can lead the local one by a few seconds.
        self.observed_unix
            .is_some_and(|t| (-5..8).contains(&(now - t)))
            && (0..8).contains(&(now - self.acquired_unix))
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DollarBook {
    #[serde(default)]
    pub curve_history: Vec<CurveDay>,
    pub curve: Vec<CurvePoint>,
    pub curve_date: String,
    pub ust2: Option<Obs>,
    pub ust10: Option<Obs>,
    pub ust30: Option<Obs>,
    pub fed_lo: Option<Obs>,
    pub fed_hi: Option<Obs>,
    pub fed_hi_6m: Option<Obs>,
    pub dxy: Option<Obs>,
    pub dxy_broad: Option<Obs>,
    pub dxy_spark: Vec<f64>,
    pub usdjpy: Option<Obs>,
    pub jgb10: Option<Obs>,
    pub vix: Option<Obs>,
    pub spx: Option<Obs>,
    pub gold: Option<Obs>,
    pub brent: Option<Obs>,
    /// Bank of Japan policy rate (TradingView economics series).
    #[serde(default)]
    pub boj: Option<Obs>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BtcBook {
    pub price: Option<f64>,
    pub price_source: String,
    pub change_24h_pct: Option<f64>,
    pub high_24h: Option<f64>,
    pub low_24h: Option<f64>,
    pub eth_price: Option<f64>,
    pub eth_change_24h_pct: Option<f64>,
    pub zec_price: Option<f64>,
    pub zec_change_24h_pct: Option<f64>,
    pub daily: Vec<Point>,
    pub weekly: Vec<Point>,
    pub spark: Vec<f64>,
    pub fng: Option<i32>,
    pub fng_label: String,
    pub fng_hist: Vec<f64>,
    pub funding: Option<f64>,
    pub funding_hist: Vec<f64>,
    /// Binance ETHUSDT daily closes, newest last.
    #[serde(default)]
    pub eth_daily: Vec<Point>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct MacroPrint {
    pub label: String,
    pub current: Option<f64>,
    pub delta_30d: Option<f64>,
    pub unit: String,
    /// Date of the latest observation, YYYY-MM-DD.
    #[serde(default)]
    pub as_of: String,
    /// The observation before the latest one.
    #[serde(default)]
    pub prev: Option<f64>,
    /// Recent observations, newest last.
    #[serde(default)]
    pub hist: Vec<f64>,
    #[serde(default)]
    pub source: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct VenuePrint {
    pub name: String,
    pub current: Option<f64>,
    pub avg_7d: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct CorrPrint {
    pub label: String,
    pub r30: Option<f64>,
    pub r90: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct ScorePrint {
    pub label: String,
    pub score: i32,
    pub detail: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct ModelPrint {
    pub label: String,
    #[serde(default)]
    pub creator: String,
    pub intelligence: Option<f64>,
    pub blended: f64,
    pub input: f64,
    pub output: f64,
    /// Per-benchmark results from the Artificial Analysis leaderboard, keyed by its benchmark id
    /// (see `BENCHMARKS`). Fractions for accuracy benchmarks, Elo for GDPval.
    #[serde(default)]
    pub scores: BTreeMap<String, f64>,
}

/// How a benchmark score reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scale {
    /// An index score shown to one decimal (AA Intelligence Index, Epoch ECI).
    Index,
    /// A 0–1 fraction shown as a percentage.
    Share,
    /// An Elo-style rating.
    Elo,
}

pub struct Benchmark {
    /// Key in `ModelPrint::scores`; "index" is the Intelligence Index itself.
    pub id: &'static str,
    /// Who runs it.
    pub source: &'static str,
    pub short: &'static str,
    pub name: &'static str,
    pub measures: &'static str,
    pub scale: Scale,
}

/// Artificial Analysis benchmarks reported for every frontier model, then indices from
/// independent evaluators (matched by model name, so a model they have not rated shows a gap).
pub const BENCHMARKS: &[Benchmark] = &[
    Benchmark {
        id: "index",
        source: "Artificial Analysis",
        short: "Index",
        name: "Intelligence Index",
        measures: "composite of its evaluation suite",
        scale: Scale::Index,
    },
    Benchmark {
        id: "hle-diamond",
        source: "Artificial Analysis",
        short: "HLE",
        name: "Humanity's Last Exam (Diamond)",
        measures: "expert-level reasoning and knowledge",
        scale: Scale::Share,
    },
    Benchmark {
        id: "terminalbench-4-0",
        source: "Artificial Analysis",
        short: "Terminal",
        name: "Terminal-Bench 4.0",
        measures: "agentic coding and shell tasks",
        scale: Scale::Share,
    },
    Benchmark {
        id: "frontiercode-2-0",
        source: "Artificial Analysis",
        short: "Code",
        name: "FrontierCode 2.0",
        measures: "hard programming problems",
        scale: Scale::Share,
    },
    Benchmark {
        id: "critpt-v2-hard",
        source: "Artificial Analysis",
        short: "CritPt",
        name: "CritPt v2 (hard)",
        measures: "research-level physics",
        scale: Scale::Share,
    },
    Benchmark {
        id: "automationbench-aa",
        source: "Artificial Analysis",
        short: "Automate",
        name: "AutomationBench",
        measures: "multi-step workflow automation",
        scale: Scale::Share,
    },
    Benchmark {
        id: "artificial-analysis-long-context-reasoning",
        source: "Artificial Analysis",
        short: "LCR",
        name: "AA Long Context Reasoning",
        measures: "reasoning over long documents",
        scale: Scale::Share,
    },
    Benchmark {
        id: "omniscience-non-hallucination",
        source: "Artificial Analysis",
        short: "No-halluc",
        name: "Omniscience non-hallucination",
        measures: "when it does not know, how often it abstains instead of guessing",
        scale: Scale::Share,
    },
    Benchmark {
        id: "gdpval-aa-normalized",
        source: "Artificial Analysis",
        short: "GDPval",
        name: "GDPval-AA",
        measures: "economically valuable knowledge work, Elo",
        scale: Scale::Elo,
    },
    Benchmark {
        id: "epoch-eci",
        source: "Epoch AI",
        short: "ECI",
        name: "Epoch Capabilities Index",
        measures: "Epoch's statistical model putting many benchmarks on one scale; CI on hover",
        scale: Scale::Index,
    },
    Benchmark {
        id: "vals-index",
        source: "Vals.ai",
        short: "Vals",
        name: "Vals Index",
        measures: "Vals.ai composite of its industry task benchmarks",
        scale: Scale::Share,
    },
];

impl ModelPrint {
    pub fn score(&self, id: &str) -> Option<f64> {
        if id == "index" {
            self.intelligence
        } else {
            self.scores.get(id).copied()
        }
    }
}

impl Benchmark {
    pub fn format(&self, v: f64) -> String {
        match self.scale {
            Scale::Index => format!("{v:.1}"),
            Scale::Share => format!("{:.0}%", v * 100.0),
            Scale::Elo => format!("{v:.0}"),
        }
    }
}

/// Headline figures the web app shows beyond the original four desks.
/// Histories are dated so charts can label their axes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Terminal {
    /// Version of the derived liquidity calculation; zero denotes a legacy cache.
    pub net_liq_version: u32,
    pub ath: Option<f64>,
    pub ath_unix: Option<i64>,
    pub mayer: Option<f64>,
    pub mvrv_z: Option<f64>,
    pub nupl: Option<f64>,
    pub realized: Option<f64>,
    pub net_liq_b: Option<f64>,
    pub net_liq_30d_b: Option<f64>,
    pub net_liq_hist: Vec<Point>,
    pub funding_avg_7d: Option<f64>,
    pub funding_ann: Option<f64>,
    pub venues: Vec<VenuePrint>,
    pub premium_bps: Option<f64>,
    pub premium_avg_bps: Option<f64>,
    pub premium_hist: Vec<Point>,
    pub correlations: Vec<CorrPrint>,
    pub macro_strip: Vec<MacroPrint>,
    pub mei_state: String,
    pub mei_valence: Option<f64>,
    pub mei_arousal: Option<f64>,
    pub mei_intensity: Option<f64>,
    pub etf_latest_m: Option<f64>,
    pub etf_latest_unix: Option<i64>,
    pub etf_7d_m: Option<f64>,
    pub etf_30d_m: Option<f64>,
    pub etf_cum_m: Option<f64>,
    pub etf_flows: Vec<Point>,
    pub regime_score: Option<i32>,
    pub regime_alloc: Option<f64>,
    pub regime_inputs: Vec<ScorePrint>,
    pub regime_hist: Vec<Point>,
    pub strc_price: Option<f64>,
    pub strc_change: Option<f64>,
    pub strc_week_m: Option<f64>,
    pub strc_btc: Option<f64>,
    /// Net STRC raised across the parsed recent 8-K filings (not a calendar YTD).
    pub strc_recent_m: Option<f64>,
    pub strc_filings: u32,
    pub strc_left_m: Option<f64>,
    pub atm_cap_b: Option<f64>,
    pub strc_filed: Option<i64>,
    pub strc_hist: Vec<Point>,
    pub uranium: Option<f64>,
    pub uranium_12m: Option<f64>,
    pub uranium_lo: Option<f64>,
    pub uranium_hi: Option<f64>,
    pub uranium_as_of: String,
    pub uranium_hist: Vec<Point>,
    pub models: Vec<ModelPrint>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Snapshot {
    pub version: u32,
    pub fetched_unix: i64,
    pub history_unix: i64,
    pub weekly_unix: i64,
    pub dollar: DollarBook,
    pub btc: BtcBook,
    pub quotes: Vec<Quote>,
    pub notes: Vec<String>,
    #[serde(default)]
    pub terminal: Terminal,
    #[serde(default)]
    pub zec: crate::zcash::ZecBook,
    /// Cross-asset daily closes keyed by instrument (see `crate::markets`).
    #[serde(default)]
    pub histories: BTreeMap<String, Vec<Point>>,
    #[serde(default)]
    pub histories_unix: i64,
    /// Last successful response per feed, unix seconds.
    #[serde(default)]
    pub feeds: BTreeMap<String, i64>,
    /// Per-symbol price metadata. Older caches deliberately have unknown timing.
    #[serde(default)]
    pub price_stamps: BTreeMap<String, PriceStamp>,
}

impl Snapshot {
    pub fn price_stamp(&self, symbol: &str) -> Option<&PriceStamp> {
        self.price_stamps.get(symbol)
    }

    pub fn price_observed_unix(&self, symbol: &str) -> Option<i64> {
        self.price_stamp(symbol)?.observed_unix
    }

    pub fn price_acquired_unix(&self, symbol: &str) -> Option<i64> {
        Some(self.price_stamp(symbol)?.acquired_unix)
    }

    pub fn price_as_of(&self, symbol: &str, now: i64) -> String {
        let Some(stamp) = self.price_stamp(symbol) else {
            return "cached".into();
        };
        if matches!(symbol, "BTC" | "ETH" | "ZEC") {
            if stamp.live_spot(now) {
                return "live".into();
            }
            let age = fmt_age(stamp.observed_unix.unwrap_or(stamp.acquired_unix), now);
            return format!("{} old", age.trim_end_matches(" ago"));
        }
        // TradingView doesn't report a last-trade timestamp in the scanner
        // response. Show receipt age without inventing the observation date.
        let age = fmt_age(stamp.acquired_unix, now);
        format!("recv {}", age.trim_end_matches(" ago"))
    }
}

pub const SNAPSHOT_VERSION: u32 = 1;

#[derive(Clone, Debug)]
pub struct Metric {
    pub id: &'static str,
    pub group: Group,
    pub label: &'static str,
    pub value: String,
    pub status: Status,
    pub context: String,
    pub rule: &'static str,
    /// Change since the previous close or session, when a feed reports one.
    pub change: Option<Delta>,
    /// False for dated facts that no feed refreshes.
    pub live: bool,
}

/// A move in price terms (percent) or rate terms (percentage points, shown in bp).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Delta {
    Pct(f64),
    Pp(f64),
}

impl Delta {
    pub fn value(self) -> f64 {
        match self {
            Self::Pct(v) | Self::Pp(v) => v,
        }
    }

    pub fn text(self) -> String {
        match self {
            Self::Pct(v) => fmt_change_pct(v),
            Self::Pp(v) => fmt_change_bps(v),
        }
    }

    /// Treat moves under half a basis point or 0.005% as flat.
    pub fn is_flat(self) -> bool {
        match self {
            Self::Pct(v) => v.abs() < 0.005,
            Self::Pp(v) => v.abs() < 0.005,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Regime {
    RiskOn,
    RiskOff,
    Mixed,
}

impl Regime {
    pub fn label(self) -> &'static str {
        match self {
            Self::RiskOn => "Risk-on",
            Self::RiskOff => "Risk-off",
            Self::Mixed => "Mixed",
        }
    }
}

#[derive(Clone, Debug)]
pub struct PowerFit {
    /// Day zero of the log-time axis.
    pub genesis: i64,
    pub a: f64,
    pub b: f64,
    pub sigma: f64,
    pub n: usize,
}

pub fn fallback_snapshot() -> Snapshot {
    let treasury = "U.S. Treasury snapshot 2026-09-02";
    let liveish = "snapshot 2026-09-03";
    Snapshot {
        version: SNAPSHOT_VERSION,
        fetched_unix: 0,
        history_unix: 0,
        weekly_unix: 0,
        dollar: DollarBook {
            curve_history: Vec::new(),
            curve: vec![curve("2Y", 4.39), curve("10Y", 4.79), curve("30Y", 5.27)],
            curve_date: "2026-09-02".into(),
            ust2: Some(obs("2026-09-02", 4.39, treasury)),
            ust10: Some(obs("2026-09-02", 4.79, treasury)),
            ust30: Some(obs("2026-09-02", 5.27, treasury)),
            fed_lo: Some(obs("2026-09-02", 3.50, "FRED DFEDTARL snapshot")),
            fed_hi: Some(obs("2026-09-02", 3.75, "FRED DFEDTARU snapshot")),
            fed_hi_6m: None,
            dxy: Some(obs("2026-09-03", 99.37, "broad USD snapshot, not ICE DXY")),
            dxy_broad: Some(obs("2026-09-03", 99.37, "broad USD snapshot")),
            dxy_spark: Vec::new(),
            usdjpy: Some(obs("2026-09-03", 159.09, liveish)),
            jgb10: Some(obs("2026-09-03", 2.95, "snapshot")),
            vix: Some(obs("2026-09-02", 15.2, "CBOE snapshot")),
            spx: Some(obs("2026-09-02", 7666.6, "S&P 500 snapshot")),
            gold: Some(obs("2026-09-03", 4433.4, "gold snapshot")),
            brent: Some(obs("2026-09-03", 94.26, "Brent snapshot")),
            boj: None,
        },
        btc: BtcBook {
            price: Some(77_751.0),
            price_source: "CoinGecko snapshot 2026-09-03".into(),
            change_24h_pct: None,
            high_24h: None,
            low_24h: None,
            eth_price: None,
            eth_change_24h_pct: None,
            zec_price: None,
            zec_change_24h_pct: None,
            daily: Vec::new(),
            weekly: Vec::new(),
            spark: Vec::new(),
            fng: None,
            fng_label: String::new(),
            fng_hist: Vec::new(),
            funding: None,
            funding_hist: Vec::new(),
            eth_daily: Vec::new(),
        },
        quotes: Vec::new(),
        notes: vec!["Showing the 3 Sep 2026 snapshot until the first refresh finishes.".into()],
        terminal: Terminal::default(),
        zec: Default::default(),
        histories: BTreeMap::new(),
        histories_unix: 0,
        feeds: BTreeMap::new(),
        price_stamps: BTreeMap::new(),
    }
}

fn obs(date: &str, value: f64, source: &str) -> Obs {
    Obs {
        date: date.into(),
        value,
        source: source.into(),
    }
}

fn curve(label: &str, yield_pct: f64) -> CurvePoint {
    CurvePoint {
        label: label.into(),
        yield_pct,
    }
}

pub fn dollar_metrics(snap: &Snapshot) -> Vec<Metric> {
    let d = &snap.dollar;
    // Daily moves: official yields against the previous Treasury close, quotes from the feed.
    let prev_curve = d
        .curve_history
        .iter()
        .rev()
        .find(|day| day.date < d.curve_date);
    let yield_change = |tenor: &str, now: f64| {
        prev_curve
            .and_then(|day| day.points.iter().find(|p| p.label == tenor))
            .map(|p| Delta::Pp(now - p.yield_pct))
    };
    let session = |symbol: &str| quote(&snap.quotes, symbol).map(|q| Delta::Pct(q.change_pct));
    let mut out = Vec::new();
    if let Some(v) = &d.ust30 {
        out.push(Metric {
            change: yield_change("30Y", v.value),
            ..metric(
                "ust30",
                Group::Rates,
                "30Y Treasury yield",
                format!("{:.2}%", v.value),
                yield_status_30(v.value),
                as_of(&v.date, &v.source),
                "Calm under 4%, neutral from 4%, elevated from 4.5%, stressed from 5%.",
            )
        });
    }
    if let Some(v) = &d.ust10 {
        out.push(Metric {
            change: yield_change("10Y", v.value),
            ..metric(
                "ust10",
                Group::Rates,
                "10Y Treasury yield",
                format!("{:.2}%", v.value),
                yield_status_10(v.value),
                as_of(&v.date, &v.source),
                "Calm under 3.75%, neutral from 3.75%, elevated from 4.25%, stressed from 4.75%.",
            )
        });
    }
    if let Some(v) = &d.ust2 {
        out.push(Metric {
            change: yield_change("2Y", v.value),
            ..metric(
                "ust2",
                Group::Rates,
                "2Y Treasury yield",
                format!("{:.2}%", v.value),
                yield_status_2(v.value),
                as_of(&v.date, &v.source),
                "Calm under 3.5%, neutral from 3.5%, elevated from 4.25%, stressed from 5%.",
            )
        });
    }
    if let (Some(lo), Some(hi)) = (&d.fed_lo, &d.fed_hi) {
        let context = match &d.fed_hi_6m {
            Some(prev) => format!(
                "{} · {} vs {}",
                as_of(&hi.date, &hi.source),
                fmt_change_bps(hi.value - prev.value),
                pretty_date(&prev.date)
            ),
            None => as_of(&hi.date, &hi.source),
        };
        out.push(metric(
            "fed",
            Group::Rates,
            "Fed funds target",
            format!("{:.2}–{:.2}%", lo.value, hi.value),
            Status::Neutral,
            context,
            "The target range is shown as context. Status stays neutral; direction is in the 6-month change when FRED returns it.",
        ));
    }
    if let (Some(y2), Some(y10)) = (&d.ust2, &d.ust10) {
        let bps = ((y10.value - y2.value) * 100.0).round() as i64;
        let status = curve_status(bps);
        let context = if bps >= 50 {
            "bear-steepening risk".into()
        } else {
            "flat or mid range".into()
        };
        let change = match (yield_change("10Y", y10.value), yield_change("2Y", y2.value)) {
            (Some(a), Some(b)) => Some(Delta::Pp(a.value() - b.value())),
            _ => None,
        };
        out.push(Metric {
            change,
            ..metric(
                "curve",
                Group::Rates,
                "2s/10s curve",
                with_minus(format!("{bps:+} bp")),
                status,
                context,
                "Stressed when inverted by more than 25 bp. Elevated when steeper than 100 bp. Otherwise neutral.",
            )
        });
    }
    if let Some(v) = &d.dxy {
        out.push(Metric {
            change: session("DXY"),
            ..metric(
                "dxy",
                Group::Dollar,
                "US dollar index",
                format!("{:.2}", v.value),
                Status::Neutral,
                as_of(&v.date, &v.source),
                "Level is shown without a status band. The home regime treats ICE DXY under 100 as fading and over 106 as bid.",
            )
        });
    }
    if let Some(v) = &d.usdjpy {
        let status = usdjpy_status(v.value);
        let context = if v.value >= 155.0 {
            format!("near the intervention zone (~160) · {}", v.source)
        } else {
            as_of(&v.date, &v.source)
        };
        out.push(Metric {
            change: session("USDJPY"),
            ..metric(
                "usdjpy",
                Group::Dollar,
                "USD/JPY",
                format!("{:.2}", v.value),
                status,
                context,
                "Calm under 145, neutral from 145, elevated from 152, stressed from 158.",
            )
        });
    }
    if let Some(v) = &d.jgb10 {
        out.push(Metric {
            change: quote(&snap.quotes, "JP10Y")
                .and_then(|q| q.change_abs)
                .map(Delta::Pp),
            ..metric(
                "jgb10",
                Group::Dollar,
                "Japan 10Y",
                format!("{:.2}%", v.value),
                jgb_status(v.value),
                as_of(&v.date, &v.source),
                "Neutral under 2%, elevated from 2%, stressed from 3.5%.",
            )
        });
    }
    let boj_rule = "Neutral under 0.5%, elevated from 0.5%, stressed from 1.0%: higher Japanese rates squeeze the yen carry trade.";
    match &d.boj {
        Some(v) => out.push(Metric {
            change: quote(&snap.quotes, "JPINTR")
                .and_then(|q| q.change_abs)
                .map(Delta::Pp),
            ..metric(
                "boj",
                Group::Dollar,
                "BOJ policy rate",
                format!("{:.2}%", v.value),
                boj_status(v.value),
                match quote(&snap.quotes, "JPINTR").and_then(|q| q.change_abs) {
                    Some(step) if step.abs() > 1e-9 => {
                        format!("last move {} · {}", fmt_change_bps(step), v.source)
                    }
                    _ => as_of(&v.date, &v.source),
                },
                boj_rule,
            )
        }),
        // Dated fallback for when the economics feed is missing.
        None => out.push(Metric {
            live: false,
            ..metric(
                "boj",
                Group::Dollar,
                "BOJ policy rate",
                "1.25%".into(),
                boj_status(1.25),
                "snapshot · raised to 1.25% on 18 Sep 2026 · feed unavailable".into(),
                boj_rule,
            )
        }),
    }
    if let Some(v) = &d.vix {
        let status = vix_status(v.value);
        let context = if v.value < 18.0 {
            format!("calm — watch divergence versus bonds · {}", v.source)
        } else {
            as_of(&v.date, &v.source)
        };
        out.push(Metric {
            change: session("VIX"),
            ..metric(
                "vix",
                Group::Risk,
                "VIX",
                format!("{:.2}", v.value),
                status,
                context,
                "Calm under 15, neutral under 22, elevated under 30, stressed from 30.",
            )
        });
    }
    if let Some(v) = &d.spx {
        out.push(Metric {
            change: session("SPX"),
            ..metric(
                "spx",
                Group::Risk,
                "S&P 500",
                fmt_grouped(v.value, 2),
                Status::Neutral,
                as_of(&v.date, &v.source),
                "Shown as context. Status stays neutral because the level alone is not a stress rule.",
            )
        });
    }
    if let Some(v) = &d.gold {
        out.push(Metric {
            change: session("GOLD"),
            ..metric(
                "gold",
                Group::Risk,
                "Gold",
                format!("${}", fmt_grouped(v.value, 0)),
                gold_status(v.value),
                if v.value >= 4000.0 {
                    format!("haven or debasement bid · {}", v.source)
                } else {
                    as_of(&v.date, &v.source)
                },
                "Calm under $2,500, neutral from $2,500, elevated from $4,000.",
            )
        });
    }
    if let Some(price) = snap.btc.price {
        out.push(Metric {
            change: snap.btc.change_24h_pct.map(Delta::Pct),
            ..metric(
                "btc",
                Group::Risk,
                "Bitcoin",
                format!("${}", fmt_grouped(price, 0)),
                Status::Neutral,
                format!("{} · 24h", snap.btc.price_source),
                "Shown as context on this desk. Cycle and funding live on the Bitcoin desk.",
            )
        });
    }
    if let Some(v) = &d.brent {
        out.push(Metric {
            change: session("BRENT"),
            ..metric(
                "brent",
                Group::Risk,
                "Brent",
                format!("${:.2}", v.value),
                brent_status(v.value),
                as_of(&v.date, &v.source),
                "Calm under $60, neutral from $60, elevated from $95.",
            )
        });
    }
    out
}

fn metric(
    id: &'static str,
    group: Group,
    label: &'static str,
    value: String,
    status: Status,
    context: String,
    rule: &'static str,
) -> Metric {
    Metric {
        id,
        group,
        label,
        value,
        status,
        context,
        rule,
        change: None,
        live: true,
    }
}

pub fn yield_status_30(v: f64) -> Status {
    if v >= 5.0 {
        Status::Stressed
    } else if v >= 4.5 {
        Status::Elevated
    } else if v >= 4.0 {
        Status::Neutral
    } else {
        Status::Calm
    }
}

pub fn yield_status_10(v: f64) -> Status {
    if v >= 4.75 {
        Status::Stressed
    } else if v >= 4.25 {
        Status::Elevated
    } else if v >= 3.75 {
        Status::Neutral
    } else {
        Status::Calm
    }
}

pub fn yield_status_2(v: f64) -> Status {
    if v >= 5.0 {
        Status::Stressed
    } else if v >= 4.25 {
        Status::Elevated
    } else if v >= 3.5 {
        Status::Neutral
    } else {
        Status::Calm
    }
}

pub fn curve_status(bps: i64) -> Status {
    if bps < -25 {
        Status::Stressed
    } else if bps >= 100 {
        Status::Elevated
    } else {
        Status::Neutral
    }
}

pub fn usdjpy_status(v: f64) -> Status {
    if v >= 158.0 {
        Status::Stressed
    } else if v >= 152.0 {
        Status::Elevated
    } else if v >= 145.0 {
        Status::Neutral
    } else {
        Status::Calm
    }
}

pub fn jgb_status(v: f64) -> Status {
    if v >= 3.5 {
        Status::Stressed
    } else if v >= 2.0 {
        Status::Elevated
    } else {
        Status::Neutral
    }
}

pub fn boj_status(v: f64) -> Status {
    if v >= 1.0 {
        Status::Stressed
    } else if v >= 0.5 {
        Status::Elevated
    } else {
        Status::Neutral
    }
}

pub fn vix_status(v: f64) -> Status {
    if v < 15.0 {
        Status::Calm
    } else if v < 22.0 {
        Status::Neutral
    } else if v < 30.0 {
        Status::Elevated
    } else {
        Status::Stressed
    }
}

pub fn gold_status(v: f64) -> Status {
    if v >= 4000.0 {
        Status::Elevated
    } else if v >= 2500.0 {
        Status::Neutral
    } else {
        Status::Calm
    }
}

pub fn brent_status(v: f64) -> Status {
    if v >= 95.0 {
        Status::Elevated
    } else {
        Status::Neutral
    }
}

pub fn fng_status(v: i32) -> Status {
    if !(25..75).contains(&v) {
        Status::Stressed
    } else if !(45..56).contains(&v) {
        Status::Elevated
    } else {
        Status::Neutral
    }
}

fn as_of(date: &str, source: &str) -> String {
    if date.is_empty() {
        source.to_string()
    } else {
        format!("as of {} · {source}", pretty_date(date))
    }
}

pub fn pretty_date(date: &str) -> String {
    let day = date.get(0..10).unwrap_or(date);
    let Ok(parsed) = chrono::NaiveDate::parse_from_str(day, "%Y-%m-%d") else {
        return day.to_string();
    };
    let d = parsed.format("%d").to_string();
    let d = d.trim_start_matches('0');
    format!("{d} {}", parsed.format("%b %Y"))
}

pub fn rule_reading(metrics: &[Metric]) -> String {
    if metrics.is_empty() {
        return "Waiting for rates and dollar prints.".into();
    }
    let stressed = metrics
        .iter()
        .filter(|m| m.status == Status::Stressed)
        .count();
    let elevated = metrics
        .iter()
        .filter(|m| m.status == Status::Elevated)
        .count();
    let mut parts = vec![format!(
        "{stressed} of {} rules read stressed and {elevated} elevated.",
        metrics.len()
    )];
    for id in ["ust30", "usdjpy", "vix", "curve"] {
        if let Some(m) = metrics.iter().find(|m| m.id == id) {
            parts.push(format!(
                "{} is {} at {}.",
                m.label,
                m.status.label().to_lowercase(),
                m.value
            ));
        }
    }
    let vix_calm = metrics
        .iter()
        .any(|m| m.id == "vix" && m.status == Status::Calm);
    let rates_tight = metrics
        .iter()
        .any(|m| matches!(m.id, "ust30" | "ust10" | "usdjpy") && m.status == Status::Stressed);
    if vix_calm && rates_tight {
        parts.push("Equity volatility is calm while funding and the long end are stressed.".into());
    }
    parts.join(" ")
}

pub fn regime(
    btc_change_pct: Option<f64>,
    vix: Option<f64>,
    fng: Option<i32>,
    dxy: Option<f64>,
) -> Regime {
    let mut bullish = 0;
    let mut bearish = 0;
    if let Some(chg) = btc_change_pct {
        if chg > 0.5 {
            bullish += 1;
        } else if chg < -0.5 {
            bearish += 1;
        }
    }
    if let Some(v) = vix {
        if v < 16.0 {
            bullish += 1;
        } else if v > 25.0 {
            bearish += 1;
        }
    }
    if let Some(v) = fng {
        if v >= 55 {
            bullish += 1;
        } else if v <= 30 {
            bearish += 1;
        }
    }
    if let Some(v) = dxy {
        if v < 100.0 {
            bullish += 1;
        } else if v > 106.0 {
            bearish += 1;
        }
    }
    if bullish > bearish + 1 {
        Regime::RiskOn
    } else if bearish > bullish + 1 {
        Regime::RiskOff
    } else {
        Regime::Mixed
    }
}

pub fn headline(
    btc_change_pct: Option<f64>,
    vix: Option<f64>,
    fng: Option<i32>,
    dxy: Option<f64>,
) -> String {
    let tone = regime(btc_change_pct, vix, fng, dxy);
    let mut lead = match tone {
        Regime::RiskOn => "Markets are risk-on".to_string(),
        Regime::RiskOff => "Markets are risk-off".to_string(),
        Regime::Mixed => "Mixed signals across markets".to_string(),
    };
    let dollar = dxy.map(|v| {
        if v > 105.0 {
            "dollar bid"
        } else if v < 100.0 {
            "dollar fading"
        } else {
            "dollar steady"
        }
    });
    let vol = vix.and_then(|v| {
        if v > 30.0 {
            Some("vol spiking")
        } else if v > 20.0 {
            Some("vol rising")
        } else if v < 14.0 {
            Some("vol crushed")
        } else {
            None
        }
    });
    let crypto = btc_change_pct.map(|v| {
        if v.abs() < 0.5 {
            "crypto flat"
        } else if v > 3.0 {
            "crypto rallying"
        } else if v > 0.0 {
            "crypto bid"
        } else if v < -3.0 {
            "crypto selling off"
        } else {
            "crypto soft"
        }
    });
    let details: Vec<&str> = [dollar, vol, crypto].into_iter().flatten().collect();
    if !details.is_empty() {
        lead.push_str(" — ");
        lead.push_str(&details.join(", "));
    }
    lead.push('.');
    lead
}

/// Bitcoin power law: days since the genesis block, prices from 17 Jul 2010.
pub fn fit_power_law(points: &[Point]) -> Option<PowerFit> {
    fit_log_log(points, GENESIS_UNIX, POWER_LAW_CUTOFF)
}

/// OLS line of log10(price) on log10(days since `genesis`), using points from `cutoff`.
pub fn fit_log_log(points: &[Point], genesis: i64, cutoff: i64) -> Option<PowerFit> {
    let mut xs = Vec::new();
    let mut ys = Vec::new();
    for point in points {
        if point.t < cutoff || point.v <= 0.0 || !point.v.is_finite() {
            continue;
        }
        let days = (point.t - genesis) as f64 / SECONDS_PER_DAY as f64;
        if days <= 0.0 {
            continue;
        }
        xs.push(days.log10());
        ys.push(point.v.log10());
    }
    let n = xs.len();
    if n < 100 {
        return None;
    }
    let mut sx = 0.0;
    let mut sy = 0.0;
    let mut sxx = 0.0;
    let mut sxy = 0.0;
    for i in 0..n {
        sx += xs[i];
        sy += ys[i];
        sxx += xs[i] * xs[i];
        sxy += xs[i] * ys[i];
    }
    let mean_x = sx / n as f64;
    let mean_y = sy / n as f64;
    let denom = sxx - n as f64 * mean_x * mean_x;
    if denom.abs() < 1e-12 {
        return None;
    }
    let b = (sxy - n as f64 * mean_x * mean_y) / denom;
    let a = mean_y - b * mean_x;
    let mut acc = 0.0;
    for i in 0..n {
        let residual = ys[i] - (a + b * xs[i]);
        acc += residual * residual;
    }
    let sigma = (acc / n as f64).sqrt();
    if !sigma.is_finite() || sigma <= 0.0 {
        return None;
    }
    Some(PowerFit {
        genesis,
        a,
        b,
        sigma,
        n,
    })
}

pub fn power_value(fit: &PowerFit, unix: i64, sigmas: f64) -> Option<f64> {
    let days = (unix - fit.genesis) as f64 / SECONDS_PER_DAY as f64;
    if days <= 0.0 {
        return None;
    }
    Some(10f64.powf(fit.a + fit.b * days.log10() + sigmas * fit.sigma))
}

pub fn power_z(fit: &PowerFit, unix: i64, price: f64) -> Option<f64> {
    if price <= 0.0 {
        return None;
    }
    let fair = power_value(fit, unix, 0.0)?;
    if fair <= 0.0 {
        return None;
    }
    Some((price.log10() - fair.log10()) / fit.sigma)
}

pub fn trailing_mean(points: &[Point], n: usize) -> Option<f64> {
    if n == 0 || points.len() < n {
        return None;
    }
    let slice = &points[points.len() - n..];
    Some(slice.iter().map(|p| p.v).sum::<f64>() / n as f64)
}

pub fn ath(points: &[Point]) -> Option<Point> {
    points
        .iter()
        .filter(|p| p.v.is_finite() && p.v > 0.0)
        .max_by(|a, b| a.v.total_cmp(&b.v))
        .cloned()
}

pub fn quote<'a>(quotes: &'a [Quote], symbol: &str) -> Option<&'a Quote> {
    quotes
        .iter()
        .find(|q| q.symbol.eq_ignore_ascii_case(symbol))
}

pub fn weighted_change(quotes: &[Quote], symbols: &[&str]) -> Option<f64> {
    let mut weighted = 0.0;
    let mut cap = 0.0;
    let mut plain = Vec::new();
    for symbol in symbols {
        let Some(q) = quote(quotes, symbol) else {
            continue;
        };
        plain.push(q.change_pct);
        if let Some(m) = q.market_cap {
            weighted += q.change_pct * m;
            cap += m;
        }
    }
    if cap > 0.0 {
        Some(weighted / cap)
    } else if plain.is_empty() {
        None
    } else {
        Some(plain.iter().sum::<f64>() / plain.len() as f64)
    }
}

pub fn total_cap(quotes: &[Quote], symbols: &[&str]) -> Option<f64> {
    let mut cap = 0.0;
    let mut any = false;
    for symbol in symbols {
        if let Some(m) = quote(quotes, symbol).and_then(|q| q.market_cap) {
            cap += m;
            any = true;
        }
    }
    any.then_some(cap)
}

pub fn fmt_grouped(value: f64, digits: usize) -> String {
    let neg = value.is_sign_negative();
    let text = format!("{:.*}", digits, value.abs());
    let (int_part, frac) = text
        .split_once('.')
        .map(|(a, b)| (a, Some(b)))
        .unwrap_or((text.as_str(), None));
    let mut grouped = String::new();
    for (i, ch) in int_part.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(ch);
    }
    let int_part: String = grouped.chars().rev().collect();
    let mut out = if neg {
        format!("-{int_part}")
    } else {
        int_part
    };
    if let Some(frac) = frac {
        out.push('.');
        out.push_str(frac);
    }
    out
}

pub fn fmt_cap(value: f64) -> String {
    let abs = value.abs();
    if abs >= 1e12 {
        format!("${:.2}T", value / 1e12)
    } else if abs >= 1e9 {
        format!("${:.1}B", value / 1e9)
    } else if abs >= 1e6 {
        format!("${:.0}M", value / 1e6)
    } else {
        format!("${}", fmt_grouped(value, 0))
    }
}

pub fn fmt_signed_pct(value: f64) -> String {
    format!("{value:+.1}%")
}

/// Signed percent with a typographic minus, two decimals under 10%.
pub fn fmt_change_pct(value: f64) -> String {
    let digits = if value.abs() < 10.0 { 2 } else { 1 };
    with_minus(format!("{value:+.digits$}%"))
}

/// Signed basis points from a change in percentage points.
pub fn fmt_change_bps(pp: f64) -> String {
    let bps = pp * 100.0;
    let digits = if bps.abs() < 10.0 { 1 } else { 0 };
    with_minus(format!("{bps:+.digits$} bp"))
}

pub fn with_minus(text: String) -> String {
    match text.strip_prefix('-') {
        Some(rest) => format!("−{rest}"),
        None => text,
    }
}

pub fn fmt_usd(value: f64) -> String {
    if value.abs() >= 1000.0 {
        with_minus(format!("${}", fmt_grouped(value, 0)).replace("$-", "-$"))
    } else {
        with_minus(format!("${}", fmt_grouped(value, 2)).replace("$-", "-$"))
    }
}

/// Price with precision that suits its magnitude: 81,723 · 2,435.04 · 1.1239.
pub fn fmt_price(value: f64) -> String {
    let abs = value.abs();
    let digits = if abs >= 10_000.0 {
        0
    } else if abs >= 10.0 {
        2
    } else if abs >= 1.0 {
        4
    } else {
        5
    };
    with_minus(fmt_grouped(value, digits))
}

/// Compact magnitude without a currency sign: 5.87T, 209.2B, 905M, 12.4k.
pub fn fmt_compact(value: f64) -> String {
    let abs = value.abs();
    let text = if abs >= 1e12 {
        format!("{:.2}T", value / 1e12)
    } else if abs >= 1e9 {
        format!("{:.1}B", value / 1e9)
    } else if abs >= 1e6 {
        format!("{:.1}M", value / 1e6)
    } else if abs >= 1e4 {
        format!("{:.1}k", value / 1e3)
    } else {
        fmt_grouped(value, 0)
    };
    with_minus(text)
}

/// "3m ago", "2h ago", "4d ago" for a unix timestamp.
pub fn fmt_age(then: i64, now: i64) -> String {
    let secs = (now - then).max(0);
    if secs < 90 {
        format!("{secs}s ago")
    } else if secs < 90 * 60 {
        format!("{}m ago", (secs + 30) / 60)
    } else if secs < 36 * 3600 {
        format!("{}h ago", (secs + 1800) / 3600)
    } else {
        format!("{}d ago", (secs + 43_200) / 86_400)
    }
}

/// Close of the last point at or before `t`.
pub fn value_at(points: &[Point], t: i64) -> Option<f64> {
    let index = points.partition_point(|p| p.t <= t);
    index.checked_sub(1).map(|i| points[i].v)
}

/// Percent change from the close `days` before the latest point.
pub fn change_since(points: &[Point], days: i64) -> Option<f64> {
    let last = points.last()?;
    let base = value_at(points, last.t - days * SECONDS_PER_DAY)?;
    (base > 0.0).then(|| (last.v / base - 1.0) * 100.0)
}

/// Difference (latest minus the value `days` earlier), for rates and spreads.
pub fn delta_since(points: &[Point], days: i64) -> Option<f64> {
    let last = points.last()?;
    let base = value_at(points, last.t - days * SECONDS_PER_DAY)?;
    Some(last.v - base)
}

/// Last `n` closes, newest last.
pub fn tail_values(points: &[Point], n: usize) -> Vec<f64> {
    points[points.len().saturating_sub(n)..]
        .iter()
        .map(|p| p.v)
        .collect()
}

pub fn now_unix() -> i64 {
    chrono::Utc::now().timestamp()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn yield_bands_match_the_web_dashboard() {
        assert_eq!(yield_status_30(5.27), Status::Stressed);
        assert_eq!(yield_status_10(4.79), Status::Stressed);
        assert_eq!(yield_status_2(4.39), Status::Elevated);
        assert_eq!(curve_status(40), Status::Neutral);
        assert_eq!(curve_status(-40), Status::Stressed);
        assert_eq!(usdjpy_status(159.0), Status::Stressed);
        assert_eq!(vix_status(14.8), Status::Calm);
        assert_eq!(gold_status(4286.0), Status::Elevated);
        assert_eq!(brent_status(104.0), Status::Elevated);
    }

    #[test]
    fn headline_uses_the_same_regime_counts() {
        let text = headline(Some(1.1), Some(14.8), Some(70), Some(101.0));
        assert!(text.starts_with("Markets are risk-on"));
        assert!(text.contains("dollar steady"));
        assert!(text.contains("crypto bid"));
    }

    #[test]
    fn power_law_recovers_a_known_line() {
        let mut points = Vec::new();
        let a = -1.5;
        let b = 5.4;
        for i in 0..400 {
            let t = POWER_LAW_CUTOFF + i * SECONDS_PER_DAY * 10;
            let days = (t - GENESIS_UNIX) as f64 / SECONDS_PER_DAY as f64;
            let v = 10f64.powf(a + b * days.log10());
            points.push(Point { t, v });
        }
        let fit = fit_power_law(&points).unwrap();
        assert!((fit.a - a).abs() < 1e-6);
        assert!((fit.b - b).abs() < 1e-6);
        let fair = power_value(&fit, points[200].t, 0.0).unwrap();
        assert!((fair - points[200].v).abs() / points[200].v < 1e-6);
        points[10].v *= 1.25;
        let noisy = fit_power_law(&points).unwrap();
        assert!(noisy.sigma > 0.0);
        let z = power_z(&noisy, points[200].t, points[200].v).unwrap();
        assert!(z.abs() < 0.05, "z={z}");
    }

    #[test]
    fn boj_rate_is_live_when_the_economics_feed_answers() {
        let mut snap = fallback_snapshot();
        let fallback = dollar_metrics(&snap)
            .into_iter()
            .find(|m| m.id == "boj")
            .unwrap();
        assert!(!fallback.live);
        assert_eq!(fallback.value, "1.25%");
        snap.dollar.boj = Some(obs("2026-10-17", 1.25, "TradingView economics"));
        snap.quotes.push(Quote {
            symbol: "JPINTR".into(),
            price: 1.25,
            change_pct: 25.0,
            market_cap: None,
            change_abs: Some(0.25),
        });
        let live = dollar_metrics(&snap)
            .into_iter()
            .find(|m| m.id == "boj")
            .unwrap();
        assert!(live.live);
        assert_eq!(live.status, Status::Stressed);
        assert_eq!(live.change, Some(Delta::Pp(0.25)));
        assert!(live.context.starts_with("last move +25 bp"));
        assert_eq!(boj_status(0.75), Status::Elevated);
    }

    #[test]
    fn grouped_numbers_keep_sign_and_commas() {
        assert_eq!(fmt_grouped(84914.0, 0), "84,914");
        assert_eq!(fmt_grouped(-1234.5, 1), "-1,234.5");
        assert_eq!(fmt_cap(5.424e12), "$5.42T");
    }
}
