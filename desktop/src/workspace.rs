use crate::model::{Quote, Snapshot};
use serde::{Deserialize, Serialize};

pub use crate::changes::VisitBaseline;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    pub watchlist: Vec<String>,
    pub alerts: Vec<PriceAlert>,
    pub refresh_minutes: u32,
    pub x_pulse_hourly: bool,
    /// Desktop notification when a dollar rule changes status on a new reading.
    pub rule_alerts: bool,
    pub scale: f32,
    pub last_desk: usize,
    pub chart_days: i64,
    pub chart_log: bool,
    pub chart_fair: bool,
    /// Last successfully displayed readings; frozen as the comparison at the next launch.
    pub last_visit: Option<VisitBaseline>,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            watchlist: ["BTC", "NVDA", "MSFT", "CCJ", "CEG", "URA"]
                .map(str::to_owned)
                .to_vec(),
            alerts: Vec::new(),
            refresh_minutes: 15,
            x_pulse_hourly: false,
            rule_alerts: true,
            scale: 1.0,
            last_desk: 0,
            chart_days: 365,
            chart_log: false,
            chart_fair: false,
            last_visit: None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Direction {
    Above,
    Below,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PriceAlert {
    pub symbol: String,
    pub direction: Direction,
    pub threshold: f64,
    pub triggered_at: Option<i64>,
}

impl PriceAlert {
    /// Only call with a newly received quote, never with a startup cache.
    pub fn evaluate(&mut self, price: f64, now: i64) -> bool {
        if self.triggered_at.is_some()
            || !price.is_finite()
            || price <= 0.0
            || !self.threshold.is_finite()
            || self.threshold <= 0.0
        {
            return false;
        }
        let hit = match self.direction {
            Direction::Above => price >= self.threshold,
            Direction::Below => price <= self.threshold,
        };
        if hit {
            self.triggered_at = Some(now);
        }
        hit
    }
}

pub fn instruments(snapshot: &Snapshot) -> Vec<Quote> {
    let mut rows: Vec<_> = snapshot
        .quotes
        .iter()
        .filter(|q| {
            crate::catalog::AI_TICKERS.contains(&q.symbol.as_str())
                || crate::catalog::NUCLEAR_TICKERS.contains(&q.symbol.as_str())
                || crate::catalog::URANIUM_ETFS.contains(&q.symbol.as_str())
        })
        .cloned()
        .collect();
    // Spot instruments take precedence over any similarly named equity feed.
    rows.retain(|q| !matches!(q.symbol.as_str(), "BTC" | "ETH" | "ZEC"));
    for (symbol, price, change) in [
        ("BTC", snapshot.btc.price, snapshot.btc.change_24h_pct),
        (
            "ETH",
            snapshot.btc.eth_price,
            snapshot.btc.eth_change_24h_pct,
        ),
        (
            "ZEC",
            snapshot.btc.zec_price,
            snapshot.btc.zec_change_24h_pct,
        ),
        (
            "STRC",
            snapshot.terminal.strc_price,
            snapshot.terminal.strc_change,
        ),
    ] {
        if let Some(price) = price.filter(|p| p.is_finite() && *p > 0.0) {
            rows.push(Quote {
                symbol: symbol.into(),
                price,
                change_pct: change.unwrap_or(0.0),
                market_cap: None,
                change_abs: None,
            });
        }
    }
    rows.retain(|q| q.price.is_finite() && q.price > 0.0);
    rows.sort_by(|a, b| a.symbol.cmp(&b.symbol));
    rows.dedup_by(|a, b| a.symbol == b.symbol);
    rows
}

pub fn quotes_csv(rows: &[Quote], fetched_unix: i64) -> String {
    let mut csv =
        String::from("symbol,price_usd,change_pct,market_cap_usd,snapshot_refreshed_utc\n");
    let date = chrono::DateTime::from_timestamp(fetched_unix, 0)
        .filter(|_| fetched_unix > 0)
        .map(|t| t.to_rfc3339())
        .unwrap_or_default();
    for row in rows {
        let symbol = row.symbol.replace('"', "\"\"");
        let cap = row
            .market_cap
            .filter(|v| v.is_finite())
            .map(|v| v.to_string())
            .unwrap_or_default();
        csv.push_str(&format!(
            "\"{symbol}\",{},{},{cap},{date}\n",
            row.price, row.change_pct
        ));
    }
    csv
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alerts_trigger_once_and_can_be_rearmed() {
        let mut alert = PriceAlert {
            symbol: "BTC".into(),
            direction: Direction::Above,
            threshold: 100.0,
            triggered_at: None,
        };
        assert!(!alert.evaluate(99.0, 1));
        assert!(alert.evaluate(100.0, 2));
        assert!(!alert.evaluate(110.0, 3));
        assert_eq!(alert.triggered_at, Some(2));
        alert.triggered_at = None;
        alert.direction = Direction::Below;
        assert!(!alert.evaluate(f64::NAN, 4));
        assert!(!alert.evaluate(0.0, 4));
        assert!(alert.evaluate(90.0, 5));
    }

    #[test]
    fn instrument_list_rejects_bad_prices_and_prefers_spot() {
        let mut snap = crate::model::fallback_snapshot();
        snap.btc.price = Some(120.0);
        snap.quotes = vec![
            Quote {
                symbol: "BTC".into(),
                price: 2.0,
                change_pct: 0.0,
                market_cap: None,
                change_abs: None,
            },
            Quote {
                symbol: "BAD".into(),
                price: f64::NAN,
                change_pct: 0.0,
                market_cap: None,
                change_abs: None,
            },
        ];
        let rows = instruments(&snap);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].price, 120.0);
    }

    #[test]
    fn zec_spot_tick_reaches_the_instrument_list() {
        let mut snap = crate::model::fallback_snapshot();
        crate::fetch::apply_tickers(
            &mut snap.btc,
            &[("ZECUSDT".into(), 1217.0, 6.4, 1250.0, 1140.0)],
        );
        let zec = instruments(&snap)
            .into_iter()
            .find(|q| q.symbol == "ZEC")
            .expect("ZEC row");
        assert_eq!((zec.price, zec.change_pct), (1217.0, 6.4));
    }

    #[test]
    fn csv_escapes_symbols_and_does_not_invent_missing_dates() {
        let rows = vec![Quote {
            symbol: "A,\"B".into(),
            price: 3.5,
            change_pct: -2.0,
            market_cap: None,
            change_abs: None,
        }];
        assert!(quotes_csv(&rows, 0).contains("\"A,\"\"B\",3.5,-2,,\n"));
    }
}
