//! The overview's cross-asset board: one row per instrument with the session
//! move, a 30-day change, the 52-week range and a 90-day trend.

use crate::model::{
    change_since, delta_since, fmt_grouped, fmt_usd, quote, Delta, Point, Snapshot, SECONDS_PER_DAY,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Link {
    Bitcoin,
    Dollar,
    Nuclear,
    Zcash,
    SpaceX,
    None,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Usd,
    Level2,
    Fx4,
    Yield,
    Spread,
}

#[derive(Clone, Debug)]
pub struct Row {
    pub group: &'static str,
    pub key: &'static str,
    pub label: &'static str,
    pub last: Option<f64>,
    pub text: String,
    pub day: Option<Delta>,
    pub month: Option<Delta>,
    /// 52-week low and high of dated observations, including a dated spot print.
    pub range: Option<(f64, f64)>,
    pub spark: Vec<f64>,
    pub source: String,
    pub as_of: String,
    /// Known price observation time; receipt time is separate for delayed feeds.
    pub observed_unix: Option<i64>,
    pub acquired_unix: Option<i64>,
    pub link: Link,
}

pub fn rows(snap: &Snapshot, now: i64) -> Vec<Row> {
    let mut out = Vec::new();
    let btc_hist = with_observation(
        &snap.btc.daily,
        snap.btc.price,
        snap.price_observed_unix("BTC"),
    );
    out.push(row(
        "Crypto",
        "BTC",
        "Bitcoin",
        Kind::Usd,
        snap.btc.price,
        snap.btc.change_24h_pct.map(Delta::Pct),
        &btc_hist,
        &snap
            .price_stamp("BTC")
            .map(|s| s.source.clone())
            .unwrap_or_else(|| snap.btc.price_source.clone()),
        &snap.price_as_of("BTC", now),
        Link::Bitcoin,
    ));
    let eth_hist = with_observation(
        &snap.btc.eth_daily,
        snap.btc.eth_price,
        snap.price_observed_unix("ETH"),
    );
    out.push(row(
        "Crypto",
        "ETH",
        "Ether",
        Kind::Usd,
        snap.btc.eth_price,
        snap.btc.eth_change_24h_pct.map(Delta::Pct),
        &eth_hist,
        "Binance spot · 24h",
        &snap.price_as_of("ETH", now),
        Link::Bitcoin,
    ));
    let zec_hist = with_observation(
        &snap.zec.daily,
        snap.btc.zec_price,
        snap.price_observed_unix("ZEC"),
    );
    out.push(row(
        "Crypto",
        "ZEC",
        "Zcash",
        Kind::Usd,
        snap.btc.zec_price,
        snap.btc.zec_change_24h_pct.map(Delta::Pct),
        &zec_hist,
        "Binance spot · 24h",
        &snap.price_as_of("ZEC", now),
        Link::Zcash,
    ));
    for (group, key, label, kind, link) in [
        ("Equities", "SPX", "S&P 500", Kind::Level2, Link::Dollar),
        ("Equities", "NDX", "Nasdaq 100", Kind::Level2, Link::None),
        ("Equities", "SPCX", "SpaceX", Kind::Usd, Link::SpaceX),
        ("Rates", "US02Y", "US 2Y", Kind::Yield, Link::Dollar),
        ("Rates", "US10Y", "US 10Y", Kind::Yield, Link::Dollar),
        ("Rates", "US30Y", "US 30Y", Kind::Yield, Link::Dollar),
    ] {
        out.push(market_row(snap, now, group, key, label, kind, link));
    }
    out.push(spread_row(snap, now));
    for (group, key, label, kind, link) in [
        ("Rates", "JP10Y", "Japan 10Y", Kind::Yield, Link::Dollar),
        (
            "Dollar & FX",
            "DXY",
            "US dollar index",
            Kind::Level2,
            Link::Dollar,
        ),
        ("Dollar & FX", "EURUSD", "EUR/USD", Kind::Fx4, Link::None),
        (
            "Dollar & FX",
            "USDJPY",
            "USD/JPY",
            Kind::Level2,
            Link::Dollar,
        ),
        (
            "Volatility & credit",
            "VIX",
            "VIX",
            Kind::Level2,
            Link::Dollar,
        ),
        (
            "Volatility & credit",
            "MOVE",
            "MOVE",
            Kind::Level2,
            Link::Dollar,
        ),
    ] {
        out.push(market_row(snap, now, group, key, label, kind, link));
    }
    if let Some(hy) = snap
        .terminal
        .macro_strip
        .iter()
        .find(|m| m.label == "HY Spread" && m.current.is_some())
    {
        let last = hy.current;
        out.push(Row {
            group: "Volatility & credit",
            key: "HY",
            label: "HY OAS",
            last,
            text: last.map(|v| format!("{v:.2}%")).unwrap_or_else(dash),
            day: last.zip(hy.prev).map(|(a, b)| Delta::Pp(a - b)),
            month: hy.delta_30d.map(Delta::Pp),
            range: None,
            spark: hy.hist[hy.hist.len().saturating_sub(90)..].to_vec(),
            source: "FRED BAMLH0A0HYM2".into(),
            as_of: short_date(&hy.as_of),
            observed_unix: date_unix(&hy.as_of),
            acquired_unix: None,
            link: Link::Dollar,
        });
    }
    for (key, label) in [
        ("GOLD", "Gold"),
        ("SILVER", "Silver"),
        ("BRENT", "Brent"),
        ("WTI", "WTI crude"),
    ] {
        out.push(market_row(
            snap,
            now,
            "Commodities",
            key,
            label,
            Kind::Usd,
            if key == "GOLD" || key == "BRENT" {
                Link::Dollar
            } else {
                Link::None
            },
        ));
    }
    // UxC futures are the live price; the IMF monthly index only lends its trend
    // shape, since it runs well below spot and lags by about three months.
    let uranium = &snap.terminal.uranium_hist;
    match quote(&snap.quotes, "URANIUM") {
        Some(q) => out.push(Row {
            group: "Commodities",
            key: "U3O8",
            label: "Uranium U3O8",
            last: Some(q.price),
            text: format!("${:.2}", q.price),
            day: Some(Delta::Pct(q.change_pct)),
            month: None,
            range: None,
            spark: tail(uranium, 36),
            source: "UxC front-month futures (TradingView) · trend: IMF monthly index".into(),
            as_of: snap.price_as_of("URANIUM", now),
            observed_unix: snap.price_observed_unix("URANIUM"),
            acquired_unix: snap.price_acquired_unix("URANIUM"),
            link: Link::Nuclear,
        }),
        None => {
            if let Some(price) = snap.terminal.uranium {
                out.push(Row {
                    group: "Commodities",
                    key: "U3O8",
                    label: "Uranium (IMF monthly)",
                    last: Some(price),
                    text: format!("${price:.2}"),
                    day: None,
                    month: None,
                    range: None,
                    spark: tail(uranium, 36),
                    source: "FRED PURANUSDM · monthly average, lags spot".into(),
                    as_of: short_date(&snap.terminal.uranium_as_of),
                    observed_unix: date_unix(&snap.terminal.uranium_as_of),
                    acquired_unix: None,
                    link: Link::Nuclear,
                });
            }
        }
    }
    for row in &mut out {
        let key = if row.key == "U3O8" {
            "URANIUM"
        } else {
            row.key
        };
        if let Some(stamp) = snap.price_stamp(key) {
            row.observed_unix = stamp.observed_unix;
            row.acquired_unix = Some(stamp.acquired_unix);
        }
    }
    out
}

#[allow(clippy::too_many_arguments)]
fn row(
    group: &'static str,
    key: &'static str,
    label: &'static str,
    kind: Kind,
    last: Option<f64>,
    day: Option<Delta>,
    hist: &[Point],
    source: &str,
    as_of: &str,
    link: Link,
) -> Row {
    let last = last.or_else(|| hist.last().map(|p| p.v));
    let end = hist.last().map(|p| p.t).unwrap_or(0);
    Row {
        group,
        key,
        label,
        last,
        text: last.map(|v| format_value(kind, v)).unwrap_or_else(dash),
        day,
        month: month_change(kind, hist),
        range: range(hist, None, end),
        spark: tail(hist, 90),
        source: source.into(),
        as_of: as_of.into(),
        observed_unix: None,
        acquired_unix: None,
        link,
    }
}

fn market_row(
    snap: &Snapshot,
    now: i64,
    group: &'static str,
    key: &'static str,
    label: &'static str,
    kind: Kind,
    link: Link,
) -> Row {
    let quote = quote(&snap.quotes, key);
    let history = snap.histories.get(key).map(Vec::as_slice).unwrap_or(&[]);
    let hist = with_observation(
        history,
        quote.map(|q| q.price),
        snap.price_observed_unix(key),
    );
    let day = match quote {
        Some(q) if matches!(kind, Kind::Yield | Kind::Spread) => q.change_abs.map(Delta::Pp),
        Some(q) => Some(Delta::Pct(q.change_pct)),
        None => last_move(kind, history),
    };
    let (source, as_of) = match (quote, history.last()) {
        (Some(_), _) => (
            "TradingView · delayed; receipt time shown".to_string(),
            snap.price_as_of(key, now),
        ),
        (None, Some(p)) => ("CNBC daily close".to_string(), day_label(p.t)),
        (None, None) => (String::new(), String::new()),
    };
    let mut out = row(
        group,
        key,
        label,
        kind,
        quote.map(|q| q.price),
        day,
        &hist,
        &source,
        &as_of,
        link,
    );
    if quote.is_some() && !history.is_empty() {
        out.source = "TradingView delayed quote (receipt age) · CNBC dated history".into();
    }
    out.observed_unix = if quote.is_some() {
        snap.price_observed_unix(key)
    } else {
        history.last().map(|p| p.t)
    };
    out.acquired_unix = snap.price_acquired_unix(key);
    out
}

/// US 2s10s from the two yields, in percentage points.
fn spread_row(snap: &Snapshot, now: i64) -> Row {
    let get = |key| snap.histories.get(key).map(Vec::as_slice).unwrap_or(&[]);
    let (two, ten) = (get("US02Y"), get("US10Y"));
    let mut hist: Vec<Point> = ten
        .iter()
        .filter_map(|p| {
            two.iter().find(|q| q.t == p.t).map(|q| Point {
                t: p.t,
                v: p.v - q.v,
            })
        })
        .collect();
    let q2 = quote(&snap.quotes, "US02Y");
    let q10 = quote(&snap.quotes, "US10Y");
    let live = q2.zip(q10).map(|(a, b)| b.price - a.price);
    let observed = snap
        .price_observed_unix("US02Y")
        .zip(snap.price_observed_unix("US10Y"))
        .map(|(a, b)| a.min(b));
    hist = with_observation(&hist, live, observed);
    let day = match (
        q2.and_then(|q| q.change_abs),
        q10.and_then(|q| q.change_abs),
    ) {
        (Some(a), Some(b)) => Some(Delta::Pp(b - a)),
        _ => last_move(Kind::Spread, &hist),
    };
    let mut out = row(
        "Rates",
        "2S10S",
        "2s10s curve",
        Kind::Spread,
        live,
        day,
        &hist,
        "10Y minus 2Y",
        if live.is_some() {
            "cached quote"
        } else {
            "close"
        },
        Link::Dollar,
    );
    out.range = None;
    out.observed_unix =
        observed.or_else(|| (live.is_none()).then(|| hist.last().map(|p| p.t)).flatten());
    out.acquired_unix = snap
        .price_acquired_unix("US02Y")
        .zip(snap.price_acquired_unix("US10Y"))
        .map(|(a, b)| a.min(b));
    if let Some(t) = out.acquired_unix {
        out.as_of = format!("recv {}", crate::model::fmt_age(t, now));
    }
    out
}

/// Compatibility helper for callers that already know this observation's time.
pub fn with_live(points: &[Point], live: Option<f64>, now: i64) -> Vec<Point> {
    with_observation(points, live, Some(now))
}

/// Extend history only at the actual observation day, never the time the chart
/// happens to be rendered. Undated legacy prices cannot create a new close.
pub fn with_observation(
    points: &[Point],
    value: Option<f64>,
    observed_unix: Option<i64>,
) -> Vec<Point> {
    let mut out = points.to_vec();
    if let (Some(v), Some(observed)) = (value.filter(|v| v.is_finite()), observed_unix) {
        let today = observed.div_euclid(SECONDS_PER_DAY) * SECONDS_PER_DAY;
        match out.last().map(|last| last.t) {
            Some(t) if t.div_euclid(SECONDS_PER_DAY) == today.div_euclid(SECONDS_PER_DAY) => {
                if let Some(last) = out.last_mut() {
                    last.v = v;
                }
            }
            Some(t) if t < today => out.push(Point { t: today, v }),
            Some(_) => {}
            None => {}
        }
    }
    out
}

fn date_unix(date: &str) -> Option<i64> {
    Some(
        chrono::NaiveDate::parse_from_str(date.get(..10)?, "%Y-%m-%d")
            .ok()?
            .and_hms_opt(0, 0, 0)?
            .and_utc()
            .timestamp(),
    )
}

fn month_change(kind: Kind, hist: &[Point]) -> Option<Delta> {
    if matches!(kind, Kind::Yield | Kind::Spread) {
        delta_since(hist, 30).map(Delta::Pp)
    } else {
        change_since(hist, 30).map(Delta::Pct)
    }
}

fn last_move(kind: Kind, hist: &[Point]) -> Option<Delta> {
    let [.., a, b] = hist else { return None };
    if matches!(kind, Kind::Yield | Kind::Spread) {
        Some(Delta::Pp(b.v - a.v))
    } else {
        (a.v > 0.0).then(|| Delta::Pct((b.v / a.v - 1.0) * 100.0))
    }
}

fn range(hist: &[Point], last: Option<f64>, end: i64) -> Option<(f64, f64)> {
    let start = end - 365 * SECONDS_PER_DAY;
    let mut values: Vec<f64> = hist
        .iter()
        .filter(|p| p.t > start && p.v.is_finite())
        .map(|p| p.v)
        .collect();
    values.extend(last);
    if values.len() < 20 {
        return None;
    }
    let lo = values.iter().copied().fold(f64::INFINITY, f64::min);
    let hi = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    (hi > lo).then_some((lo, hi))
}

fn tail(hist: &[Point], n: usize) -> Vec<f64> {
    hist[hist.len().saturating_sub(n)..]
        .iter()
        .map(|p| p.v)
        .collect()
}

fn format_value(kind: Kind, v: f64) -> String {
    match kind {
        Kind::Usd if v >= 1000.0 => fmt_usd(v),
        Kind::Usd => format!("${v:.2}"),
        Kind::Level2 => fmt_grouped(v, 2),
        Kind::Fx4 => format!("{v:.4}"),
        Kind::Yield => format!("{v:.3}%"),
        Kind::Spread => crate::model::with_minus(format!("{:+.1} bp", v * 100.0)),
    }
}

fn day_label(t: i64) -> String {
    chrono::DateTime::from_timestamp(t, 0)
        .map(|d| d.format("%-d %b").to_string())
        .unwrap_or_default()
}

fn short_date(date: &str) -> String {
    chrono::NaiveDate::parse_from_str(date.get(0..10).unwrap_or(""), "%Y-%m-%d")
        .map(|d| d.format("%-d %b").to_string())
        .unwrap_or_else(|_| date.to_string())
}

fn dash() -> String {
    "—".into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{fallback_snapshot, Quote};

    fn series(days: i64, start: f64, step: f64) -> Vec<Point> {
        (0..days)
            .map(|i| Point {
                t: i * SECONDS_PER_DAY,
                v: start + i as f64 * step,
            })
            .collect()
    }

    #[test]
    fn live_print_replaces_or_extends_the_last_close() {
        let points = series(3, 10.0, 1.0);
        let extended = with_live(&points, Some(20.0), 5 * SECONDS_PER_DAY + 60);
        assert_eq!(extended.len(), 4);
        assert_eq!(extended[3].v, 20.0);
        let replaced = with_live(&points, Some(20.0), 2 * SECONDS_PER_DAY + 60);
        assert_eq!(replaced.len(), 3);
        assert_eq!(replaced[2].v, 20.0);
        assert!(with_live(&[], Some(1.0), 0).is_empty());
    }

    #[test]
    fn legacy_prices_do_not_claim_live_or_add_an_undated_close() {
        let mut snap = fallback_snapshot();
        snap.btc.daily = series(40, 10.0, 1.0);
        let old_history = snap.btc.daily.clone();
        let rows = rows(&snap, 60 * SECONDS_PER_DAY);
        let btc = rows.iter().find(|r| r.key == "BTC").unwrap();
        assert_eq!(btc.as_of, "cached");
        assert!(btc.observed_unix.is_none());
        assert_eq!(btc.spark.len(), old_history.len());
        assert_eq!(btc.range, Some((10.0, 49.0)));
        assert_eq!(
            with_observation(&old_history, snap.btc.price, None),
            old_history
        );
    }

    #[test]
    fn out_of_window_retained_price_cannot_distort_the_52_week_range() {
        let mut snap = fallback_snapshot();
        snap.btc.daily = series(40, 10.0, 1.0)
            .into_iter()
            .map(|p| Point {
                t: p.t + 1000 * SECONDS_PER_DAY,
                v: p.v,
            })
            .collect();
        snap.btc.price = Some(1_000_000.0);
        snap.price_stamps.insert(
            "BTC".into(),
            crate::model::PriceStamp {
                acquired_unix: 500 * SECONDS_PER_DAY,
                observed_unix: Some(500 * SECONDS_PER_DAY),
                source: "old cached spot".into(),
            },
        );
        let rows = rows(&snap, 1040 * SECONDS_PER_DAY);
        let btc = rows.iter().find(|r| r.key == "BTC").unwrap();
        assert_eq!(btc.range, Some((10.0, 49.0)));
        assert_eq!(btc.spark.len(), 40);
    }

    #[test]
    fn dated_price_stays_on_its_observation_day_and_cannot_rewrite_a_later_close() {
        let points = series(3, 10.0, 1.0);
        let unchanged = with_observation(&points, Some(99.0), Some(SECONDS_PER_DAY + 60));
        assert_eq!(unchanged, points);
        let observed = with_observation(&points, Some(99.0), Some(4 * SECONDS_PER_DAY + 60));
        assert_eq!(observed.last().unwrap().t, 4 * SECONDS_PER_DAY);
    }

    #[test]
    fn delayed_quote_receipt_does_not_invent_a_trade_date() {
        let mut snap = fallback_snapshot();
        snap.histories.insert("SPX".into(), series(40, 100.0, 1.0));
        snap.quotes.push(Quote {
            symbol: "SPX".into(),
            price: 150.0,
            change_pct: 2.0,
            change_abs: None,
            market_cap: None,
        });
        snap.price_stamps.insert(
            "SPX".into(),
            crate::model::PriceStamp {
                acquired_unix: 60 * SECONDS_PER_DAY,
                observed_unix: None,
                source: "TradingView delayed".into(),
            },
        );
        let rows = rows(&snap, 60 * SECONDS_PER_DAY + 120);
        let spx = rows.iter().find(|r| r.key == "SPX").unwrap();
        assert!(spx.as_of.starts_with("recv "));
        assert_eq!(spx.acquired_unix, Some(60 * SECONDS_PER_DAY));
        assert!(spx.observed_unix.is_none());
        assert_eq!(spx.spark.len(), 40);
        assert_eq!(spx.spark.last(), Some(&139.0));
    }

    #[test]
    fn yields_move_in_basis_points_and_prices_in_percent() {
        let mut snap = fallback_snapshot();
        snap.histories.insert("US10Y".into(), series(60, 4.0, 0.01));
        snap.histories.insert("DXY".into(), series(60, 100.0, 0.1));
        snap.quotes.push(Quote {
            symbol: "US10Y".into(),
            price: 4.60,
            change_pct: 0.5,
            market_cap: None,
            change_abs: Some(0.02),
        });
        let now = 59 * SECONDS_PER_DAY + 100;
        snap.price_stamps.insert(
            "US10Y".into(),
            crate::model::PriceStamp {
                acquired_unix: now,
                observed_unix: Some(now),
                source: "dated test quote".into(),
            },
        );
        let rows = rows(&snap, now);
        let ten = rows.iter().find(|r| r.key == "US10Y").unwrap();
        assert_eq!(ten.day, Some(Delta::Pp(0.02)));
        assert_eq!(ten.text, "4.600%");
        // 30 days before the live print the close was 4.0 + 29 * 0.01.
        assert!((ten.month.unwrap().value() - (4.60 - 4.29)).abs() < 1e-9);
        let dxy = rows.iter().find(|r| r.key == "DXY").unwrap();
        let Some(Delta::Pct(day)) = dxy.day else {
            panic!("DXY day move should be a percent");
        };
        assert!((day - (105.9 / 105.8 - 1.0) * 100.0).abs() < 1e-9);
        assert_eq!(dxy.spark.len(), 60);
    }

    #[test]
    fn uranium_prefers_live_futures_over_the_imf_index() {
        let mut snap = fallback_snapshot();
        snap.terminal.uranium = Some(69.2);
        snap.terminal.uranium_as_of = "2026-07-01".into();
        let lagging = rows(&snap, 0);
        let imf = lagging.iter().find(|r| r.key == "U3O8").unwrap();
        assert_eq!(imf.label, "Uranium (IMF monthly)");
        assert!(imf.day.is_none());
        snap.quotes.push(Quote {
            symbol: "URANIUM".into(),
            price: 90.05,
            change_pct: 0.17,
            market_cap: None,
            change_abs: Some(0.15),
        });
        let live = rows(&snap, 0);
        let futures = live.iter().find(|r| r.key == "U3O8").unwrap();
        assert_eq!(futures.text, "$90.05");
        assert_eq!(futures.day, Some(Delta::Pct(0.17)));
        assert!(futures.range.is_none());
    }

    #[test]
    fn missing_feeds_render_as_dashes() {
        let rows = rows(&fallback_snapshot(), 0);
        let ndx = rows.iter().find(|r| r.key == "NDX").unwrap();
        assert_eq!(ndx.text, "—");
        assert!(ndx.day.is_none() && ndx.month.is_none() && ndx.range.is_none());
    }
}
