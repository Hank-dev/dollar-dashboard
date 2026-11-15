use crate::model::{
    now_unix, CorrPrint, MacroPrint, ModelPrint, Point, ScorePrint, Terminal, VenuePrint,
    SECONDS_PER_DAY,
};
use crate::parse;
use chrono::Datelike;
use regex::Regex;
use reqwest::blocking::Client;
use serde_json::Value;
use std::collections::HashMap;
use std::thread;
use std::time::Duration;

/// Bump when a cached derived liquidity value would otherwise retain old units.
pub const NET_LIQ_VERSION: u32 = 1;

const MODELS: &[(&str, &str, Option<&str>)] = &[
    (
        "Claude Opus 5.5",
        "/models/anthropic/claude-opus-5-5",
        Some("anthropic/claude-opus-5.5"),
    ),
    (
        "Claude Sonnet 5.5",
        "/models/anthropic/claude-sonnet-5-5",
        Some("anthropic/claude-sonnet-5.5"),
    ),
    (
        "Claude Fable 5.1",
        "/models/anthropic/claude-fable-5-1",
        Some("anthropic/claude-fable-5.1"),
    ),
    (
        "GPT-6 Astra",
        "/models/openai/gpt-6-astra",
        Some("openai/gpt-6-astra"),
    ),
    (
        "GPT-6.1 Sol",
        "/models/openai/gpt-6-1-sol",
        Some("openai/gpt-6.1-sol"),
    ),
    ("Gemini 4 Argon", "/models/google/gemini-4-argon", None),
    ("Grok 4.7", "/models/xai/grok-4-7", Some("x-ai/grok-4.7")),
    (
        "Muse Spark 1.3",
        "/models/meta/muse-spark-1-3",
        Some("meta/muse-spark-1.3"),
    ),
    (
        "Qwen3.8 Max",
        "/models/alibaba/qwen3-8-max",
        Some("qwen/qwen3.8-max-0902"),
    ),
    (
        "DeepSeek V4 Pro",
        "/models/deepseek/deepseek-v4-pro",
        Some("deepseek/deepseek-v4-pro"),
    ),
];

struct Filing {
    filed: i64,
    period: Option<i64>,
    rows: Vec<AtmRow>,
}

struct AtmRow {
    ticker: String,
    net_m: f64,
    avail_m: f64,
}

/// One round of terminal readouts plus which feeds answered.
pub struct Enriched {
    pub terminal: Terminal,
    /// Coin Metrics daily reference prices, empty when the feed failed.
    pub btc_daily: Vec<Point>,
    pub feeds: Vec<&'static str>,
    pub notes: Vec<String>,
}

pub fn enrich(client: &Client, snap: &crate::model::Snapshot) -> Enriched {
    let ymd = |days: i64| {
        (chrono::Utc::now() - chrono::Duration::days(days))
            .format("%Y-%m-%d")
            .to_string()
    };
    let start_4y = ymd(365 * 4);
    let start_400 = ymd(400);
    let start_liq = "2015-01-01".to_string();
    let start_u = ymd(365 * 3);
    // CNBC history (fetched just before) covers MOVE and a fresher VIX than FRED.
    let cnbc = |key: &str| {
        snap.histories
            .get(key)
            .filter(|points| points.len() >= 30)
            .cloned()
    };
    let cnbc_move = cnbc("MOVE");
    let cnbc_vix = cnbc("VIX");

    let (
        prices,
        mvrv,
        nupl,
        realized,
        walcl,
        tga,
        rrp,
        dxy,
        real10,
        hy,
        vix,
        spx,
        ndx,
        fed,
        m2,
        uranium,
        move_px,
        strc_px,
        gold,
        coinbase,
        binance_h,
        binance_f,
        bybit_f,
        okx_f,
        farside,
        volume,
        filings,
        models,
    ) = thread::scope(|scope| {
        let prices = scope.spawn(|| coinmetrics_prices(client, "btc", "2010-07-17"));
        let mvrv = scope.spawn(|| bitcoin_data(client, "mvrv-zscore", "mvrvZscore"));
        let nupl = scope.spawn(|| bitcoin_data(client, "nupl", "nupl"));
        let realized = scope.spawn(|| bitcoin_data(client, "realized-price", "realizedPrice"));
        let walcl = scope.spawn(|| fred_points(client, "WALCL", &start_liq));
        let tga = scope.spawn(|| fred_points(client, "WTREGEN", &start_liq));
        let rrp = scope.spawn(|| fred_points(client, "RRPONTSYD", &start_liq));
        let dxy = scope.spawn(|| fred_points(client, "DTWEXBGS", &start_4y));
        let real10 = scope.spawn(|| fred_points(client, "DFII10", &start_400));
        let hy = scope.spawn(|| fred_points(client, "BAMLH0A0HYM2", &start_400));
        let vix = scope.spawn(|| fred_points(client, "VIXCLS", &start_400));
        let spx = scope.spawn(|| fred_points(client, "SP500", &start_400));
        let ndx = scope.spawn(|| fred_points(client, "NASDAQ100", &start_400));
        let fed = scope.spawn(|| fred_points(client, "FEDFUNDS", &start_4y));
        let m2 = scope.spawn(|| fred_points(client, "WM2NS", &start_4y));
        let uranium = scope.spawn(|| fred_points(client, "PURANUSDM", &start_u));
        let move_px = scope.spawn(|| match &cnbc_move {
            Some(points) => Some(points.clone()),
            None => yahoo_daily(client, "^MOVE", "3mo"),
        });
        let strc_px = scope.spawn(|| yahoo_daily(client, "STRC", "1mo"));
        let gold = scope.spawn(|| gold_series(client));
        let coinbase = scope.spawn(|| coinbase_hourly(client));
        let binance_h = scope.spawn(|| {
            get(
                client,
                "https://api.binance.com/api/v3/klines?symbol=BTCUSDT&interval=1h&limit=72",
            )
            .map(|body| parse::klines(&body))
            .unwrap_or_default()
        });
        let binance_f = scope.spawn(|| binance_funding(client));
        let bybit_f = scope.spawn(|| bybit_funding(client));
        let okx_f = scope.spawn(|| okx_funding(client));
        let farside = scope.spawn(|| {
            get(client, "https://farside.co.uk/bitcoin-etf-flow-all-data/").unwrap_or_default()
        });
        let volume = scope.spawn(|| kraken_volume(client));
        let filings = scope.spawn(|| edgar_filings(client));
        let models = scope.spawn(|| frontier_with_outside(client));
        (
            prices.join(),
            mvrv.join(),
            nupl.join(),
            realized.join(),
            walcl.join(),
            tga.join(),
            rrp.join(),
            dxy.join(),
            real10.join(),
            hy.join(),
            vix.join(),
            spx.join(),
            ndx.join(),
            fed.join(),
            m2.join(),
            uranium.join(),
            move_px.join(),
            strc_px.join(),
            gold.join(),
            coinbase.join(),
            binance_h.join(),
            binance_f.join(),
            bybit_f.join(),
            okx_f.join(),
            farside.join(),
            volume.join(),
            filings.join(),
            models.join(),
        )
    });

    let prices = opt_series(prices);
    let mvrv = opt_series(mvrv);
    let nupl = opt_series(nupl);
    let realized = opt_series(realized);
    let walcl = opt_series(walcl);
    let tga = opt_series(tga);
    let rrp = opt_series(rrp);
    let dxy = opt_series(dxy);
    let real10 = opt_series(real10);
    let hy = opt_series(hy);
    let fred_vix = opt_series(vix);
    let spx = opt_series(spx);
    let ndx = opt_series(ndx);
    let fed = opt_series(fed);
    let m2 = opt_series(m2);
    let uranium = opt_series(uranium);
    let move_px = opt_series(move_px);
    let strc_px = opt_series(strc_px);
    let gold = opt_series(gold);
    let coinbase = opt_series(coinbase);
    let binance_h = plain_series(binance_h);
    let binance_f = opt_rates(binance_f);
    let bybit_f = opt_rates(bybit_f);
    let okx_f = opt_rates(okx_f);
    let farside = plain_text(farside);
    let volume = opt_series(volume);
    let filings = joined_filings(filings);
    let models = joined_models(models);

    let mut feeds = Vec::new();
    let mut notes = Vec::new();
    let liquidity = net_liquidity(&walcl, &tga, &rrp);
    if liquidity.is_empty() {
        notes.push(
            "FRED net liquidity needs WALCL, TGA and reverse repo observations; keeping the last complete calculation."
                .into(),
        );
    }
    for (id, answered) in [
        ("coinmetrics", !prices.is_empty()),
        (
            "btc-data",
            !mvrv.is_empty() || !nupl.is_empty() || !realized.is_empty(),
        ),
        ("fred-liquidity", !liquidity.is_empty()),
        (
            "fred-macro",
            [&real10, &hy, &fred_vix, &spx, &ndx, &m2]
                .iter()
                .any(|s| !s.is_empty()),
        ),
        ("fred-uranium", !uranium.is_empty()),
        (
            "yahoo",
            !strc_px.is_empty() || (cnbc_move.is_none() && !move_px.is_empty()),
        ),
        ("coingecko-paxg", !gold.is_empty()),
        ("coinbase", !coinbase.is_empty()),
        ("bybit", !bybit_f.is_empty()),
        ("okx", !okx_f.is_empty()),
        ("kraken", !volume.is_empty()),
        ("edgar", !filings.is_empty()),
    ] {
        if answered {
            feeds.push(id);
        }
    }
    // The CNBC close is a day fresher than FRED VIXCLS; use it wherever it exists.
    let vix = cnbc_vix.unwrap_or(fred_vix);

    let mut out = Terminal::default();
    let daily = if prices.is_empty() {
        snap.btc.daily.clone()
    } else {
        prices.clone()
    };
    if let Some(high) = daily
        .iter()
        .filter(|p| p.v.is_finite() && p.v > 0.0)
        .max_by(|a, b| a.v.total_cmp(&b.v))
    {
        out.ath = Some(high.v);
        out.ath_unix = Some(high.t);
    }
    if daily.len() >= 200 {
        let sma = daily[daily.len() - 200..].iter().map(|p| p.v).sum::<f64>() / 200.0;
        let last = daily.last().map(|p| p.v).unwrap_or(0.0);
        if sma > 0.0 && last > 0.0 {
            out.mayer = Some(last / sma);
        }
    }
    out.mvrv_z = mvrv.last().map(|p| p.v);
    out.nupl = nupl.last().map(|p| p.v);
    out.realized = realized.last().map(|p| p.v);
    let regime = regime_book(&fed, &m2, &mvrv, &daily, &dxy, now_unix());
    if let Err(missing) = &regime {
        notes.push(format!(
            "Regime score needs all five inputs (missing: {}); keeping the last complete score.",
            missing.join(", ")
        ));
    }
    if let Ok(regime) = regime {
        out.regime_score = Some(regime.0);
        out.regime_alloc = Some(regime.1);
        out.regime_inputs = regime.2;
        out.regime_hist = regime.3;
    }
    if let Some(last) = liquidity.last() {
        out.net_liq_version = NET_LIQ_VERSION;
        out.net_liq_b = Some(last.v);
        out.net_liq_30d_b = delta_back(&liquidity, 30);
        out.net_liq_hist = liquidity[liquidity.len().saturating_sub(400)..].to_vec();
    }
    let venues = vec![
        venue("Binance", binance_f),
        venue("Bybit", bybit_f),
        venue("OKX", okx_f),
    ];
    let live: Vec<&VenuePrint> = venues.iter().filter(|v| v.avg_7d.is_some()).collect();
    if !live.is_empty() {
        let avg = live.iter().filter_map(|v| v.avg_7d).sum::<f64>() / live.len() as f64;
        out.funding_avg_7d = Some(avg);
        out.funding_ann = Some(avg * 3.0 * 365.0);
    }
    out.venues = venues;
    let (prem, prem_avg, prem_hist) = coinbase_premium(&coinbase, &binance_h);
    out.premium_bps = prem;
    out.premium_avg_bps = prem_avg;
    out.premium_hist = prem_hist;
    out.correlations = vec![
        corr_print("S&P 500", &daily, &spx),
        corr_print("NASDAQ 100", &daily, &ndx),
        corr_print("Gold", &daily, &gold),
        corr_print("Broad USD", &daily, &dxy),
        corr_print("VIX", &daily, &vix),
        corr_print("10Y Real Yield", &daily, &real10),
    ];
    out.macro_strip = vec![
        macro_print("Broad USD", "", &dxy, "FRED DTWEXBGS"),
        macro_print("10Y Real Yield", "%", &real10, "FRED DFII10"),
        macro_print("HY Spread", "%", &hy, "FRED BAMLH0A0HYM2"),
        macro_print(
            "VIX",
            "",
            &vix,
            if snap.histories.contains_key("VIX") {
                "CNBC daily close"
            } else {
                "FRED VIXCLS"
            },
        ),
        macro_print(
            "MOVE",
            "",
            &move_px,
            if cnbc_move.is_some() {
                "CNBC daily close"
            } else {
                "Yahoo Finance"
            },
        ),
    ];
    if let Some(mei) = mei_latest(&daily, &volume) {
        out.mei_state = mei.0;
        out.mei_valence = mei.1;
        out.mei_arousal = mei.2;
        out.mei_intensity = mei.3;
    }
    if !farside.is_empty() {
        let days = parse_farside(&farside);
        if !days.is_empty() {
            let summary = summarise_flows(&days);
            out.etf_latest_m = summary.0;
            out.etf_latest_unix = days.last().map(|d| d.0);
            out.etf_7d_m = Some(summary.1);
            out.etf_30d_m = Some(summary.2);
            out.etf_cum_m = Some(summary.3);
            let flows: Vec<Point> = days
                .iter()
                .filter_map(|(t, v)| v.map(|v| Point { t: *t, v }))
                .collect();
            out.etf_flows = flows[flows.len().saturating_sub(120)..].to_vec();
            feeds.push("farside");
        }
    }
    apply_strategy(&mut out, &filings, &strc_px, &daily);
    if let Some(last) = uranium.last() {
        out.uranium = Some(last.v);
        out.uranium_as_of = chrono::DateTime::from_timestamp(last.t, 0)
            .map(|t| t.format("%Y-%m-%d").to_string())
            .unwrap_or_default();
        if uranium.len() > 12 {
            let prior = uranium[uranium.len() - 13].v;
            if prior > 0.0 {
                out.uranium_12m = Some((last.v - prior) / prior * 100.0);
            }
        }
        out.uranium_lo = uranium.iter().map(|p| p.v).min_by(f64::total_cmp);
        out.uranium_hi = uranium.iter().map(|p| p.v).max_by(f64::total_cmp);
        out.uranium_hist = uranium.clone();
    }
    match models {
        Some((list, source, outside)) => {
            feeds.push(source);
            for missing in ["epoch", "vals"].into_iter().filter(|f| !outside.contains(f)) {
                notes.push(format!("{} model scores unavailable; keeping the last ones.", if missing == "epoch" { "Epoch AI" } else { "Vals.ai" }));
            }
            feeds.extend(outside);
            if !list.iter().any(|m| m.intelligence.is_some()) {
                notes.push("Artificial Analysis leaderboard was unreadable; scores keep their last refresh.".into());
            }
            out.models = list;
        }
        None => notes.push("Frontier model feeds unavailable; keeping the previous roster.".into()),
    }
    Enriched {
        terminal: out,
        btc_daily: prices,
        feeds,
        notes,
    }
}

fn opt_series(result: thread::Result<Option<Vec<Point>>>) -> Vec<Point> {
    result.ok().flatten().unwrap_or_default()
}

fn plain_series(result: thread::Result<Vec<Point>>) -> Vec<Point> {
    result.unwrap_or_default()
}

fn opt_rates(result: thread::Result<Option<Vec<f64>>>) -> Vec<f64> {
    result.ok().flatten().unwrap_or_default()
}

fn plain_text(result: thread::Result<String>) -> String {
    result.unwrap_or_default()
}

fn joined_filings(result: thread::Result<Option<Vec<Filing>>>) -> Vec<Filing> {
    result.ok().flatten().unwrap_or_default()
}

type Roster = (Vec<ModelPrint>, &'static str, Vec<&'static str>);

fn joined_models(result: thread::Result<Option<Roster>>) -> Option<Roster> {
    result.ok().flatten()
}

pub fn merge(prev: &Terminal, fresh: Terminal) -> Terminal {
    let mut out = prev.clone();
    invalidate_legacy_liquidity(&mut out);
    out.ath = fresh.ath.or(out.ath);
    out.ath_unix = fresh.ath_unix.or(out.ath_unix);
    out.mayer = fresh.mayer.or(out.mayer);
    out.mvrv_z = fresh.mvrv_z.or(out.mvrv_z);
    out.nupl = fresh.nupl.or(out.nupl);
    out.realized = fresh.realized.or(out.realized);
    if fresh.net_liq_version == NET_LIQ_VERSION && fresh.net_liq_b.is_some() {
        out.net_liq_version = NET_LIQ_VERSION;
        out.net_liq_b = fresh.net_liq_b;
        out.net_liq_30d_b = fresh.net_liq_30d_b;
        out.net_liq_hist = fresh.net_liq_hist;
    }
    if fresh.funding_ann.is_some() {
        out.funding_avg_7d = fresh.funding_avg_7d.or(out.funding_avg_7d);
        out.funding_ann = fresh.funding_ann;
    }
    out.venues = merge_venues(&out.venues, &fresh.venues);
    out.premium_bps = fresh.premium_bps.or(out.premium_bps);
    out.premium_avg_bps = fresh.premium_avg_bps.or(out.premium_avg_bps);
    if !fresh.premium_hist.is_empty() {
        out.premium_hist = fresh.premium_hist;
    }
    out.correlations = merge_corr(&out.correlations, &fresh.correlations);
    out.macro_strip = merge_macro(&out.macro_strip, &fresh.macro_strip);
    if !fresh.mei_state.is_empty() {
        out.mei_state = fresh.mei_state;
        out.mei_valence = fresh.mei_valence.or(out.mei_valence);
        out.mei_arousal = fresh.mei_arousal.or(out.mei_arousal);
        out.mei_intensity = fresh.mei_intensity.or(out.mei_intensity);
    }
    if fresh.etf_latest_unix.is_some() || fresh.etf_cum_m.is_some() {
        // A new day may still be pending. Never attach an older total to its date.
        if fresh.etf_latest_unix.is_some() {
            out.etf_latest_m = if fresh.etf_latest_unix == out.etf_latest_unix {
                fresh.etf_latest_m.or(out.etf_latest_m)
            } else {
                fresh.etf_latest_m
            };
            out.etf_latest_unix = fresh.etf_latest_unix;
        }
        out.etf_7d_m = fresh.etf_7d_m.or(out.etf_7d_m);
        out.etf_30d_m = fresh.etf_30d_m.or(out.etf_30d_m);
        out.etf_cum_m = fresh.etf_cum_m.or(out.etf_cum_m);
        if !fresh.etf_flows.is_empty() {
            out.etf_flows = fresh.etf_flows;
        }
    }
    if fresh.regime_score.is_some() {
        out.regime_score = fresh.regime_score;
        out.regime_alloc = fresh.regime_alloc.or(out.regime_alloc);
        if !fresh.regime_inputs.is_empty() {
            out.regime_inputs = fresh.regime_inputs;
        }
        if !fresh.regime_hist.is_empty() {
            out.regime_hist = fresh.regime_hist;
        }
    }
    out.strc_price = fresh.strc_price.or(out.strc_price);
    out.strc_change = fresh.strc_change.or(out.strc_change);
    out.strc_week_m = fresh.strc_week_m.or(out.strc_week_m);
    out.strc_btc = fresh.strc_btc.or(out.strc_btc);
    if fresh.strc_recent_m.is_some() {
        out.strc_recent_m = fresh.strc_recent_m;
        out.strc_filings = fresh.strc_filings;
    }
    out.strc_left_m = fresh.strc_left_m.or(out.strc_left_m);
    out.atm_cap_b = fresh.atm_cap_b.or(out.atm_cap_b);
    out.strc_filed = fresh.strc_filed.or(out.strc_filed);
    if !fresh.strc_hist.is_empty() {
        out.strc_hist = fresh.strc_hist;
    }
    out.uranium = fresh.uranium.or(out.uranium);
    out.uranium_12m = fresh.uranium_12m.or(out.uranium_12m);
    out.uranium_lo = fresh.uranium_lo.or(out.uranium_lo);
    out.uranium_hi = fresh.uranium_hi.or(out.uranium_hi);
    if !fresh.uranium_as_of.is_empty() {
        out.uranium_as_of = fresh.uranium_as_of;
    }
    if !fresh.uranium_hist.is_empty() {
        out.uranium_hist = fresh.uranium_hist;
    }
    out.models = merge_models(&out.models, fresh.models);
    out
}

/// Keep all unrelated cached readings when upgrading the liquidity formula.
/// Returns true when the old calculation (and its feed stamp) must be discarded.
pub fn invalidate_legacy_liquidity(terminal: &mut Terminal) -> bool {
    if terminal.net_liq_version == NET_LIQ_VERSION {
        return false;
    }
    terminal.net_liq_version = 0;
    terminal.net_liq_b = None;
    terminal.net_liq_30d_b = None;
    terminal.net_liq_hist.clear();
    true
}

fn merge_models(prev: &[ModelPrint], fresh: Vec<ModelPrint>) -> Vec<ModelPrint> {
    if fresh.is_empty() {
        return prev.to_vec();
    }
    // A scored refresh defines the current ranked roster. A pricing fallback
    // only updates fields on the last roster, which can include newer models.
    if fresh.iter().any(|m| m.intelligence.is_some()) {
        fresh
            .into_iter()
            .map(|m| {
                let old = prev
                    .iter()
                    .find(|o| model_key(&o.label) == model_key(&m.label));
                merge_model(old, m)
            })
            .collect()
    } else {
        let mut out = prev.to_vec();
        for m in fresh {
            if let Some(index) = out
                .iter()
                .position(|o| model_key(&o.label) == model_key(&m.label))
            {
                out[index] = merge_model(Some(&out[index]), m);
            } else {
                out.push(m);
            }
        }
        out
    }
}

fn merge_model(old: Option<&ModelPrint>, fresh: ModelPrint) -> ModelPrint {
    let Some(old) = old else { return fresh };
    let mut out = fresh;
    let pricing_only = out.intelligence.is_none();
    if pricing_only {
        // Preserve the scored effort variant and lab when only prices arrived.
        out.label = old.label.clone();
        out.intelligence = old.intelligence;
    }
    if out.creator.is_empty() {
        out.creator = old.creator.clone();
    }
    // The AA roster can change its selected effort or evaluation suite. Only a
    // pricing fallback retains its old AA benchmarks; outside evaluators fail
    // independently and can keep their own absent results on either path.
    for (key, value) in &old.scores {
        if pricing_only || OUTSIDE_SCORES.contains(&key.as_str()) {
            out.scores.entry(key.clone()).or_insert(*value);
        }
    }
    out
}

fn delta_back(series: &[Point], days: i64) -> Option<f64> {
    let last = series.last()?;
    let cutoff = last.t - days * SECONDS_PER_DAY;
    let prev = series.iter().rev().find(|p| p.t <= cutoff)?;
    Some(last.v - prev.v)
}

pub(crate) fn net_liquidity(walcl: &[Point], tga: &[Point], rrp: &[Point]) -> Vec<Point> {
    if walcl.is_empty() || tga.is_empty() || rrp.is_empty() {
        return Vec::new();
    }
    let start = [walcl, tga, rrp]
        .into_iter()
        .filter_map(|s| s.first().map(|p| p.t))
        .max()
        .unwrap_or(0);
    let end = [walcl, tga, rrp]
        .into_iter()
        .filter_map(|s| s.last().map(|p| p.t))
        .max()
        .unwrap_or(start);
    let fill = |series: &[Point], t: i64, cursor: &mut (usize, Option<f64>)| -> Option<f64> {
        while cursor.0 < series.len() && series[cursor.0].t <= t {
            cursor.1 = Some(series[cursor.0].v);
            cursor.0 += 1;
        }
        cursor.1
    };
    let mut w_c = (0usize, None);
    let mut t_c = (0usize, None);
    let mut r_c = (0usize, None);
    let mut out = Vec::new();
    let mut t = start;
    while t <= end {
        let w = fill(walcl, t, &mut w_c);
        let tg = fill(tga, t, &mut t_c);
        let r = fill(rrp, t, &mut r_c);
        if let (Some(w), Some(tg), Some(r)) = (w, tg, r) {
            // FRED WALCL/WTREGEN are millions; RRPONTSYD is already billions.
            let v = (w - tg) / 1000.0 - r;
            if v.is_finite() {
                out.push(Point { t, v });
            }
        }
        if t > i64::MAX - SECONDS_PER_DAY {
            break;
        }
        t += SECONDS_PER_DAY;
    }
    out
}

pub(crate) fn macro_print(label: &str, unit: &str, series: &[Point], source: &str) -> MacroPrint {
    let Some(latest) = series.last() else {
        return MacroPrint {
            label: label.into(),
            unit: unit.into(),
            source: source.into(),
            ..Default::default()
        };
    };
    let cutoff = latest.t - 30 * SECONDS_PER_DAY;
    let mut month_ago = series.iter().rev().find(|p| p.t <= cutoff).map(|p| p.v);
    if month_ago.is_none() && series.len() > 1 {
        month_ago = Some(series[0].v);
    }
    MacroPrint {
        label: label.into(),
        current: Some(latest.v),
        delta_30d: month_ago.map(|p| latest.v - p),
        unit: unit.into(),
        as_of: chrono::DateTime::from_timestamp(latest.t, 0)
            .map(|t| t.format("%Y-%m-%d").to_string())
            .unwrap_or_default(),
        prev: series.len().checked_sub(2).map(|i| series[i].v),
        hist: series[series.len().saturating_sub(120)..]
            .iter()
            .map(|p| p.v)
            .collect(),
        source: source.into(),
    }
}

pub(crate) fn corr_print(label: &str, btc: &[Point], asset: &[Point]) -> CorrPrint {
    let mut btc = btc.to_vec();
    let mut asset = asset.to_vec();
    btc.sort_by_key(|p| p.t);
    asset.sort_by_key(|p| p.t);
    let mut map = HashMap::new();
    for point in &btc {
        map.insert(point.t.div_euclid(SECONDS_PER_DAY), point.v);
    }
    let mut left = Vec::new();
    let mut right = Vec::new();
    for point in &asset {
        if let Some(v) = map.get(&point.t.div_euclid(SECONDS_PER_DAY)) {
            left.push(*v);
            right.push(point.v);
        }
    }
    if left.len() < 91 {
        return CorrPrint {
            label: label.into(),
            r30: None,
            r90: None,
        };
    }
    let a = log_returns(&left);
    let b = log_returns(&right);
    CorrPrint {
        label: label.into(),
        r30: last_pearson(&a, &b, 30),
        r90: last_pearson(&a, &b, 90),
    }
}

fn log_returns(prices: &[f64]) -> Vec<f64> {
    let mut out = Vec::new();
    for pair in prices.windows(2) {
        if pair[0] > 0.0 && pair[1] > 0.0 {
            out.push((pair[1] / pair[0]).ln());
        } else {
            out.push(0.0);
        }
    }
    out
}

fn last_pearson(a: &[f64], b: &[f64], window: usize) -> Option<f64> {
    if a.len() != b.len() || a.len() < window || window < 2 {
        return None;
    }
    let i = a.len();
    Some(pearson(&a[i - window..i], &b[i - window..b.len()]))
}

pub(crate) fn pearson(a: &[f64], b: &[f64]) -> f64 {
    let n = a.len();
    if n < 2 || n != b.len() {
        return f64::NAN;
    }
    let mean_a = a.iter().sum::<f64>() / n as f64;
    let mean_b = b.iter().sum::<f64>() / n as f64;
    let mut num = 0.0;
    let mut va = 0.0;
    let mut vb = 0.0;
    for i in 0..n {
        let da = a[i] - mean_a;
        let db = b[i] - mean_b;
        num += da * db;
        va += da * da;
        vb += db * db;
    }
    let den = (va * vb).sqrt();
    if den == 0.0 || !den.is_finite() {
        0.0
    } else {
        num / den
    }
}

fn value_at_or_before(series: &[Point], ts: i64) -> Option<f64> {
    if series.is_empty() {
        return None;
    }
    let mut lo = 0usize;
    let mut hi = series.len() - 1;
    let mut found = None;
    while lo <= hi {
        let mid = (lo + hi) / 2;
        if series[mid].t <= ts {
            found = Some(mid);
            lo = mid + 1;
        } else if mid == 0 {
            break;
        } else {
            hi = mid - 1;
        }
    }
    found.map(|i| series[i].v).filter(|v| v.is_finite())
}

fn score_fed(series: &[Point], ts: i64) -> i32 {
    let now = value_at_or_before(series, ts);
    let past = value_at_or_before(series, ts - 6 * 30 * SECONDS_PER_DAY);
    match (now, past) {
        (Some(now), Some(past)) if now - past <= -0.25 => 1,
        (Some(now), Some(past)) if now - past >= 0.25 => -1,
        _ => 0,
    }
}

fn score_m2(series: &[Point], ts: i64) -> i32 {
    let now = value_at_or_before(series, ts);
    let past = value_at_or_before(series, ts - 365 * SECONDS_PER_DAY);
    match (now, past) {
        (Some(now), Some(past)) if past != 0.0 => {
            let yoy = (now - past) / past * 100.0;
            if yoy > 3.0 {
                1
            } else if yoy < -1.0 {
                -1
            } else {
                0
            }
        }
        _ => 0,
    }
}

fn score_mvrv(series: &[Point], ts: i64) -> i32 {
    match value_at_or_before(series, ts) {
        Some(v) if v < 2.0 => 1,
        Some(v) if v > 5.0 => -1,
        _ => 0,
    }
}

fn weekly_closes(daily: &[Point], ts: i64) -> Vec<f64> {
    let mut weekly = Vec::new();
    let mut last_key: Option<String> = None;
    let mut last_value: Option<f64> = None;
    for point in daily {
        if point.t > ts {
            break;
        }
        let key = week_key(point.t);
        if last_key.as_ref() != Some(&key) {
            if let Some(v) = last_value {
                weekly.push(v);
            }
            last_key = Some(key);
        }
        last_value = Some(point.v);
    }
    if let Some(v) = last_value {
        weekly.push(v);
    }
    weekly
}

fn week_key(ts: i64) -> String {
    let dt = chrono::DateTime::from_timestamp(ts, 0).unwrap_or(chrono::DateTime::UNIX_EPOCH);
    let year = dt.year();
    let one_jan = chrono::NaiveDate::from_ymd_opt(year, 1, 1)
        .and_then(|d| d.and_hms_opt(0, 0, 0))
        .map(|d| d.and_utc().timestamp())
        .unwrap_or(ts);
    let day_of_year = (ts - one_jan).div_euclid(SECONDS_PER_DAY) + 1;
    let dow = (dt.weekday().num_days_from_sunday() as i64 + 6) % 7;
    let week = ((day_of_year + 6 - dow) as f64 / 7.0).ceil() as i64;
    format!("{year}-W{week}")
}

fn score_btc(daily: &[Point], ts: i64) -> i32 {
    let weekly = weekly_closes(daily, ts);
    if weekly.len() < 200 {
        return 0;
    }
    let window = &weekly[weekly.len() - 200..];
    let sma = window.iter().sum::<f64>() / 200.0;
    let close = *weekly.last().unwrap_or(&0.0);
    if sma <= 0.0 {
        return 0;
    }
    if close >= sma * 1.10 {
        1
    } else if close <= sma * 0.90 {
        -1
    } else {
        0
    }
}

fn score_dxy(series: &[Point], ts: i64) -> i32 {
    let now = value_at_or_before(series, ts);
    let past = value_at_or_before(series, ts - 6 * 30 * SECONDS_PER_DAY);
    match (now, past) {
        (Some(now), Some(past)) if past != 0.0 => {
            let chg = (now - past) / past * 100.0;
            if chg < -2.0 {
                1
            } else if chg > 2.0 {
                -1
            } else {
                0
            }
        }
        _ => 0,
    }
}

fn alloc_from_score(score: i32) -> f64 {
    match score {
        3.. => 100.0,
        2 => 75.0,
        1 => 50.0,
        0 => 25.0,
        -1 => 10.0,
        _ => 0.0,
    }
}

/// Score, target allocation, per-input detail and two years of weekly history.
type RegimeBook = (i32, f64, Vec<ScorePrint>, Vec<Point>);

fn regime_book(
    fed: &[Point],
    m2: &[Point],
    mvrv: &[Point],
    daily: &[Point],
    dxy: &[Point],
    now: i64,
) -> Result<RegimeBook, Vec<String>> {
    let inputs = regime_inputs(fed, m2, mvrv, daily, dxy, now);
    // A missing input scores zero, which reads as a real neutral vote and can
    // move the allocation by itself. Only a complete set is a score.
    let missing: Vec<String> = inputs
        .iter()
        .filter(|row| row.detail.starts_with("insufficient"))
        .map(|row| row.label.clone())
        .collect();
    if !missing.is_empty() {
        return Err(missing);
    }
    let score: i32 = inputs.iter().map(|row| row.score).sum();
    let history_start = now - 2 * 365 * SECONDS_PER_DAY;
    let mut history = Vec::new();
    let mut last_key = String::new();
    for point in daily {
        if point.t < history_start || point.t > now {
            continue;
        }
        let key = week_key(point.t);
        if key == last_key {
            continue;
        }
        last_key = key;
        let total = score_fed(fed, point.t)
            + score_m2(m2, point.t)
            + score_mvrv(mvrv, point.t)
            + score_btc(daily, point.t)
            + score_dxy(dxy, point.t);
        history.push(Point {
            t: point.t,
            v: total as f64,
        });
    }
    Ok((score, alloc_from_score(score), inputs, history))
}

fn regime_inputs(
    fed: &[Point],
    m2: &[Point],
    mvrv: &[Point],
    daily: &[Point],
    dxy: &[Point],
    now: i64,
) -> Vec<ScorePrint> {
    let fed_now = value_at_or_before(fed, now);
    let fed_past = value_at_or_before(fed, now - 180 * SECONDS_PER_DAY);
    let m2_now = value_at_or_before(m2, now);
    let m2_past = value_at_or_before(m2, now - 365 * SECONDS_PER_DAY);
    let mvrv_now = value_at_or_before(mvrv, now);
    let dxy_now = value_at_or_before(dxy, now);
    let dxy_past = value_at_or_before(dxy, now - 180 * SECONDS_PER_DAY);
    let weekly = weekly_closes(daily, now);
    vec![
        ScorePrint {
            label: "Fed funds".into(),
            score: score_fed(fed, now),
            detail: describe_fed(fed_now, fed_past),
        },
        ScorePrint {
            label: "M2 YoY".into(),
            score: score_m2(m2, now),
            detail: describe_m2(m2_now, m2_past),
        },
        ScorePrint {
            label: "MVRV Z".into(),
            score: score_mvrv(mvrv, now),
            detail: describe_mvrv(mvrv_now),
        },
        ScorePrint {
            label: "BTC vs 200w SMA".into(),
            score: score_btc(daily, now),
            detail: describe_btc(&weekly),
        },
        ScorePrint {
            label: "USD 6m trend".into(),
            score: score_dxy(dxy, now),
            detail: describe_dxy(dxy_now, dxy_past),
        },
    ]
}

fn describe_fed(now: Option<f64>, past: Option<f64>) -> String {
    match (now, past) {
        (Some(now), Some(past)) => {
            let diff = now - past;
            let dir = if diff <= -0.25 {
                "falling"
            } else if diff >= 0.25 {
                "rising"
            } else {
                "flat"
            };
            format!("now {now:.2}% · 6m ago {past:.2}% · {dir} (Δ {diff:+.2} pp)")
        }
        _ => "insufficient data".into(),
    }
}

fn describe_m2(now: Option<f64>, past: Option<f64>) -> String {
    match (now, past) {
        (Some(now), Some(past)) if past != 0.0 => {
            let yoy = (now - past) / past * 100.0;
            format!("YoY {yoy:+.2}%")
        }
        _ => "insufficient data".into(),
    }
}

fn describe_mvrv(v: Option<f64>) -> String {
    match v {
        Some(v) => {
            let zone = if v < 2.0 {
                "value zone"
            } else if v > 5.0 {
                "overheated"
            } else {
                "neutral"
            };
            format!("Z {v:.2} · {zone}")
        }
        None => "insufficient data".into(),
    }
}

fn describe_btc(weekly: &[f64]) -> String {
    if weekly.len() < 200 {
        return "insufficient weekly history".into();
    }
    let window = &weekly[weekly.len() - 200..];
    let sma = window.iter().sum::<f64>() / 200.0;
    let close = *weekly.last().unwrap_or(&0.0);
    if sma <= 0.0 {
        return "insufficient weekly history".into();
    }
    let pct = (close - sma) / sma * 100.0;
    format!("close ${close:.0} · 200w SMA ${sma:.0} · {pct:+.1}%")
}

fn describe_dxy(now: Option<f64>, past: Option<f64>) -> String {
    match (now, past) {
        (Some(now), Some(past)) if past != 0.0 => {
            let chg = (now - past) / past * 100.0;
            format!("now {now:.2} · 6m ago {past:.2} · {chg:+.1}%")
        }
        _ => "insufficient data".into(),
    }
}

fn venue(name: &str, rates: Vec<f64>) -> VenuePrint {
    if rates.is_empty() {
        return VenuePrint {
            name: name.into(),
            current: None,
            avg_7d: None,
        };
    }
    VenuePrint {
        name: name.into(),
        current: rates.last().copied(),
        avg_7d: Some(rates.iter().sum::<f64>() / rates.len() as f64),
    }
}

fn merge_venues(prev: &[VenuePrint], fresh: &[VenuePrint]) -> Vec<VenuePrint> {
    ["Binance", "Bybit", "OKX"]
        .into_iter()
        .filter_map(|name| {
            let fresh = fresh.iter().find(|v| v.name == name);
            let prev = prev.iter().find(|v| v.name == name);
            match fresh {
                Some(row) if row.current.is_some() || row.avg_7d.is_some() => Some(row.clone()),
                _ => prev.cloned(),
            }
        })
        .collect()
}

fn merge_macro(prev: &[MacroPrint], fresh: &[MacroPrint]) -> Vec<MacroPrint> {
    let labels = ["Broad USD", "10Y Real Yield", "HY Spread", "VIX", "MOVE"];
    labels
        .into_iter()
        .filter_map(|label| {
            let fresh = fresh.iter().find(|m| m.label == label);
            let prev = prev.iter().find(|m| m.label == label);
            match fresh {
                Some(row) if row.current.is_some() => Some(row.clone()),
                _ => prev.cloned(),
            }
        })
        .collect()
}

fn merge_corr(prev: &[CorrPrint], fresh: &[CorrPrint]) -> Vec<CorrPrint> {
    let labels = [
        "S&P 500",
        "NASDAQ 100",
        "Gold",
        "Broad USD",
        "VIX",
        "10Y Real Yield",
    ];
    labels
        .into_iter()
        .filter_map(|label| {
            let fresh = fresh.iter().find(|m| m.label == label);
            let prev = prev.iter().find(|m| m.label == label);
            match fresh {
                Some(row) if row.r30.is_some() || row.r90.is_some() => Some(row.clone()),
                _ => prev.cloned(),
            }
        })
        .collect()
}

pub(crate) fn coinbase_premium(
    cb: &[Point],
    bn: &[Point],
) -> (Option<f64>, Option<f64>, Vec<Point>) {
    let mut cb_map = HashMap::new();
    for point in cb {
        cb_map.insert(point.t.div_euclid(3600), point.v);
    }
    let mut series = Vec::new();
    for point in bn {
        let hour = point.t.div_euclid(3600);
        let Some(cb_px) = cb_map.get(&hour) else {
            continue;
        };
        if point.v <= 0.0 {
            continue;
        }
        series.push((point.t, (cb_px - point.v) / point.v * 10_000.0));
    }
    series.sort_by_key(|p| p.0);
    let history: Vec<Point> = series.iter().map(|(t, v)| Point { t: *t, v: *v }).collect();
    let current = history.last().map(|p| p.v);
    let start = history.len().saturating_sub(24);
    let avg = if history.is_empty() {
        None
    } else {
        Some(history[start..].iter().map(|p| p.v).sum::<f64>() / (history.len() - start) as f64)
    };
    (current, avg, history)
}

pub(crate) fn parse_farside(html: &str) -> Vec<(i64, Option<f64>)> {
    let Some(start) = html.to_ascii_lowercase().find("class=\"etf") else {
        return Vec::new();
    };
    let Some(table_open) = html[..start].rfind("<table") else {
        return Vec::new();
    };
    let Some(table_end) = html[start..].to_ascii_lowercase().find("</table>") else {
        return Vec::new();
    };
    let table = &html[table_open..start + table_end];
    let rows = html_rows(table);
    let Some(header_at) = rows.iter().position(|row| {
        row.first()
            .map(|c| c.to_ascii_lowercase().contains("date"))
            .unwrap_or(false)
    }) else {
        return Vec::new();
    };
    let header: Vec<String> = rows[header_at]
        .iter()
        .map(|c| c.to_ascii_lowercase())
        .collect();
    let total_at = header.iter().position(|h| h.contains("total"));
    let mut out = Vec::new();
    for row in rows.iter().skip(header_at + 1) {
        let Some(t) = row.first().and_then(|c| parse_farside_date(c)) else {
            continue;
        };
        let total = total_at
            .and_then(|i| row.get(i))
            .and_then(|c| parse_optional_number(c));
        out.push((t, total));
    }
    out.sort_by_key(|row| row.0);
    out.dedup_by_key(|row| row.0);
    out
}

pub(crate) fn summarise_flows(days: &[(i64, Option<f64>)]) -> (Option<f64>, f64, f64, f64) {
    if days.is_empty() {
        return (None, 0.0, 0.0, 0.0);
    }
    let cum = days.iter().filter_map(|d| d.1).sum::<f64>();
    let latest = days[days.len() - 1];
    let avg = |cutoff: i64| {
        let rows: Vec<f64> = days
            .iter()
            .filter_map(|d| (d.0 > cutoff).then_some(d.1).flatten())
            .collect();
        if rows.is_empty() {
            0.0
        } else {
            rows.iter().sum::<f64>() / rows.len() as f64
        }
    };
    (
        latest.1,
        avg(latest.0 - 7 * SECONDS_PER_DAY),
        avg(latest.0 - 30 * SECONDS_PER_DAY),
        cum,
    )
}

fn html_rows(table: &str) -> Vec<Vec<String>> {
    let mut rows = Vec::new();
    let mut rest = table;
    while let Some(start) = rest.to_ascii_lowercase().find("<tr") {
        rest = &rest[start..];
        let Some(end) = rest.to_ascii_lowercase().find("</tr>") else {
            break;
        };
        let row = &rest[..end];
        rest = &rest[end + 5..];
        let mut cells = Vec::new();
        let mut cell_rest = row;
        loop {
            let lower = cell_rest.to_ascii_lowercase();
            let td = lower.find("<td");
            let th = lower.find("<th");
            let Some(at) = (match (td, th) {
                (Some(a), Some(b)) => Some(a.min(b)),
                (Some(a), None) => Some(a),
                (None, Some(b)) => Some(b),
                (None, None) => None,
            }) else {
                break;
            };
            cell_rest = &cell_rest[at..];
            let Some(gt) = cell_rest.find('>') else { break };
            cell_rest = &cell_rest[gt + 1..];
            let lower = cell_rest.to_ascii_lowercase();
            let close_td = lower.find("</td>");
            let close_th = lower.find("</th>");
            let Some(close) = (match (close_td, close_th) {
                (Some(a), Some(b)) => Some(a.min(b)),
                (Some(a), None) => Some(a),
                (None, Some(b)) => Some(b),
                (None, None) => None,
            }) else {
                break;
            };
            cells.push(strip_tags(&cell_rest[..close]));
            cell_rest = &cell_rest[close..];
        }
        if !cells.is_empty() {
            rows.push(cells);
        }
    }
    rows
}

fn strip_tags(value: &str) -> String {
    let value = value
        .replace("&nbsp;", " ")
        .replace("&#160;", " ")
        .replace("&amp;", "&");
    let mut out = String::new();
    let mut in_tag = false;
    for ch in value.chars() {
        if ch == '<' {
            in_tag = true;
            out.push(' ');
            continue;
        }
        if ch == '>' {
            in_tag = false;
            continue;
        }
        if !in_tag {
            out.push(ch);
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn parse_farside_date(value: &str) -> Option<i64> {
    let mut parts = value.split_whitespace();
    let day: u32 = parts.next()?.parse().ok()?;
    let month = match parts.next()?.to_ascii_lowercase().get(..3)? {
        "jan" => 1,
        "feb" => 2,
        "mar" => 3,
        "apr" => 4,
        "may" => 5,
        "jun" => 6,
        "jul" => 7,
        "aug" => 8,
        "sep" => 9,
        "oct" => 10,
        "nov" => 11,
        "dec" => 12,
        _ => return None,
    };
    let year: i32 = parts.next()?.parse().ok()?;
    let date = chrono::NaiveDate::from_ymd_opt(year, month, day)?;
    Some(date.and_hms_opt(0, 0, 0)?.and_utc().timestamp())
}

fn parse_optional_number(raw: &str) -> Option<f64> {
    let text = raw.trim();
    if text.is_empty() || text == "-" || text == "—" {
        return None;
    }
    let neg = text.starts_with('(') && text.ends_with(')');
    let cleaned: String = text
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == '.' || *c == '-')
        .collect();
    let value = cleaned.parse::<f64>().ok()?;
    if !value.is_finite() {
        return None;
    }
    Some(if neg { -value.abs() } else { value })
}

fn parse_zero_number(raw: &str) -> f64 {
    parse_optional_number(raw).unwrap_or(0.0)
}

fn parse_atm(doc: &str) -> Vec<AtmRow> {
    let text = strip_tags(doc);
    let re = Regex::new(
        r"(?i)\b(STRC|STRF|STRK|STRD|MSTR)\s+(?:Stock|Common\s+Stock)\s+(\$?\s*(?:[\d,]+(?:\.\d+)?|-|—))\s+\$?\s*(\$?\s*(?:[\d,]+(?:\.\d+)?|-|—))\s+\$?\s*(\$?\s*(?:[\d,]+(?:\.\d+)?|-|—))\s+\$?\s*(\$?\s*(?:[\d,]+(?:\.\d+)?|-|—))",
    )
    .expect("atm pattern");
    let mut seen = HashMap::<String, bool>::new();
    let mut rows = Vec::new();
    for cap in re.captures_iter(&text) {
        let ticker = cap[1].to_ascii_uppercase();
        if seen.insert(ticker.clone(), true).is_some() {
            continue;
        }
        rows.push(AtmRow {
            ticker,
            net_m: parse_zero_number(&cap[4]),
            avail_m: parse_zero_number(&cap[5]),
        });
    }
    rows
}

fn apply_strategy(out: &mut Terminal, filings: &[Filing], strc: &[Point], btc: &[Point]) {
    if let Some(last) = strc.last() {
        out.strc_price = Some(last.v);
        if strc.len() >= 2 && strc[strc.len() - 2].v > 0.0 {
            let prev = strc[strc.len() - 2].v;
            out.strc_change = Some((last.v - prev) / prev * 100.0);
        }
    }
    if filings.is_empty() {
        return;
    }
    let mut filings: Vec<&Filing> = filings.iter().collect();
    filings.sort_by_key(|f| f.filed);
    let Some(latest) = filings.last() else { return };
    out.strc_filed = Some(latest.filed);
    if let Some(strc_row) = latest.rows.iter().find(|r| r.ticker == "STRC") {
        out.strc_week_m = Some(strc_row.net_m);
        out.strc_left_m = Some(strc_row.avail_m);
        let ref_date = latest.period.unwrap_or(latest.filed);
        if let Some(avg) = avg_between(btc, ref_date - 7 * SECONDS_PER_DAY, ref_date) {
            if strc_row.net_m > 0.0 && avg > 0.0 {
                out.strc_btc = Some(strc_row.net_m * 1_000_000.0 / avg);
            }
        }
    }
    out.atm_cap_b = Some(latest.rows.iter().map(|r| r.avail_m).sum::<f64>() / 1000.0);
    // Only the most recent 8-Ks are read, so this is a recent-filings total, not a YTD.
    let history: Vec<Point> = filings
        .iter()
        .filter_map(|filing| {
            let row = filing.rows.iter().find(|r| r.ticker == "STRC")?;
            Some(Point {
                t: filing.period.unwrap_or(filing.filed),
                v: row.net_m,
            })
        })
        .collect();
    out.strc_recent_m = Some(history.iter().map(|p| p.v).sum());
    out.strc_filings = history.len() as u32;
    out.strc_hist = history;
}

fn avg_between(series: &[Point], start: i64, end: i64) -> Option<f64> {
    let rows: Vec<f64> = series
        .iter()
        .filter(|p| p.t > start && p.t <= end)
        .map(|p| p.v)
        .collect();
    if rows.is_empty() {
        None
    } else {
        Some(rows.iter().sum::<f64>() / rows.len() as f64)
    }
}

pub(crate) fn mei_latest(
    closes: &[Point],
    volume: &[Point],
) -> Option<(String, Option<f64>, Option<f64>, Option<f64>)> {
    if closes.len() < 800 {
        return None;
    }
    let mut vol = HashMap::new();
    for point in volume {
        vol.insert(point.t.div_euclid(SECONDS_PER_DAY), point.v);
    }
    let n = closes.len();
    let px: Vec<f64> = closes.iter().map(|p| p.v).collect();
    let volumes: Vec<Option<f64>> = closes
        .iter()
        .map(|p| vol.get(&p.t.div_euclid(SECONDS_PER_DAY)).copied())
        .collect();
    let trend_sma = rolling_mean_f64(&px, 200);
    let trend: Vec<Option<f64>> = px
        .iter()
        .zip(trend_sma.iter())
        .map(|(c, s)| s.and_then(|s| (s != 0.0).then_some(c / s - 1.0)))
        .collect();
    let momentum: Vec<Option<f64>> = px
        .iter()
        .enumerate()
        .map(|(i, c)| {
            if i < 60 || px[i - 60] == 0.0 {
                None
            } else {
                Some(c / px[i - 60] - 1.0)
            }
        })
        .collect();
    let cummax = rolling_max(&px, 504);
    let drawdown: Vec<Option<f64>> = px
        .iter()
        .zip(cummax.iter())
        .map(|(c, m)| (*m > 0.0).then_some(c / m - 1.0))
        .collect();
    let returns: Vec<Option<f64>> = px
        .iter()
        .enumerate()
        .map(|(i, c)| {
            if i == 0 || px[i - 1] == 0.0 {
                None
            } else {
                Some(c / px[i - 1] - 1.0)
            }
        })
        .collect();
    let rvol: Vec<Option<f64>> = rolling_std(&returns, 20)
        .into_iter()
        .map(|v| v.map(|v| v * 252_f64.sqrt()))
        .collect();
    let volume_sma = rolling_mean_opt(&volumes, 20);
    let volume_ratio: Vec<Option<f64>> = volumes
        .iter()
        .zip(volume_sma.iter())
        .map(|(v, s)| match (v, s) {
            (Some(v), Some(s)) if *s != 0.0 => Some(v / s),
            _ => None,
        })
        .collect();
    let abs_ret: Vec<Option<f64>> = returns.iter().map(|v| v.map(f64::abs)).collect();
    let range = rolling_mean_opt(&abs_ret, 20);
    let trend_p = rolling_pct(&trend, 504);
    let mom_p = rolling_pct(&momentum, 504);
    let dd_p = rolling_pct(&drawdown, 504);
    let rvol_p = rolling_pct(&rvol, 504);
    let vol_p = rolling_pct(&volume_ratio, 504);
    let range_p = rolling_pct(&range, 504);
    let mut valence_raw = vec![None; n];
    let mut arousal_raw = vec![None; n];
    for i in 0..n {
        if let (Some(t), Some(m), Some(d)) = (trend_p[i], mom_p[i], dd_p[i]) {
            valence_raw[i] = Some((0.35 * t + 0.30 * m + 0.20 * d) / 0.85);
        }
        if let (Some(rv), Some(rg)) = (rvol_p[i], range_p[i]) {
            if let Some(vo) = vol_p[i] {
                arousal_raw[i] = Some((0.4 * rv + 0.3 * vo + 0.3 * rg) / 1.0);
            } else {
                arousal_raw[i] = Some((0.4 * rv + 0.3 * rg) / 0.7);
            }
        }
    }
    let valence = ewm(&valence_raw, 3);
    let arousal = ewm(&arousal_raw, 3);
    let v = valence[n - 1]?;
    let a = arousal[n - 1]?;
    let vb = if v < 40.0 {
        0
    } else if v > 60.0 {
        2
    } else {
        1
    };
    let ab = if a < 35.0 {
        0
    } else if a > 65.0 {
        2
    } else {
        1
    };
    let state = match (vb, ab) {
        (0, 2) => "Panic",
        (1, 2) => "Volatile",
        (2, 2) => "Euphoria",
        (0, 1) => "Anxiety",
        (1, 1) => "Neutral",
        (2, 1) => "Optimism",
        (0, 0) => "Despondency",
        (1, 0) => "Apathy",
        _ => "Complacency",
    };
    let intensity = ((v - 50.0).abs().max((a - 50.0).abs()) / 50.0).clamp(0.0, 1.0);
    Some((state.into(), Some(v), Some(a), Some(intensity)))
}

fn rolling_mean_f64(xs: &[f64], window: usize) -> Vec<Option<f64>> {
    let wrapped: Vec<Option<f64>> = xs.iter().copied().map(Some).collect();
    rolling_mean_opt(&wrapped, window)
}

fn rolling_mean_opt(xs: &[Option<f64>], window: usize) -> Vec<Option<f64>> {
    let mut out = vec![None; xs.len()];
    let mut sum = 0.0;
    let mut count = 0i32;
    for i in 0..xs.len() {
        if let Some(v) = xs[i].filter(|v| v.is_finite()) {
            sum += v;
            count += 1;
        }
        if i >= window {
            if let Some(old) = xs[i - window].filter(|v| v.is_finite()) {
                sum -= old;
                count -= 1;
            }
        }
        if i + 1 >= window && count == window as i32 {
            out[i] = Some(sum / window as f64);
        }
    }
    out
}

fn rolling_std(xs: &[Option<f64>], window: usize) -> Vec<Option<f64>> {
    let mut out = vec![None; xs.len()];
    if window < 2 {
        return out;
    }
    for i in window - 1..xs.len() {
        let mut sum = 0.0;
        let mut ok = true;
        for j in i + 1 - window..=i {
            match xs[j] {
                Some(v) if v.is_finite() => sum += v,
                _ => {
                    ok = false;
                    break;
                }
            }
        }
        if !ok {
            continue;
        }
        let mean = sum / window as f64;
        let mut sq = 0.0;
        for j in i + 1 - window..=i {
            let d = xs[j].unwrap_or(0.0) - mean;
            sq += d * d;
        }
        out[i] = Some((sq / (window as f64 - 1.0)).sqrt());
    }
    out
}

fn rolling_max(xs: &[f64], window: usize) -> Vec<f64> {
    let mut out = vec![f64::NAN; xs.len()];
    for i in 0..xs.len() {
        let start = i.saturating_sub(window - 1);
        let mut m = f64::NEG_INFINITY;
        for value in &xs[start..=i] {
            if *value > m {
                m = *value;
            }
        }
        out[i] = m;
    }
    out
}

pub(crate) fn rolling_pct(xs: &[Option<f64>], window: usize) -> Vec<Option<f64>> {
    let mut out = vec![None; xs.len()];
    if window == 0 {
        return out;
    }
    for i in window - 1..xs.len() {
        let mut buf = Vec::with_capacity(window);
        for j in i + 1 - window..=i {
            match xs[j] {
                Some(v) if v.is_finite() => buf.push(v),
                _ => {
                    buf.clear();
                    break;
                }
            }
        }
        if buf.len() != window {
            continue;
        }
        let target = *buf.last().unwrap_or(&0.0);
        let mut less = 0usize;
        let mut equal = 0usize;
        for value in &buf {
            if *value < target {
                less += 1;
            } else if *value == target {
                equal += 1;
            }
        }
        let avg_rank = less as f64 + (equal as f64 + 1.0) / 2.0;
        out[i] = Some(avg_rank / window as f64 * 100.0);
    }
    out
}

fn ewm(xs: &[Option<f64>], span: usize) -> Vec<Option<f64>> {
    if span <= 1 {
        return xs.to_vec();
    }
    let alpha = 2.0 / (span as f64 + 1.0);
    let mut out = vec![None; xs.len()];
    let mut prev: Option<f64> = None;
    for (i, value) in xs.iter().enumerate() {
        match value.filter(|v| v.is_finite()) {
            Some(v) => {
                prev = Some(match prev {
                    Some(p) => p + alpha * (v - p),
                    None => v,
                });
                out[i] = prev;
            }
            None => prev = None,
        }
    }
    out
}

fn get(client: &Client, url: &str) -> Result<String, String> {
    let response = client.get(url).send().map_err(|err| err.to_string())?;
    let status = response.status();
    let body = response.text().map_err(|err| err.to_string())?;
    if !status.is_success() {
        return Err(format!("HTTP {status}"));
    }
    Ok(body)
}

fn get_headed(client: &Client, url: &str, accept: &str, ua: &str) -> Result<String, String> {
    let response = client
        .get(url)
        .header("Accept", accept)
        .header("User-Agent", ua)
        .send()
        .map_err(|err| err.to_string())?;
    let status = response.status();
    let body = response.text().map_err(|err| err.to_string())?;
    if !status.is_success() {
        return Err(format!("HTTP {status}"));
    }
    Ok(body)
}

fn fred_points(client: &Client, series: &str, start: &str) -> Option<Vec<Point>> {
    let body = get(
        client,
        &format!("https://fred.stlouisfed.org/graph/fredgraph.csv?id={series}&cosd={start}"),
    )
    .ok()?;
    let mut points = Vec::new();
    for obs in parse::fred_series(&body) {
        let Some(t) = date_unix(&obs.date) else {
            continue;
        };
        points.push(Point { t, v: obs.value });
    }
    (!points.is_empty()).then_some(points)
}

fn date_unix(date: &str) -> Option<i64> {
    let day = date.get(0..10)?;
    let date = chrono::NaiveDate::parse_from_str(day, "%Y-%m-%d").ok()?;
    Some(date.and_hms_opt(0, 0, 0)?.and_utc().timestamp())
}

fn json_f64(value: &Value) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value.as_i64().map(|v| v as f64))
        .or_else(|| value.as_str().and_then(|s| s.parse().ok()))
        .filter(|v| v.is_finite())
}

/// Coin Metrics community daily reference price (PriceUSD) for `asset` from `start` (YYYY-MM-DD).
pub fn coinmetrics_prices(client: &Client, asset: &str, start: &str) -> Option<Vec<Point>> {
    let mut url = format!("https://community-api.coinmetrics.io/v4/timeseries/asset-metrics?assets={asset}&metrics=PriceUSD&frequency=1d&start_time={start}&page_size=10000");
    let mut out = Vec::new();
    for _ in 0..4 {
        let body = get(client, &url).ok()?;
        let json: Value = serde_json::from_str(&body).ok()?;
        let rows = json["data"].as_array()?;
        for row in rows {
            let Some(t) = row["time"].as_str().and_then(date_unix) else {
                continue;
            };
            let Some(v) = json_f64(&row["PriceUSD"]) else {
                continue;
            };
            if v > 0.0 {
                out.push(Point { t, v });
            }
        }
        match json["next_page_url"].as_str() {
            Some(next) if !next.is_empty() => url = next.to_string(),
            _ => break,
        }
    }
    out.sort_by_key(|p| p.t);
    (!out.is_empty()).then_some(out)
}

fn bitcoin_data(client: &Client, path: &str, key: &str) -> Option<Vec<Point>> {
    let body = get(client, &format!("https://bitcoin-data.com/api/v1/{path}")).ok()?;
    let json: Value = serde_json::from_str(&body).ok()?;
    let rows = json.as_array().or_else(|| json["data"].as_array())?;
    let mut out = Vec::new();
    for row in rows {
        let t = json_f64(&row["unixTs"])
            .map(|v| {
                if v > 10_000_000_000.0 {
                    (v / 1000.0) as i64
                } else {
                    v as i64
                }
            })
            .or_else(|| row["d"].as_str().and_then(date_unix));
        let Some(t) = t else { continue };
        let Some(v) = json_f64(&row[key]) else {
            continue;
        };
        out.push(Point { t, v });
    }
    out.sort_by_key(|p| p.t);
    (!out.is_empty()).then_some(out)
}

pub fn yahoo_daily(client: &Client, symbol: &str, range: &str) -> Option<Vec<Point>> {
    let symbol = symbol.replace('^', "%5E");
    let ua = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/121.0.0.0 Safari/537.36";
    for host in [
        "https://query1.finance.yahoo.com",
        "https://query2.finance.yahoo.com",
    ] {
        let url = format!("{host}/v8/finance/chart/{symbol}?range={range}&interval=1d");
        let Ok(body) = get_headed(client, &url, "application/json", ua) else {
            continue;
        };
        if let Some(points) = parse_yahoo(&body) {
            return Some(points);
        }
    }
    None
}

fn parse_yahoo(body: &str) -> Option<Vec<Point>> {
    let json: Value = serde_json::from_str(body).ok()?;
    if !json["chart"]["error"].is_null() && json["chart"]["error"].as_object().is_some() {
        return None;
    }
    let result = json["chart"]["result"].get(0)?;
    let stamps = result["timestamp"].as_array()?;
    let closes = result["indicators"]["quote"].get(0)?["close"].as_array()?;
    let mut out = Vec::new();
    for (stamp, close) in stamps.iter().zip(closes.iter()) {
        let Some(t) = stamp.as_i64() else { continue };
        let Some(v) = json_f64(close) else { continue };
        if v > 0.0 {
            out.push(Point { t, v });
        }
    }
    (!out.is_empty()).then_some(out)
}

fn gold_series(client: &Client) -> Option<Vec<Point>> {
    let url = "https://api.coingecko.com/api/v3/coins/pax-gold/market_chart?vs_currency=usd&days=365&interval=daily";
    if let Ok(body) = get(client, url) {
        if let Some(points) = parse_coingecko(&body) {
            if points.len() >= 91 {
                return Some(points);
            }
        }
    }
    yahoo_daily(client, "GLD", "1y")
}

fn parse_coingecko(body: &str) -> Option<Vec<Point>> {
    let json: Value = serde_json::from_str(body).ok()?;
    let rows = json["prices"].as_array()?;
    let mut out = Vec::new();
    for row in rows {
        let Some(pair) = row.as_array() else { continue };
        let Some(t) = pair
            .first()
            .and_then(|v| v.as_i64().or_else(|| v.as_f64().map(|n| n as i64)))
        else {
            continue;
        };
        let Some(v) = pair.get(1).and_then(json_f64) else {
            continue;
        };
        if v > 0.0 {
            out.push(Point { t: t / 1000, v });
        }
    }
    (!out.is_empty()).then_some(out)
}

fn coinbase_hourly(client: &Client) -> Option<Vec<Point>> {
    let body = get_headed(
        client,
        "https://api.exchange.coinbase.com/products/BTC-USD/candles?granularity=3600",
        "application/json",
        "dollar-dashboard/0.1 (local Market Monitor)",
    )
    .ok()?;
    let json: Value = serde_json::from_str(&body).ok()?;
    let mut out = Vec::new();
    for row in json.as_array()? {
        let Some(cells) = row.as_array() else {
            continue;
        };
        let Some(t) = cells.first().and_then(json_f64) else {
            continue;
        };
        let Some(close) = cells.get(4).and_then(json_f64) else {
            continue;
        };
        if close > 0.0 {
            out.push(Point {
                t: t as i64,
                v: close,
            });
        }
    }
    out.sort_by_key(|p| p.t);
    (!out.is_empty()).then_some(out)
}

fn binance_funding(client: &Client) -> Option<Vec<f64>> {
    let body = get(
        client,
        "https://fapi.binance.com/fapi/v1/fundingRate?symbol=BTCUSDT&limit=21",
    )
    .ok()?;
    let json: Value = serde_json::from_str(&body).ok()?;
    let mut rows = Vec::new();
    for row in json.as_array()? {
        let Some(rate) = json_f64(&row["fundingRate"]) else {
            continue;
        };
        let t = json_f64(&row["fundingTime"]).unwrap_or(0.0) as i64;
        rows.push((if t > 10_000_000_000 { t / 1000 } else { t }, rate));
    }
    rows.sort_by_key(|row| row.0);
    let rates: Vec<f64> = rows.into_iter().map(|row| row.1).collect();
    (!rates.is_empty()).then_some(rates)
}

fn bybit_funding(client: &Client) -> Option<Vec<f64>> {
    let body = get(
        client,
        "https://api.bybit.com/v5/market/funding/history?category=linear&symbol=BTCUSDT&limit=21",
    )
    .ok()?;
    let json: Value = serde_json::from_str(&body).ok()?;
    let mut rows = Vec::new();
    for row in json["result"]["list"].as_array()? {
        let t = json_f64(&row["fundingRateTimestamp"]).unwrap_or(0.0) as i64;
        let Some(rate) = json_f64(&row["fundingRate"]) else {
            continue;
        };
        rows.push((if t > 10_000_000_000 { t / 1000 } else { t }, rate));
    }
    rows.sort_by_key(|row| row.0);
    let rates: Vec<f64> = rows.into_iter().map(|row| row.1).collect();
    (!rates.is_empty()).then_some(rates)
}

fn okx_funding(client: &Client) -> Option<Vec<f64>> {
    let body = get(
        client,
        "https://www.okx.com/api/v5/public/funding-rate-history?instId=BTC-USDT-SWAP&limit=21",
    )
    .ok()?;
    let json: Value = serde_json::from_str(&body).ok()?;
    let mut rows = Vec::new();
    for row in json["data"].as_array()? {
        let t = json_f64(&row["fundingTime"]).unwrap_or(0.0) as i64;
        let Some(rate) = json_f64(&row["fundingRate"]) else {
            continue;
        };
        rows.push((if t > 10_000_000_000 { t / 1000 } else { t }, rate));
    }
    rows.sort_by_key(|row| row.0);
    let rates: Vec<f64> = rows.into_iter().map(|row| row.1).collect();
    (!rates.is_empty()).then_some(rates)
}

fn kraken_volume(client: &Client) -> Option<Vec<Point>> {
    let body = get(
        client,
        "https://api.kraken.com/0/public/OHLC?pair=XBTUSDT&interval=1440",
    )
    .ok()?;
    let json: Value = serde_json::from_str(&body).ok()?;
    if json["error"]
        .as_array()
        .map(|e| !e.is_empty())
        .unwrap_or(false)
    {
        return None;
    }
    let result = json["result"].as_object()?;
    let rows = result.values().find_map(Value::as_array)?;
    let mut out = Vec::new();
    for row in rows {
        let Some(cells) = row.as_array() else {
            continue;
        };
        let Some(t) = cells.first().and_then(json_f64) else {
            continue;
        };
        let Some(vwap) = cells.get(5).and_then(json_f64) else {
            continue;
        };
        let Some(volume) = cells.get(6).and_then(json_f64) else {
            continue;
        };
        let quote = vwap * volume;
        if quote.is_finite() && quote > 0.0 {
            out.push(Point {
                t: t as i64,
                v: quote,
            });
        }
    }
    (!out.is_empty()).then_some(out)
}

fn edgar_filings(client: &Client) -> Option<Vec<Filing>> {
    let ua = "dollar-dashboard/0.1 johanneshankoe@gmail.com (local Market Monitor)";
    let atom = get_headed(
        client,
        "https://www.sec.gov/cgi-bin/browse-edgar?action=getcompany&CIK=0001050446&type=8-K&dateb=&owner=include&count=20&output=atom",
        "application/atom+xml, text/html, */*",
        ua,
    )
    .ok()?;
    let mut filings = Vec::new();
    for entry in atom.split("<entry>").skip(1).take(8) {
        let entry = entry.split("</entry>").next().unwrap_or("");
        let href = between(entry, "href=\"", "\"");
        let updated = between(entry, "<updated>", "</updated>");
        let has_accession = entry.contains("accession-number=");
        let (Some(href), Some(updated)) = (href, updated) else {
            continue;
        };
        if !has_accession {
            continue;
        };
        let Ok(filed_at) = chrono::DateTime::parse_from_rfc3339(updated) else {
            continue;
        };
        thread::sleep(Duration::from_millis(150));
        let Ok(index) = get_headed(client, href, "text/html", ua) else {
            continue;
        };
        let Some(path) = mstr_doc_path(&index) else {
            continue;
        };
        let doc_url = format!("https://www.sec.gov{path}");
        let period = period_end(&path);
        thread::sleep(Duration::from_millis(150));
        let Ok(doc) = get_headed(client, &doc_url, "text/html", ua) else {
            continue;
        };
        let rows = parse_atm(&doc);
        if rows.is_empty() {
            continue;
        }
        filings.push(Filing {
            filed: filed_at.timestamp(),
            period,
            rows,
        });
    }
    Some(filings)
}

fn between<'a>(text: &'a str, start: &str, end: &str) -> Option<&'a str> {
    let i = text.find(start)? + start.len();
    let rest = &text[i..];
    let j = rest.find(end)?;
    Some(&rest[..j])
}

fn mstr_doc_path(html: &str) -> Option<String> {
    let re = Regex::new(r#"(/Archives/edgar/data/[^"'?\s]+?mstr-\d{8}\.htm)"#).ok()?;
    re.captures(html).map(|cap| cap[1].to_string())
}

fn period_end(path: &str) -> Option<i64> {
    let re = Regex::new(r"mstr-(\d{4})(\d{2})(\d{2})\.htm").ok()?;
    let cap = re.captures(path)?;
    let year: i32 = cap[1].parse().ok()?;
    let month: u32 = cap[2].parse().ok()?;
    let day: u32 = cap[3].parse().ok()?;
    let date = chrono::NaiveDate::from_ymd_opt(year, month, day)?;
    Some(date.and_hms_opt(0, 0, 0)?.and_utc().timestamp())
}

/// Scored rosters (the Artificial Analysis index, directly or via mirrors) come
/// first; OpenRouter token prices without scores are the last resort.
fn frontier_models(client: &Client) -> Option<(Vec<ModelPrint>, &'static str)> {
    frontier_from_leaderboard(client)
        .or_else(|| frontier_from_pages(client))
        .or_else(|| frontier_from_benchmarks(client))
        .map(|models| (models, "aa"))
        .or_else(|| {
            frontier_pricing(&openrouter_prices(client)).map(|models| (models, "openrouter"))
        })
}

/// Score keys filled from evaluators other than Artificial Analysis.
pub const OUTSIDE_SCORES: &[&str] = &["epoch-eci", "epoch-eci-lo", "epoch-eci-hi", "vals-index"];

/// The roster plus scores from independent evaluators, fetched in parallel.
/// Returns the feed ids of the outside evaluators that answered.
fn frontier_with_outside(client: &Client) -> Option<(Vec<ModelPrint>, &'static str, Vec<&'static str>)> {
    let (roster, eci, vals) = thread::scope(|s| {
        let eci = s.spawn(|| epoch_eci(client));
        let vals = s.spawn(|| vals_index(client));
        let roster = frontier_models(client);
        (roster, eci.join().ok().flatten(), vals.join().ok().flatten())
    });
    let (mut models, source) = roster?;
    let answered = attach_outside(&mut models, eci.as_ref(), vals.as_ref());
    Some((models, source, answered))
}

/// Match outside scores to roster rows by normalised model name.
pub(crate) fn attach_outside(
    models: &mut [ModelPrint],
    eci: Option<&HashMap<String, (f64, f64, f64)>>,
    vals: Option<&HashMap<String, f64>>,
) -> Vec<&'static str> {
    for m in models.iter_mut() {
        let key = model_key(&m.label);
        if let Some(&(v, lo, hi)) = eci.and_then(|e| e.get(&key)) {
            m.scores.insert("epoch-eci".into(), v);
            m.scores.insert("epoch-eci-lo".into(), lo);
            m.scores.insert("epoch-eci-hi".into(), hi);
        }
        if let Some(&v) = vals.and_then(|e| e.get(&key)) {
            m.scores.insert("vals-index".into(), v);
        }
    }
    let mut answered = Vec::new();
    if eci.is_some_and(|e| !e.is_empty()) {
        answered.push("epoch");
    }
    if vals.is_some_and(|e| !e.is_empty()) {
        answered.push("vals");
    }
    answered
}

/// "Claude Opus 5.5 (Max)", "anthropic/claude-opus-5-5" and "Claude Opus 5.5" all
/// become "claudeopus55": drop the effort suffix and the provider, keep letters and digits.
pub(crate) fn model_key(name: &str) -> String {
    let name = name.split(" (").next().unwrap_or(name);
    let name = name.rsplit('/').next().unwrap_or(name);
    name.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

/// Epoch AI Capabilities Index: (score, 90% CI low, CI high) per model.
fn epoch_eci(client: &Client) -> Option<HashMap<String, (f64, f64, f64)>> {
    let body = get(client, "https://epoch.ai/data/eci_scores.csv").ok()?;
    let rows = parse_eci_csv(&body);
    (!rows.is_empty()).then_some(rows)
}

pub(crate) fn parse_eci_csv(body: &str) -> HashMap<String, (f64, f64, f64)> {
    let mut lines = body.lines();
    let Some(header) = lines.next() else {
        return HashMap::new();
    };
    let cols: Vec<&str> = header.split(',').map(str::trim).collect();
    let at = |name: &str| cols.iter().position(|c| *c == name);
    let (Some(model), Some(eci), Some(lo), Some(hi)) = (at("Model"), at("eci"), at("eci_ci_low"), at("eci_ci_high")) else {
        return HashMap::new();
    };
    let mut out = HashMap::new();
    for line in lines {
        // Model names carry no commas; later quoted fields are never read.
        let f: Vec<&str> = line.split(',').collect();
        let num = |i: usize| f.get(i).and_then(|v| v.trim().parse::<f64>().ok());
        if let (Some(name), Some(v), Some(l), Some(h)) = (f.get(model), num(eci), num(lo), num(hi)) {
            out.entry(model_key(name)).or_insert((v, l, h));
        }
    }
    out
}

/// Vals.ai Vals Index: overall accuracy (0–1) per model, from the page's Astro props.
fn vals_index(client: &Client) -> Option<HashMap<String, f64>> {
    let html = get(client, "https://www.vals.ai/benchmarks/vals_index").ok()?;
    let rows = parse_vals_index(&html);
    (!rows.is_empty()).then_some(rows)
}

pub(crate) fn parse_vals_index(html: &str) -> HashMap<String, f64> {
    let text = html.replace("&quot;", "\"");
    let mut out = HashMap::new();
    let Some(start) = text.find(r#""overall":[0,{"#) else {
        return out;
    };
    // The overall map ends where the next per-task map begins.
    let rest = &text[start + 14..];
    let end = rest.find(r#"}]],"#).map(|i| i + 3).unwrap_or(rest.len());
    let Ok(row) = Regex::new(r#""[A-Za-z0-9_.-]+/([A-Za-z0-9_.-]+)":\[0,\{"accuracy":\[0,([0-9.]+)\]"#) else {
        return out;
    };
    for cap in row.captures_iter(&rest[..end]) {
        if let Ok(v) = cap[2].parse::<f64>() {
            out.entry(model_key(&cap[1])).or_insert(v / 100.0);
        }
    }
    out
}

fn frontier_from_leaderboard(client: &Client) -> Option<Vec<ModelPrint>> {
    let html = get(client, "https://artificialanalysis.ai/leaderboards/models").ok()?;
    let models = frontier_from_leaderboard_html(&html, 10);
    (!models.is_empty()).then_some(models)
}

/// The Artificial Analysis leaderboard streams its table as Next.js flight
/// chunks; the model rows sit in one `{"models":[...]}` object inside them.
pub(crate) fn frontier_from_leaderboard_html(html: &str, limit: usize) -> Vec<ModelPrint> {
    let Ok(chunk) = Regex::new(r#"self\.__next_f\.push\(\[1,("(?:[^"\\]|\\.)*")\]\)"#) else {
        return Vec::new();
    };
    let flight: String = chunk
        .captures_iter(html)
        .filter_map(|cap| serde_json::from_str::<String>(&cap[1]).ok())
        .collect();
    // The row list has shipped as `models` and, since October 2026, `initialModels`.
    let Some((start, key)) = ["initialModels", "models"].into_iter().find_map(|key| {
        flight
            .find(&format!(r#"{{"{key}":[{{"#))
            .map(|start| (start, key))
    }) else {
        return Vec::new();
    };
    let mut stream = serde_json::Deserializer::from_str(&flight[start..]).into_iter::<Value>();
    let Some(Ok(json)) = stream.next() else {
        return Vec::new();
    };
    let Some(rows) = json[key].as_array() else {
        return Vec::new();
    };
    // One row per base model: keep its best-scoring effort variant.
    let mut best: HashMap<String, (String, ModelPrint)> = HashMap::new();
    for row in rows {
        if row["deprecated"].as_bool().unwrap_or(false) {
            continue;
        }
        let (Some(slug), Some(name), Some(intel)) = (
            row["slug"].as_str(),
            row["name"].as_str(),
            json_f64(&row["intelligenceIndex"]),
        ) else {
            continue;
        };
        let input = json_f64(&row["price1mInputTokens"]).filter(|v| *v > 0.0);
        let output = json_f64(&row["price1mOutputTokens"]).filter(|v| *v > 0.0);
        let (Some(input), Some(output)) = (input, output) else {
            continue;
        };
        let model = ModelPrint {
            label: name
                .replace(", Default Fallback", "")
                .replace(" (Default Fallback)", ""),
            creator: row["modelCreatorName"].as_str().unwrap_or("").to_string(),
            intelligence: Some(intel),
            blended: (input * 3.0 + output) / 4.0,
            input,
            output,
            scores: row["scores"]
                .as_object()
                .map(|m| {
                    m.iter()
                        .filter_map(|(k, v)| Some((k.clone(), json_f64(v)?)))
                        .collect()
                })
                .unwrap_or_default(),
        };
        let base = base_slug(slug);
        if best
            .get(&base)
            .is_none_or(|(_, held)| model.intelligence > held.intelligence)
        {
            best.insert(base.clone(), (base, model));
        }
    }
    let mut ranked: Vec<ModelPrint> = best.into_values().map(|(_, model)| model).collect();
    ranked.sort_by(|a, b| {
        b.intelligence
            .unwrap_or(0.0)
            .total_cmp(&a.intelligence.unwrap_or(0.0))
    });
    let mut per_creator: HashMap<String, usize> = HashMap::new();
    ranked
        .into_iter()
        .filter(|model| {
            let seen = per_creator.entry(model.creator.clone()).or_default();
            *seen += 1;
            *seen <= 3
        })
        .take(limit)
        .collect()
}

fn base_slug(slug: &str) -> String {
    const EFFORTS: &[&str] = &[
        "-non-reasoning",
        "-reasoning",
        "-minimal",
        "-medium",
        "-xhigh",
        "-high",
        "-low",
        "-max",
    ];
    let mut base = slug.to_ascii_lowercase();
    while let Some(cut) = EFFORTS.iter().find_map(|s| base.strip_suffix(s)) {
        base = cut.to_string();
    }
    base
}

fn frontier_pricing(prices: &HashMap<String, (f64, f64)>) -> Option<Vec<ModelPrint>> {
    let rows: Vec<_> = MODELS
        .iter()
        .filter_map(|(label, _, id)| {
            let (input, output) = *prices.get((*id)?)?;
            Some(ModelPrint {
                label: (*label).into(),
                creator: String::new(),
                intelligence: None,
                blended: (input * 3.0 + output) / 4.0,
                input,
                output,
                scores: Default::default(),
            })
        })
        .collect();
    (!rows.is_empty()).then_some(rows)
}

fn frontier_from_pages(client: &Client) -> Option<Vec<ModelPrint>> {
    let prices = openrouter_prices(client);
    let mut models = Vec::new();
    for (label, path, id) in MODELS {
        let url = format!("https://www.aiapicost.com{path}");
        let Ok(html) = get_headed(
            client,
            &url,
            "text/html",
            "dollar-dashboard/0.1 frontier-model-monitor",
        ) else {
            continue;
        };
        let Some(intel) = first_number(
            &html,
            &[
                r"Intelligence Index:\s*([0-9.]+)",
                r"Intelligence Index</a><span[^>]*>([0-9.]+)</span>",
            ],
        ) else {
            continue;
        };
        let page_in = first_number(
            &html,
            &[r"Input:\s*\$([0-9.]+)/M tokens", r":\s*\$([0-9.]+)/M input"],
        );
        let page_out = first_number(
            &html,
            &[
                r"Output:\s*\$([0-9.]+)/M tokens",
                r"\$[0-9.]+/M input,\s*\$([0-9.]+)/M output",
            ],
        );
        let router = id.and_then(|id| prices.get(id).copied());
        let input = page_in.or(router.map(|p| p.0));
        let output = page_out.or(router.map(|p| p.1));
        let (Some(input), Some(output)) = (input, output) else {
            continue;
        };
        models.push(ModelPrint {
            label: (*label).into(),
            creator: String::new(),
            intelligence: Some(intel),
            blended: (input * 3.0 + output) / 4.0,
            input,
            output,
            scores: Default::default(),
        });
    }
    models.sort_by(|a, b| {
        b.intelligence
            .unwrap_or(0.0)
            .total_cmp(&a.intelligence.unwrap_or(0.0))
    });
    (!models.is_empty()).then_some(models)
}

pub(crate) fn frontier_from_benchmark_body(body: &str) -> Vec<ModelPrint> {
    let Ok(json) = serde_json::from_str::<Value>(body) else {
        return Vec::new();
    };
    let Some(rows) = json["models"].as_array() else {
        return Vec::new();
    };
    let mut parsed = Vec::new();
    for row in rows {
        let Some(label) = row["name"].as_str().filter(|s| !s.is_empty()) else {
            continue;
        };
        let slug = row["slug"].as_str().unwrap_or("").to_ascii_lowercase();
        let Some(intel) = json_f64(&row["evals"]["artificial_analysis_intelligence_index"]) else {
            continue;
        };
        let pricing = &row["pricing"];
        let input = json_f64(&pricing["price_1m_input_tokens"]);
        let output = json_f64(&pricing["price_1m_output_tokens"]);
        let blended =
            json_f64(&pricing["price_1m_blended_3_to_1"]).or_else(|| match (input, output) {
                (Some(input), Some(output)) => Some((input * 3.0 + output) / 4.0),
                _ => None,
            });
        let (Some(input), Some(output), Some(blended)) = (input, output, blended) else {
            continue;
        };
        parsed.push((
            slug,
            ModelPrint {
                label: label.to_string(),
                creator: String::new(),
                intelligence: Some(intel),
                blended,
                input,
                output,
                scores: Default::default(),
            },
        ));
    }
    let needles = [
        "claude-opus-5-5",
        "claude-sonnet-5-5",
        "claude-fable-5-1",
        "gpt-6-astra",
        "gpt-6-1-sol",
        "gemini-4",
        "grok-4-7",
        "muse-spark",
        "qwen3-8",
        "deepseek-v4",
    ];
    let mut picked = Vec::new();
    let mut used = std::collections::HashSet::new();
    for needle in needles {
        let needle_dot = needle.replace('-', ".");
        let mut best: Option<usize> = None;
        for (index, (slug, model)) in parsed.iter().enumerate() {
            if used.contains(&index) {
                continue;
            }
            let dotted = slug.replace('-', ".");
            if !slug.contains(needle) && !dotted.contains(&needle_dot) {
                continue;
            }
            let better = best
                .map(|i| model.intelligence > parsed[i].1.intelligence)
                .unwrap_or(true);
            if better {
                best = Some(index);
            }
        }
        if let Some(index) = best {
            used.insert(index);
            picked.push(parsed[index].1.clone());
        }
    }
    if picked.len() < 3 {
        let mut ranked = parsed;
        ranked.sort_by(|a, b| {
            b.1.intelligence
                .unwrap_or(0.0)
                .total_cmp(&a.1.intelligence.unwrap_or(0.0))
        });
        picked = ranked.into_iter().take(9).map(|(_, model)| model).collect();
    }
    picked.sort_by(|a, b| {
        b.intelligence
            .unwrap_or(0.0)
            .total_cmp(&a.intelligence.unwrap_or(0.0))
    });
    picked
}

fn frontier_from_benchmarks(client: &Client) -> Option<Vec<ModelPrint>> {
    let body = get(
        client,
        "https://benchmarks-proxy.leontloveless.workers.dev/",
    )
    .ok()?;
    let models = frontier_from_benchmark_body(&body);
    (!models.is_empty()).then_some(models)
}

fn openrouter_prices(client: &Client) -> HashMap<String, (f64, f64)> {
    let Ok(body) = get(client, "https://openrouter.ai/api/v1/models") else {
        return HashMap::new();
    };
    let Ok(json) = serde_json::from_str::<Value>(&body) else {
        return HashMap::new();
    };
    let Some(rows) = json["data"].as_array() else {
        return HashMap::new();
    };
    let mut out = HashMap::new();
    for row in rows {
        let Some(id) = row["id"].as_str() else {
            continue;
        };
        let Some(input) = row["pricing"]["prompt"].as_str().and_then(per_million) else {
            continue;
        };
        let Some(output) = row["pricing"]["completion"].as_str().and_then(per_million) else {
            continue;
        };
        out.insert(id.to_string(), (input, output));
    }
    out
}

fn per_million(raw: &str) -> Option<f64> {
    let value: f64 = raw.parse().ok()?;
    (value.is_finite() && value > 0.0).then_some(value * 1_000_000.0)
}

pub(crate) fn first_number(text: &str, patterns: &[&str]) -> Option<f64> {
    for pattern in patterns {
        let re = Regex::new(pattern).ok()?;
        if let Some(cap) = re.captures(text) {
            if let Ok(value) = cap[1].parse::<f64>() {
                if value.is_finite() {
                    return Some(value);
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regime_score_needs_every_input() {
        let now = 1_794_000_000;
        let series = |value: f64, days: i64| -> Vec<Point> {
            (0..days)
                .map(|i| Point {
                    t: now - (days - i) * SECONDS_PER_DAY,
                    v: value,
                })
                .collect()
        };
        let fed = series(4.0, 400);
        let m2 = series(22_000.0, 500);
        let mvrv = series(1.5, 400);
        let daily = series(100_000.0, 1_600);
        let dxy = series(120.0, 400);
        let full = regime_book(&fed, &m2, &mvrv, &daily, &dxy, now).unwrap();
        assert_eq!(full.2.len(), 5);
        // Losing two feeds used to lower the score by their votes and still report it.
        let missing = regime_book(&fed, &[], &[], &daily, &dxy, now).unwrap_err();
        assert_eq!(missing, ["M2 YoY", "MVRV Z"]);
        assert!(regime_book(&fed, &m2, &mvrv, &daily, &[], now).is_err());
    }

    #[test]
    fn odd_farside_date_cells_are_skipped_without_panicking() {
        assert!(parse_farside_date("11 Nov 2026").is_some());
        for cell in ["12 - 14", "3 é 2026", "7 x", "Total", ""] {
            assert_eq!(parse_farside_date(cell), None, "{cell}");
        }
    }

    #[test]
    fn model_keys_match_across_evaluators() {
        assert_eq!(model_key("Claude Opus 5.5 (Max)"), "claudeopus55");
        assert_eq!(model_key("anthropic/claude-opus-5-5"), "claudeopus55");
        assert_eq!(model_key("GPT-6.1 Sol"), model_key("openai/gpt-6.1-sol"));
        assert_ne!(model_key("Muse Wonder (Max)"), model_key("meta/muse-wonder-full"));
    }

    #[test]
    fn outside_scores_attach_by_name() {
        let eci = parse_eci_csv(
            "Model,Display name,eci,eci_ci_low,eci_ci_high,date\nClaude Opus 5.5,Claude Opus 5.5,168.4,165.37,173.03,2026-09-22\nClaude Opus 5.5,Claude Opus 5.5,1,1,1,2026-09-22\n",
        );
        let vals = parse_vals_index(
            r#"props="{&quot;tasks&quot;:[0,{&quot;overall&quot;:[0,{&quot;anthropic/claude-opus-5-5&quot;:[0,{&quot;accuracy&quot;:[0,70.177],&quot;latency&quot;:[0,1]}],&quot;openai/gpt-6-astra&quot;:[0,{&quot;accuracy&quot;:[0,65.023]}]}]],&quot;finance&quot;:[0,{&quot;openai/gpt-6-astra&quot;:[0,{&quot;accuracy&quot;:[0,10.0]}]}]}]}""#,
        );
        let near = |a: Option<f64>, b: f64| a.is_some_and(|a| (a - b).abs() < 1e-9);
        assert!(near(vals.get("gpt6astra").copied(), 0.65023));
        let mut models = vec![
            ModelPrint { label: "Claude Opus 5.5 (Max)".into(), ..Default::default() },
            ModelPrint { label: "Gemini 4 Carbon (High)".into(), ..Default::default() },
        ];
        let answered = attach_outside(&mut models, Some(&eci), Some(&vals));
        assert_eq!(answered, vec!["epoch", "vals"]);
        assert_eq!(models[0].score("epoch-eci"), Some(168.4));
        assert_eq!(models[0].score("epoch-eci-hi"), Some(173.03));
        assert!(near(models[0].score("vals-index"), 0.70177));
        assert_eq!(models[1].score("epoch-eci"), None);
    }

    #[test]
    fn pricing_fallback_preserves_missing_benchmark_scores() {
        let prices = HashMap::from([("openai/gpt-6-astra".to_string(), (2.0, 8.0))]);
        let models = frontier_pricing(&prices).unwrap();
        assert_eq!(models[0].intelligence, None);
        assert_eq!(models[0].blended, 3.5);
        assert!(frontier_pricing(&HashMap::new()).is_none());
    }

    fn pt(t: i64, v: f64) -> Point {
        Point { t, v }
    }

    #[test]
    fn net_liquidity_forward_fills_and_converts_to_billions() {
        let walcl = vec![pt(0, 1_000.0)];
        let tga = vec![pt(0, 100.0), pt(2 * SECONDS_PER_DAY, 200.0)];
        let rrp = vec![pt(0, 0.05)];
        let series = net_liquidity(&walcl, &tga, &rrp);
        assert_eq!(series.len(), 3);
        assert!((series[0].v - 0.85).abs() < 1e-9);
        assert!((series[1].v - 0.85).abs() < 1e-9);
        assert!((series[2].v - 0.75).abs() < 1e-9);
    }

    #[test]
    fn net_liquidity_matches_shared_source_contract() {
        #[derive(serde::Deserialize)]
        struct Case {
            name: String,
            walcl: Vec<Point>,
            tga: Vec<Point>,
            rrp: Vec<Point>,
            expected: Vec<Point>,
        }
        #[derive(serde::Deserialize)]
        struct Contract {
            cases: Vec<Case>,
        }
        let contract: Contract =
            serde_json::from_str(include_str!("../../testdata/net-liquidity.json")).unwrap();
        for case in contract.cases {
            assert_eq!(
                net_liquidity(&case.walcl, &case.tga, &case.rrp),
                case.expected,
                "{}",
                case.name
            );
        }
    }

    #[test]
    fn liquidity_failure_keeps_the_last_complete_corrected_calculation() {
        let mut prev = Terminal::default();
        prev.net_liq_version = NET_LIQ_VERSION;
        prev.net_liq_b = Some(5_500.0);
        prev.net_liq_30d_b = Some(-50.0);
        prev.net_liq_hist = vec![pt(0, 5_550.0), pt(30 * SECONDS_PER_DAY, 5_500.0)];
        let partial = net_liquidity(&[pt(0, 8_000_000.0)], &[], &[pt(0, 2_000.0)]);
        assert!(partial.is_empty());
        let merged = merge(&prev, Terminal::default());
        assert_eq!(merged.net_liq_version, NET_LIQ_VERSION);
        assert_eq!(merged.net_liq_b, prev.net_liq_b);
        assert_eq!(merged.net_liq_30d_b, prev.net_liq_30d_b);
        assert_eq!(merged.net_liq_hist, prev.net_liq_hist);
    }

    #[test]
    fn legacy_liquidity_is_cleared_without_discarding_other_cached_readings() {
        let mut legacy = Terminal::default();
        legacy.net_liq_b = Some(7_498.0);
        legacy.net_liq_30d_b = Some(10.0);
        legacy.net_liq_hist = vec![pt(0, 7_498.0)];
        legacy.mvrv_z = Some(1.5);
        assert!(invalidate_legacy_liquidity(&mut legacy));
        assert_eq!(legacy.net_liq_b, None);
        assert_eq!(legacy.net_liq_30d_b, None);
        assert!(legacy.net_liq_hist.is_empty());
        assert_eq!(legacy.mvrv_z, Some(1.5));
        // The merge also guards callers that supplied an unmigrated snapshot.
        legacy.net_liq_b = Some(7_498.0);
        assert_eq!(merge(&legacy, Terminal::default()).net_liq_b, None);
    }

    #[test]
    fn pending_etf_day_keeps_its_date_and_missing_total_together() {
        let mut prev = Terminal::default();
        prev.etf_latest_unix = Some(0);
        prev.etf_latest_m = Some(100.0);
        prev.etf_cum_m = Some(500.0);
        let mut fresh = Terminal::default();
        fresh.etf_latest_unix = Some(SECONDS_PER_DAY);
        fresh.etf_cum_m = Some(500.0);
        let merged = merge(&prev, fresh);
        assert_eq!(merged.etf_latest_unix, Some(SECONDS_PER_DAY));
        assert_eq!(merged.etf_latest_m, None);
        // A completely failed refresh must preserve the original pair.
        let failed = merge(&prev, Terminal::default());
        assert_eq!(failed.etf_latest_unix, prev.etf_latest_unix);
        assert_eq!(failed.etf_latest_m, prev.etf_latest_m);
        let mut reported_zero = Terminal::default();
        reported_zero.etf_latest_unix = Some(SECONDS_PER_DAY);
        reported_zero.etf_latest_m = Some(0.0);
        let zero = merge(&prev, reported_zero);
        assert_eq!(zero.etf_latest_unix, Some(SECONDS_PER_DAY));
        assert_eq!(zero.etf_latest_m, Some(0.0));
    }

    #[test]
    fn macro_delta_uses_the_print_thirty_days_back() {
        let series = vec![
            pt(0, 10.0),
            pt(20 * SECONDS_PER_DAY, 12.0),
            pt(40 * SECONDS_PER_DAY, 15.0),
        ];
        let row = macro_print("HY Spread", "%", &series, "FRED");
        assert_eq!(row.current, Some(15.0));
        assert_eq!(row.delta_30d, Some(5.0));
        assert_eq!(row.prev, Some(12.0));
        assert_eq!(row.hist, vec![10.0, 12.0, 15.0]);
        assert_eq!(row.as_of, "1970-02-10");
    }

    #[test]
    fn identical_series_correlate_at_one() {
        let btc: Vec<Point> = (0..120)
            .map(|i| pt(i * SECONDS_PER_DAY, 100.0 + i as f64))
            .collect();
        let row = corr_print("S&P 500", &btc, &btc);
        assert!((row.r30.unwrap() - 1.0).abs() < 1e-9);
        assert!((row.r90.unwrap() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn fed_cut_scores_plus_one() {
        let now = 400 * SECONDS_PER_DAY;
        let fed = vec![pt(0, 5.0), pt(now, 4.0)];
        assert_eq!(score_fed(&fed, now), 1);
        assert!(describe_fed(Some(4.0), Some(5.0)).contains("falling"));
    }

    #[test]
    fn farside_reads_total_and_parentheses() {
        let html = r#"<table class="etf"><tr><th>Date</th><th>IBIT</th><th>Total</th></tr>
            <tr><td>12 May 2026</td><td>10</td><td>20.5</td></tr>
            <tr><td>13 May 2026</td><td>(5)</td><td>(1.5)</td></tr>
            <tr><td>Week</td><td>5</td><td>19</td></tr></table>"#;
        let days = parse_farside(html);
        assert_eq!(days.len(), 2);
        assert_eq!(days[1].1, Some(-1.5));
        let summary = summarise_flows(&days);
        assert_eq!(summary.0, Some(-1.5));
        assert!((summary.3 - 19.0).abs() < 1e-9);
    }

    #[test]
    fn atm_table_keeps_the_first_row_per_ticker() {
        let doc = "STRC Stock 1,000 $ 100.0 $ 90.5 $ 400 STRC Stock 9 $ 9 $ 9 $ 9";
        let rows = parse_atm(doc);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].ticker, "STRC");
        assert!((rows[0].net_m - 90.5).abs() < 1e-9);
        assert!((rows[0].avail_m - 400.0).abs() < 1e-9);
    }

    #[test]
    fn benchmark_feed_keeps_named_frontier_models() {
        let body = r#"{"models":[
            {"name":"Grok 4","slug":"grok-4","evals":{"artificial_analysis_intelligence_index":48},"pricing":{"price_1m_input_tokens":1.25,"price_1m_output_tokens":2.5,"price_1m_blended_3_to_1":1.56}},
            {"name":"Other","slug":"tiny","evals":{"artificial_analysis_intelligence_index":10},"pricing":{"price_1m_input_tokens":0.1,"price_1m_output_tokens":0.2,"price_1m_blended_3_to_1":0.12}},
            {"name":"GPT-5.5","slug":"gpt-5.5","evals":{"artificial_analysis_intelligence_index":60},"pricing":{"price_1m_input_tokens":5,"price_1m_output_tokens":15}}
        ]}"#;
        let models = frontier_from_benchmark_body(body);
        assert!(models
            .iter()
            .any(|m| m.label == "Grok 4" && (m.blended - 1.56).abs() < 1e-9));
        assert!(models
            .iter()
            .any(|m| m.label == "GPT-5.5" && (m.blended - 7.5).abs() < 1e-9));
        assert!(models.iter().any(|m| m.label == "Other"));
    }

    #[test]
    fn leaderboard_flight_keeps_best_variant_per_model() {
        let rows = serde_json::json!({"models": [
            {"slug":"claude-opus-5-5","name":"Claude Opus 5.5 (Max, Default Fallback)","modelCreatorName":"Anthropic","deprecated":false,"intelligenceIndex":57.6,"price1mInputTokens":4,"price1mOutputTokens":20},
            {"slug":"claude-opus-5-5-high","name":"Claude Opus 5.5 (High, Default Fallback)","modelCreatorName":"Anthropic","deprecated":false,"intelligenceIndex":53.6,"price1mInputTokens":4,"price1mOutputTokens":20},
            {"slug":"gpt-6-astra-xhigh","name":"GPT-6 Astra (Xhigh)","modelCreatorName":"OpenAI","deprecated":false,"intelligenceIndex":52.4,"price1mInputTokens":10,"price1mOutputTokens":50},
            {"slug":"old-model","name":"Old","modelCreatorName":"X","deprecated":true,"intelligenceIndex":70,"price1mInputTokens":1,"price1mOutputTokens":1},
            {"slug":"free-model","name":"Free","modelCreatorName":"Y","deprecated":false,"intelligenceIndex":40,"price1mInputTokens":0,"price1mOutputTokens":0}
        ]});
        let payload = format!("3a:[\"$\",\"div\",null,{rows}]");
        let (a, b) = payload.split_at(payload.len() / 2);
        let html = format!(
            "<script>self.__next_f.push([1,{}])</script><script>self.__next_f.push([1,{}])</script>",
            serde_json::to_string(a).unwrap(),
            serde_json::to_string(b).unwrap()
        );
        let models = frontier_from_leaderboard_html(&html, 10);
        let renamed = frontier_from_leaderboard_html(&html.replace("\\\"models\\\"", "\\\"initialModels\\\""), 10);
        assert_eq!(renamed, models);
        assert_eq!(models.len(), 2);
        assert_eq!(models[0].label, "Claude Opus 5.5 (Max)");
        assert_eq!(models[0].creator, "Anthropic");
        assert_eq!(models[0].intelligence, Some(57.6));
        assert!((models[0].blended - 8.0).abs() < 1e-9);
        assert_eq!(models[1].label, "GPT-6 Astra (Xhigh)");
        assert_eq!(base_slug("gpt-6-1-sol-xhigh"), "gpt-6-1-sol");
    }

    #[test]
    fn pricing_only_refresh_keeps_scored_models() {
        let mut prev = Terminal::default();
        prev.models = vec![ModelPrint {
            label: "A".into(),
            intelligence: Some(50.0),
            ..Default::default()
        }];
        let mut fresh = Terminal::default();
        fresh.models = vec![ModelPrint {
            label: "B".into(),
            ..Default::default()
        }];
        assert_eq!(merge(&prev, fresh).models[0].label, "A");
    }

    #[test]
    fn pricing_fallback_updates_prices_and_outside_scores_on_the_scored_roster() {
        let old = ModelPrint {
            label: "GPT-6.1 Sol (Max)".into(),
            creator: "OpenAI".into(),
            intelligence: Some(50.0),
            input: 10.0,
            output: 40.0,
            blended: 17.5,
            scores: std::collections::BTreeMap::from([
                ("epoch-eci".into(), 150.0),
                ("vals-index".into(), 0.6),
                ("hle-diamond".into(), 0.55),
            ]),
        };
        let price = ModelPrint {
            label: "GPT-6.1 Sol".into(),
            input: 1.0,
            output: 4.0,
            blended: 1.75,
            scores: std::collections::BTreeMap::from([("epoch-eci".into(), 160.0)]),
            ..Default::default()
        };
        let other = ModelPrint {
            label: "New scored model".into(),
            intelligence: Some(60.0),
            ..Default::default()
        };
        let prev = Terminal {
            models: vec![old, other.clone()],
            ..Default::default()
        };
        let fresh = Terminal {
            models: vec![price],
            ..Default::default()
        };
        let models = merge(&prev, fresh).models;
        assert_eq!(models.len(), 2);
        assert_eq!(models[0].label, "GPT-6.1 Sol (Max)");
        assert_eq!(models[0].creator, "OpenAI");
        assert_eq!(models[0].intelligence, Some(50.0));
        assert_eq!(
            (models[0].input, models[0].output, models[0].blended),
            (1.0, 4.0, 1.75)
        );
        assert_eq!(models[0].score("epoch-eci"), Some(160.0));
        assert_eq!(models[0].score("vals-index"), Some(0.6));
        assert_eq!(models[0].score("hle-diamond"), Some(0.55));
        assert_eq!(models[1], other);
    }

    #[test]
    fn scored_refresh_replaces_the_roster_and_preserves_only_missing_score_fields() {
        let old = ModelPrint {
            label: "A (Max)".into(),
            intelligence: Some(50.0),
            scores: std::collections::BTreeMap::from([
                ("epoch-eci".into(), 150.0),
                ("vals-index".into(), 0.6),
                ("hle-diamond".into(), 0.7),
            ]),
            ..Default::default()
        };
        let fresh = ModelPrint {
            label: "A (High)".into(),
            intelligence: Some(55.0),
            scores: std::collections::BTreeMap::from([("epoch-eci".into(), 160.0)]),
            ..Default::default()
        };
        let models = merge_models(
            &[
                old,
                ModelPrint {
                    label: "Retired".into(),
                    ..Default::default()
                },
            ],
            vec![fresh],
        );
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].label, "A (High)");
        assert_eq!(models[0].intelligence, Some(55.0));
        assert_eq!(models[0].score("epoch-eci"), Some(160.0));
        assert_eq!(models[0].score("vals-index"), Some(0.6));
        assert_eq!(models[0].score("hle-diamond"), None);
    }

    #[test]
    fn frontier_index_and_token_prices_parse() {
        let html = "Intelligence Index: 52.4 Input: $1.25/M tokens Output: $5.00/M tokens";
        assert_eq!(
            first_number(html, &[r"Intelligence Index:\s*([0-9.]+)"]),
            Some(52.4)
        );
        assert_eq!(
            first_number(html, &[r"Input:\s*\$([0-9.]+)/M tokens"]),
            Some(1.25)
        );
    }

    #[test]
    fn rolling_percentile_uses_average_rank() {
        let xs = vec![Some(1.0), Some(2.0), Some(2.0), Some(2.0), Some(5.0)];
        let pct = rolling_pct(&xs, 5);
        assert!((pct[4].unwrap() - 100.0).abs() < 1e-9);
    }

    #[test]
    fn premium_is_coinbase_minus_binance_in_bps() {
        let hour = 1_700_000_000;
        let cb = vec![pt(hour, 101.0)];
        let bn = vec![pt(hour, 100.0)];
        let (cur, avg, history) = coinbase_premium(&cb, &bn);
        assert!((cur.unwrap() - 100.0).abs() < 1e-6);
        assert!((avg.unwrap() - 100.0).abs() < 1e-6);
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].t, hour);
    }

    #[test]
    fn merge_keeps_a_previous_reading_when_the_refresh_misses_it() {
        let mut prev = Terminal::default();
        prev.mvrv_z = Some(1.5);
        prev.venues = vec![VenuePrint {
            name: "Bybit".into(),
            current: Some(0.0001),
            avg_7d: Some(0.0001),
        }];
        let mut fresh = Terminal::default();
        fresh.nupl = Some(0.4);
        fresh.venues = vec![VenuePrint {
            name: "Binance".into(),
            current: Some(0.0002),
            avg_7d: Some(0.0002),
        }];
        let merged = merge(&prev, fresh);
        assert_eq!(merged.mvrv_z, Some(1.5));
        assert_eq!(merged.nupl, Some(0.4));
        assert_eq!(merged.venues.len(), 2);
        assert_eq!(merged.venues[1].name, "Bybit");
    }

    #[test]
    fn week_buckets_share_a_monday_start() {
        let monday = chrono::NaiveDate::from_ymd_opt(2024, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc()
            .timestamp();
        let sunday = monday + 6 * SECONDS_PER_DAY;
        let next = monday + 7 * SECONDS_PER_DAY;
        assert_eq!(week_key(monday), week_key(sunday));
        assert_ne!(week_key(monday), week_key(next));
    }
}
