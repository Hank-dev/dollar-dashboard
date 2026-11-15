//! Cross-asset instruments: TradingView delayed quotes carry the session move,
//! CNBC chart bars carry daily history. Both use the same instrument keys.

use crate::fetch::{flatten, get_text};
use crate::model::Point;
use crate::parse;
use reqwest::blocking::Client;
use std::collections::BTreeMap;
use std::thread;

/// Instrument key → CNBC chart symbol.
pub const HISTORY: &[(&str, &str)] = &[
    ("SPX", ".SPX"),
    ("NDX", ".NDX"),
    ("US02Y", "US2Y"),
    ("US10Y", "US10Y"),
    ("US30Y", "US30Y"),
    ("JP10Y", "JP10Y"),
    ("DXY", ".DXY"),
    ("EURUSD", "EUR="),
    ("USDJPY", "JPY="),
    ("VIX", ".VIX"),
    ("MOVE", ".MOVE"),
    // Spot, to match the TradingView spot quotes (front-month futures carry a basis).
    ("GOLD", "XAU="),
    ("SILVER", "XAG="),
    ("BRENT", "@LCO.1"),
    ("WTI", "@CL.1"),
    ("SPCX", "SPCX"),
];

/// Recent listings: CNBC pads the days before the first trade with flat offer-price bars.
const FIRST_TRADE: &[(&str, i64)] = &[("SPCX", crate::catalog::SPACEX_FIRST_TRADE_UNIX)];

/// TradingView scanner tickers for the session quote of each instrument.
pub const QUOTES: &[&str] = &[
    "TVC:DXY",
    "TVC:JP10Y",
    "TVC:GOLD",
    "TVC:SILVER",
    "SP:SPX",
    "NASDAQ:NDX",
    "TVC:VIX",
    "TVC:US02Y",
    "TVC:US10Y",
    "TVC:US30Y",
    "FX:USDJPY",
    "FX:EURUSD",
    // Japan's policy rate; the session change is the size of the last move.
    "ECONOMICS:JPINTR",
];

/// Brent and WTI front months, and UxC uranium (U3O8) futures as the live uranium price.
pub const FUTURES: &[&str] = &["NYMEX:BZ1!", "NYMEX:CL1!", "COMEX:UX1!"];

/// Daily history is slow-moving; refetch it every two hours.
pub const HISTORY_TTL: i64 = 2 * 60 * 60;
const KEEP_POINTS: usize = 420;

pub fn history_due(histories: &BTreeMap<String, Vec<Point>>, fetched_unix: i64, now: i64) -> bool {
    now - fetched_unix > HISTORY_TTL
        || HISTORY
            .iter()
            .any(|(key, _)| histories.get(*key).is_none_or(|points| points.len() < 30))
}

/// Fetch every history in parallel. Failed instruments keep their previous series.
pub fn refresh_histories(
    client: &Client,
    prev: &BTreeMap<String, Vec<Point>>,
) -> (BTreeMap<String, Vec<Point>>, usize, Vec<String>) {
    let results: Vec<(&str, Result<String, String>)> = thread::scope(|scope| {
        let handles: Vec<_> = HISTORY
            .iter()
            .map(|(key, symbol)| {
                let url = format!(
                    "https://ts-api.cnbc.com/harmony/app/charts/1Y.json?symbol={}",
                    symbol.replace('=', "%3D").replace('@', "%40")
                );
                (*key, scope.spawn(move || get_text(client, &url)))
            })
            .collect();
        handles
            .into_iter()
            .map(|(key, handle)| (key, flatten(handle.join())))
            .collect()
    });
    let mut next = prev.clone();
    let mut ok = 0;
    let mut failed = Vec::new();
    for (key, result) in results {
        let mut points = result
            .map(|body| parse::cnbc_bars(&body))
            .unwrap_or_default();
        if let Some((_, first)) = FIRST_TRADE.iter().find(|(k, _)| *k == key) {
            points.retain(|p| p.t >= *first);
        }
        if points.len() >= 30 {
            let start = points.len().saturating_sub(KEEP_POINTS);
            next.insert(key.to_string(), points[start..].to_vec());
            ok += 1;
        } else {
            failed.push(key.to_string());
        }
    }
    (next, ok, failed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_is_due_when_stale_or_incomplete() {
        let full: BTreeMap<String, Vec<Point>> = HISTORY
            .iter()
            .map(|(key, _)| {
                let points = (0..40).map(|i| Point { t: i, v: 1.0 }).collect();
                (key.to_string(), points)
            })
            .collect();
        assert!(!history_due(&full, 1_000, 1_000 + HISTORY_TTL - 1));
        assert!(history_due(&full, 1_000, 1_000 + HISTORY_TTL + 1));
        let mut partial = full.clone();
        partial.remove("VIX");
        assert!(history_due(&partial, 1_000, 1_001));
    }
}
