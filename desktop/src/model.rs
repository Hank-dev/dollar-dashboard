use serde::{Deserialize, Serialize};

pub const GENESIS_UNIX: i64 = 1_230_940_800;
pub const POWER_LAW_CUTOFF: i64 = 1_279_324_800;
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
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DollarBook {
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
    pub daily: Vec<Point>,
    pub weekly: Vec<Point>,
    pub spark: Vec<f64>,
    pub fng: Option<i32>,
    pub fng_label: String,
    pub fng_hist: Vec<f64>,
    pub funding: Option<f64>,
    pub funding_hist: Vec<f64>,
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
        },
        btc: BtcBook {
            price: Some(77_751.0),
            price_source: "CoinGecko snapshot 2026-09-03".into(),
            change_24h_pct: None,
            high_24h: None,
            low_24h: None,
            eth_price: None,
            eth_change_24h_pct: None,
            daily: Vec::new(),
            weekly: Vec::new(),
            spark: Vec::new(),
            fng: None,
            fng_label: String::new(),
            fng_hist: Vec::new(),
            funding: None,
            funding_hist: Vec::new(),
        },
        quotes: Vec::new(),
        notes: vec!["Showing the 3 Sep 2026 snapshot until the first refresh finishes.".into()],
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
    let mut out = Vec::new();
    if let Some(v) = &d.ust30 {
        out.push(metric(
            "ust30",
            Group::Rates,
            "30Y Treasury yield",
            format!("{:.2}%", v.value),
            yield_status_30(v.value),
            as_of(&v.date, &v.source),
            "Calm under 4%, neutral from 4%, elevated from 4.5%, stressed from 5%.",
        ));
    }
    if let Some(v) = &d.ust10 {
        out.push(metric(
            "ust10",
            Group::Rates,
            "10Y Treasury yield",
            format!("{:.2}%", v.value),
            yield_status_10(v.value),
            as_of(&v.date, &v.source),
            "Calm under 3.75%, neutral from 3.75%, elevated from 4.25%, stressed from 4.75%.",
        ));
    }
    if let Some(v) = &d.ust2 {
        out.push(metric(
            "ust2",
            Group::Rates,
            "2Y Treasury yield",
            format!("{:.2}%", v.value),
            yield_status_2(v.value),
            as_of(&v.date, &v.source),
            "Calm under 3.5%, neutral from 3.5%, elevated from 4.25%, stressed from 5%.",
        ));
    }
    if let (Some(lo), Some(hi)) = (&d.fed_lo, &d.fed_hi) {
        let context = match &d.fed_hi_6m {
            Some(prev) => {
                let diff = hi.value - prev.value;
                format!(
                    "{} · {:+.2} pp vs {}",
                    as_of(&hi.date, &hi.source),
                    diff,
                    prev.date
                )
            }
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
        out.push(metric(
            "curve",
            Group::Rates,
            "2s/10s curve",
            format!("{bps:+} bps"),
            status,
            context,
            "Stressed when inverted by more than 25 bps. Elevated when steeper than 100 bps. Otherwise neutral.",
        ));
    }
    if let Some(v) = &d.dxy {
        out.push(metric(
            "dxy",
            Group::Dollar,
            "US dollar index",
            format!("{:.2}", v.value),
            Status::Neutral,
            as_of(&v.date, &v.source),
            "Level is shown without a status band. The home regime treats ICE DXY under 100 as fading and over 106 as bid.",
        ));
    }
    if let Some(v) = &d.usdjpy {
        let status = usdjpy_status(v.value);
        let context = if v.value >= 155.0 {
            format!("near the intervention zone (~160) · {}", v.source)
        } else {
            as_of(&v.date, &v.source)
        };
        out.push(metric(
            "usdjpy",
            Group::Dollar,
            "USD/JPY",
            format!("{:.1}", v.value),
            status,
            context,
            "Calm under 145, neutral from 145, elevated from 152, stressed from 158.",
        ));
    }
    if let Some(v) = &d.jgb10 {
        out.push(metric(
            "jgb10",
            Group::Dollar,
            "Japan 10Y",
            format!("{:.2}%", v.value),
            jgb_status(v.value),
            as_of(&v.date, &v.source),
            "Neutral under 2%, elevated from 2%, stressed from 3.5%.",
        ));
    }
    out.push(metric(
        "boj",
        Group::Dollar,
        "BOJ policy rate",
        "1.0%".into(),
        Status::Stressed,
        "snapshot · held 31 Jul 2026 · not a live feed".into(),
        "The July 2026 hold at 1.0% is a dated fact. Status stays at the snapshot judgment until a live BOJ series is wired in.",
    ));
    if let Some(v) = &d.vix {
        let status = vix_status(v.value);
        let context = if v.value < 18.0 {
            format!("calm — watch divergence versus bonds · {}", v.source)
        } else {
            as_of(&v.date, &v.source)
        };
        out.push(metric(
            "vix",
            Group::Risk,
            "VIX",
            format!("{:.1}", v.value),
            status,
            context,
            "Calm under 15, neutral under 22, elevated under 30, stressed from 30.",
        ));
    }
    if let Some(v) = &d.spx {
        out.push(metric(
            "spx",
            Group::Risk,
            "S&P 500",
            fmt_grouped(v.value, 0),
            Status::Neutral,
            as_of(&v.date, &v.source),
            "Shown as context. Status stays neutral because the level alone is not a stress rule.",
        ));
    }
    if let Some(v) = &d.gold {
        out.push(metric(
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
        ));
    }
    if let Some(price) = snap.btc.price {
        out.push(metric(
            "btc",
            Group::Risk,
            "Bitcoin",
            format!("${}", fmt_grouped(price, 0)),
            Status::Neutral,
            snap.btc.price_source.clone(),
            "Shown as context on this desk. Cycle and funding live on the Bitcoin desk.",
        ));
    }
    if let Some(v) = &d.brent {
        out.push(metric(
            "brent",
            Group::Risk,
            "Brent",
            format!("${:.1}", v.value),
            brent_status(v.value),
            as_of(&v.date, &v.source),
            "Calm under $60, neutral from $60, elevated from $95.",
        ));
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
        "Rule reading: {stressed} stressed, {elevated} elevated."
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

pub fn fit_power_law(points: &[Point]) -> Option<PowerFit> {
    let mut xs = Vec::new();
    let mut ys = Vec::new();
    for point in points {
        if point.t < POWER_LAW_CUTOFF || point.v <= 0.0 || !point.v.is_finite() {
            continue;
        }
        let days = (point.t - GENESIS_UNIX) as f64 / SECONDS_PER_DAY as f64;
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
    Some(PowerFit { a, b, sigma, n })
}

pub fn power_value(fit: &PowerFit, unix: i64, sigmas: f64) -> Option<f64> {
    let days = (unix - GENESIS_UNIX) as f64 / SECONDS_PER_DAY as f64;
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

pub fn fmt_usd(value: f64) -> String {
    if value >= 1000.0 {
        format!("${}", fmt_grouped(value, 0))
    } else {
        format!("${}", fmt_grouped(value, 2))
    }
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
    fn grouped_numbers_keep_sign_and_commas() {
        assert_eq!(fmt_grouped(84914.0, 0), "84,914");
        assert_eq!(fmt_grouped(-1234.5, 1), "-1,234.5");
        assert_eq!(fmt_cap(5.424e12), "$5.42T");
    }
}
