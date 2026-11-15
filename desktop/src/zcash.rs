//! Zcash desk: Binance spot and perps, a public zcashd node (value pools, supply,
//! upgrades), Blockchair network stats, and CoinGecko market data.
//!
//! Every source is optional. A failed request keeps the previous reading.

use crate::fetch::{flatten, get_text};
use crate::model::Point;
use crate::parse;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::thread;

pub const MAX_SUPPLY: f64 = 21_000_000.0;
/// Post-Blossom schedule: 75 s blocks and a halving every 1,680,000 blocks.
/// Checked against node supply: 16,977,006 ZEC reported at block 3,511,919.
const POST_BLOSSOM_SECONDS: i64 = 75;
const POST_BLOSSOM_SUBSIDY: f64 = 6.25;
const FIRST_HALVING: u64 = 1_046_400;
const SECOND_HALVING: u64 = 2_726_400;
/// ZIP 259 mainnet activation. ZIP 218 sets 25 s blocks, divides the subsidy by 3 and
/// triples the blocks left to each halving, so halvings keep their dates.
pub const NU7_HEIGHT: u64 = 3_543_405;
const POST_NU7_SECONDS: i64 = 25;
/// ZIP 259: 13,219,200 − 2 × NU7_HEIGHT.
const THIRD_HALVING: u64 = 6_132_390;
const POST_NU7_HALVING_INTERVAL: u64 = 5_040_000;
/// One halving era in seconds, the same on both sides of NU7.
const ERA_SECONDS: i64 = 1_680_000 * POST_BLOSSOM_SECONDS;

const DAILY: &str = "https://api.binance.com/api/v3/klines?symbol=ZECUSDT&interval=1d&limit=1000";
const RATIO: &str = "https://api.binance.com/api/v3/ticker/24hr?symbol=ZECBTC";
const RATIO_HIST: &str =
    "https://api.binance.com/api/v3/klines?symbol=ZECBTC&interval=1d&limit=365";
const FUNDING: &str = "https://fapi.binance.com/fapi/v1/fundingRate?symbol=ZECUSDT&limit=21";
const OPEN_INTEREST: &str = "https://fapi.binance.com/fapi/v1/openInterest?symbol=ZECUSDT";
const NODE: &str = "https://mainnet.zcashexplorer.app/api/v1/blockchain-info";
const CHAIN: &str = "https://api.blockchair.com/zcash/stats";
const MARKET: &str = "https://api.coingecko.com/api/v3/coins/zcash?localization=false&tickers=false&community_data=false&developer_data=false&sparkline=false";

// Cypherpunk Technologies (CYPH) inputs, updated by hand from its disclosures.
// Only the CYPH close and ZEC spot are live. Recheck after each 10-Q and monthly mining update.
/// cypherpunk.com: 323,394.38 at 18 Aug (8-K) plus 3,023.13 and 5,944.17 mined to 30 Sep.
pub const CYPH_ZEC: f64 = 332_361.68;
pub const CYPH_ZEC_AS_OF: &str = "30 Sep 2026";
/// Weighted purchase cost of the 323,394.38 ZEC bought; mined ZEC is excluded (8-K, 10-Q).
pub const CYPH_AVG_COST: f64 = 341.83;
/// 107,764,382 on the 10-Q cover (10 Aug) plus 4,938,801 warrant shares exercised 12 Oct (Form 4).
pub const CYPH_COMMON: f64 = 112_703_183.0;
/// Implied by the company's own $722.06M market cap at $2.87. Filings explain 231.8M
/// (common plus 124.06M $0.001 pre-funded warrants); the 10-Q for Q3 should explain the rest.
pub const CYPH_ISSUED_AND_PREFUNDED: f64 = 251_590_000.0;
/// cypherpunk.com, gross of warrant exercise proceeds.
pub const CYPH_FULLY_DILUTED: f64 = 355_130_000.0;
/// The mining deal closed on this date (18 Aug 2026), so earlier ratios at today's counts mislead.
pub const CYPH_RATIO_FROM: i64 = 1_787_011_200;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Pool {
    pub id: String,
    pub zec: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ZecBook {
    pub daily: Vec<Point>,
    /// Coin Metrics daily reference price since launch (28 Oct 2016), for the log-log chart.
    pub history: Vec<Point>,
    pub btc_ratio: Option<f64>,
    pub btc_ratio_change: Option<f64>,
    /// ZEC/BTC daily closes, newest last.
    pub btc_ratio_daily: Vec<Point>,
    pub funding: Option<f64>,
    pub funding_hist: Vec<f64>,
    /// Binance USDT-margined perp open interest, in ZEC.
    pub open_interest: Option<f64>,
    pub height: Option<u64>,
    pub supply: Option<f64>,
    pub pools: Vec<Pool>,
    pub upgrade: String,
    pub upgrade_height: Option<u64>,
    pub tx_24h: Option<f64>,
    pub blocks_24h: Option<f64>,
    /// Equihash solutions per second, 24h average.
    pub hashrate: Option<f64>,
    pub difficulty: Option<f64>,
    pub nodes: Option<f64>,
    pub median_fee_usd: Option<f64>,
    pub mempool_tx: Option<f64>,
    pub market_cap: Option<f64>,
    pub rank: Option<u32>,
    pub volume_24h: Option<f64>,
    pub ath: Option<f64>,
    pub ath_date: String,
    /// Cypherpunk (CYPH) daily closes, newest last.
    pub cyph_daily: Vec<Point>,
}

/// Equity value divided by the spot value of the ZEC held.
pub fn cyph_ratio(shares: f64, cyph_price: f64, zec_price: f64) -> Option<f64> {
    let holdings = CYPH_ZEC * zec_price;
    (holdings > 0.0 && cyph_price > 0.0).then(|| shares * cyph_price / holdings)
}

/// The ratio per CYPH close since the mining deal, at today's share and ZEC counts.
pub fn cyph_ratio_history(cyph: &[Point], zec: &[Point], shares: f64) -> Vec<Point> {
    const DAY: i64 = 86_400;
    cyph.iter()
        .filter(|close| close.t >= CYPH_RATIO_FROM)
        .filter_map(|close| {
            let spot = zec.iter().rev().find(|p| p.t / DAY == close.t / DAY)?;
            Some(Point {
                t: close.t,
                v: cyph_ratio(shares, close.v, spot.v)?,
            })
        })
        .collect()
}

/// What the ratio implies for new share sales. Warrants already paid for count as shares.
pub fn cyph_readout(cyph_price: f64, zec_price: f64) -> Option<String> {
    let paid_up = cyph_ratio(CYPH_ISSUED_AND_PREFUNDED, cyph_price, zec_price)?;
    let common = cyph_ratio(CYPH_COMMON, cyph_price, zec_price)?;
    let hidden = (CYPH_ISSUED_AND_PREFUNDED - CYPH_COMMON) / 1e6;
    let meaning = if paid_up >= 1.0 {
        "selling new stock can buy more ZEC per share than it dilutes"
    } else {
        "selling new stock dilutes ZEC per share, and buybacks funded by ZEC sales become the rational move"
    };
    Some(format!(
        "CYPH equity is {paid_up:.2}× its ZEC counting issued and pre-funded shares, so {meaning}. The {common:.2}× common-only figure leaves out {hidden:.0}M shares."
    ))
}

impl ZecBook {
    pub fn shielded(&self) -> Option<f64> {
        (!self.pools.is_empty()).then(|| {
            self.pools
                .iter()
                .filter(|pool| is_shielded(&pool.id))
                .map(|pool| pool.zec)
                .sum()
        })
    }

    pub fn shielded_share(&self) -> Option<f64> {
        let supply = self.supply.filter(|s| *s > 0.0)?;
        self.shielded().map(|zec| zec / supply * 100.0)
    }

    /// Protocol issuance over a year at the current subsidy, as a share of supply.
    pub fn inflation_pct(&self) -> Option<f64> {
        let supply = self.supply.filter(|s| *s > 0.0)?;
        self.height.map(|h| annual_issuance(h) / supply * 100.0)
    }

    pub fn change_pct(&self, days: i64) -> Option<f64> {
        change_over(&self.daily, days)
    }
}

/// Every node value pool except transparent outputs and the dev-fund lockbox is shielded.
pub fn is_shielded(id: &str) -> bool {
    !matches!(id, "transparent" | "lockbox")
}

pub fn pool_name(id: &str) -> String {
    match id {
        "lockbox" => "Dev-fund lockbox".into(),
        _ => {
            let mut chars = id.chars();
            chars
                .next()
                .map(|c| c.to_uppercase().chain(chars).collect())
                .unwrap_or_default()
        }
    }
}

pub fn pool_note(id: &str) -> &'static str {
    match id {
        "ironwood" => "NU6.3 pool, receives new shielded funds since Jul 2026",
        "orchard" => "NU5 pool, funds migrating to Ironwood",
        "sapling" => "2018 pool",
        "sprout" => "original 2016 pool",
        "lockbox" => "deferred dev-fund share since NU6",
        "transparent" => "public addresses",
        _ => "",
    }
}

fn halvings_before(height: u64) -> u32 {
    match height {
        h if h < FIRST_HALVING => 0,
        h if h < SECOND_HALVING => 1,
        h if h < THIRD_HALVING => 2,
        h => 3 + ((h - THIRD_HALVING) / POST_NU7_HALVING_INTERVAL) as u32,
    }
}

pub fn block_seconds(height: u64) -> i64 {
    if height < NU7_HEIGHT {
        POST_BLOSSOM_SECONDS
    } else {
        POST_NU7_SECONDS
    }
}

pub fn blocks_per_day(height: u64) -> f64 {
    86_400.0 / block_seconds(height) as f64
}

pub fn block_subsidy(height: u64) -> f64 {
    let subsidy = POST_BLOSSOM_SUBSIDY / 2f64.powi(halvings_before(height) as i32);
    if height < NU7_HEIGHT {
        subsidy
    } else {
        // ZIP 218 rounds down to whole zatoshi.
        (subsidy * 1e8 / 3.0).floor() / 1e8
    }
}

pub fn next_halving(height: u64) -> u64 {
    match halvings_before(height) {
        0 => FIRST_HALVING,
        1 => SECOND_HALVING,
        2 => THIRD_HALVING,
        n => THIRD_HALVING + (n as u64 - 2) * POST_NU7_HALVING_INTERVAL,
    }
}

/// Target seconds until `to`, counting the spacing change at NU7.
fn seconds_until(height: u64, to: u64) -> i64 {
    let slow = NU7_HEIGHT.clamp(height, to) - height;
    slow as i64 * POST_BLOSSOM_SECONDS + (to - height - slow) as i64 * POST_NU7_SECONDS
}

pub fn days_to_halving(height: u64) -> f64 {
    seconds_until(height, next_halving(height)) as f64 / 86_400.0
}

pub fn halving_eta_unix(height: u64, now: i64) -> i64 {
    now + seconds_until(height, next_halving(height))
}

/// Share of the current halving era elapsed, measured in time because NU7 falls inside one.
pub fn era_progress(height: u64) -> f64 {
    1.0 - seconds_until(height, next_halving(height)) as f64 / ERA_SECONDS as f64
}

pub fn annual_issuance(height: u64) -> f64 {
    block_subsidy(height) * blocks_per_day(height) * 365.25
}

/// Percent change from the last close to the close nearest `days` earlier.
pub fn change_over(points: &[Point], days: i64) -> Option<f64> {
    let last = points.last()?;
    let target = last.t - days * 86_400;
    let base = points
        .iter()
        .filter(|p| p.t <= target)
        .min_by_key(|p| target - p.t)?;
    (target - base.t <= 3 * 86_400 && base.v > 0.0).then(|| (last.v / base.v - 1.0) * 100.0)
}

/// One plain sentence built only from fields the feeds returned.
pub fn readout(book: &ZecBook) -> String {
    let mut parts = Vec::new();
    if let (Some(zec), Some(share)) = (book.shielded(), book.shielded_share()) {
        let mut line = format!(
            "{:.2}M ZEC ({share:.1}% of supply) sits in shielded pools",
            zec / 1e6
        );
        if let Some(top) = book
            .pools
            .iter()
            .filter(|p| is_shielded(&p.id))
            .max_by(|a, b| a.zec.total_cmp(&b.zec))
            .filter(|_| zec > 0.0)
        {
            line.push_str(&format!(
                "; {} holds {:.0}% of it",
                pool_name(&top.id),
                top.zec / zec * 100.0
            ));
        }
        parts.push(line);
    }
    if let Some(height) = book.height {
        let days = days_to_halving(height);
        parts.push(format!(
            "issuance runs at {} ZEC per block until the halving in about {days:.0} days",
            block_subsidy(height)
        ));
    }
    if parts.is_empty() {
        return "Node and network feeds have not answered yet. Refresh sources to load shielded supply and issuance.".into();
    }
    let mut text = parts.join(", and ");
    if let Some(first) = text.get(0..1) {
        text = first.to_uppercase() + &text[1..];
    }
    text.push('.');
    text
}

pub struct NodeInfo {
    pub height: u64,
    pub supply: Option<f64>,
    pub pools: Vec<Pool>,
    pub upgrade: Option<(String, u64)>,
}

pub fn parse_node(body: &str) -> Option<NodeInfo> {
    let json: Value = serde_json::from_str(body).ok()?;
    let height = json["blocks"].as_u64()?;
    let pools = json["valuePools"]
        .as_array()
        .map(|rows| {
            rows.iter()
                .filter_map(|row| {
                    Some(Pool {
                        id: row["id"].as_str()?.to_string(),
                        zec: num(&row["chainValue"]).filter(|v| *v >= 0.0)?,
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    let upgrade = json["upgrades"].as_object().and_then(|map| {
        map.values()
            .filter(|u| u["status"].as_str() == Some("active"))
            .filter_map(|u| {
                Some((
                    u["name"].as_str()?.to_string(),
                    u["activationheight"].as_u64()?,
                ))
            })
            .max_by_key(|(_, h)| *h)
    });
    Some(NodeInfo {
        height,
        supply: num(&json["chainSupply"]["chainValue"]).filter(|v| *v > 0.0),
        pools,
        upgrade,
    })
}

fn apply_chain(book: &mut ZecBook, body: &str) -> bool {
    let Ok(json) = serde_json::from_str::<Value>(body) else {
        return false;
    };
    let data = &json["data"];
    if !data.is_object() {
        return false;
    }
    book.tx_24h = num(&data["transactions_24h"]).or(book.tx_24h);
    book.blocks_24h = num(&data["blocks_24h"]).or(book.blocks_24h);
    book.hashrate = num(&data["hashrate_24h"]).or(book.hashrate);
    book.difficulty = num(&data["difficulty"]).or(book.difficulty);
    book.nodes = num(&data["nodes"]).or(book.nodes);
    book.median_fee_usd = num(&data["median_transaction_fee_usd_24h"]).or(book.median_fee_usd);
    book.mempool_tx = num(&data["mempool_transactions"]).or(book.mempool_tx);
    true
}

fn apply_market(book: &mut ZecBook, body: &str) -> bool {
    let Ok(json) = serde_json::from_str::<Value>(body) else {
        return false;
    };
    let market = &json["market_data"];
    let Some(cap) = num(&market["market_cap"]["usd"]).filter(|v| *v > 0.0) else {
        return false;
    };
    book.market_cap = Some(cap);
    book.rank = json["market_cap_rank"]
        .as_u64()
        .map(|r| r as u32)
        .or(book.rank);
    book.volume_24h = num(&market["total_volume"]["usd"]).or(book.volume_24h);
    book.ath = num(&market["ath"]["usd"]).or(book.ath);
    if let Some(date) = market["ath_date"]["usd"].as_str() {
        book.ath_date = date.get(0..10).unwrap_or(date).to_string();
    }
    if book.supply.is_none() {
        book.supply = num(&market["circulating_supply"]).filter(|v| *v > 0.0);
    }
    true
}

fn num(value: &Value) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value.as_str().and_then(|s| s.parse().ok()))
        .filter(|v| v.is_finite())
}

/// Returns the next book, readable failures, and the feed ids that answered.
pub fn refresh(client: &Client, prev: &ZecBook) -> (ZecBook, Vec<String>, Vec<&'static str>) {
    let (daily, ratio, ratio_hist, funding, open_interest, node, chain, market, history, cyph) =
        thread::scope(|scope| {
            let daily = scope.spawn(|| get_text(client, DAILY));
            let ratio = scope.spawn(|| get_text(client, RATIO));
            let ratio_hist = scope.spawn(|| get_text(client, RATIO_HIST));
            let funding = scope.spawn(|| get_text(client, FUNDING));
            let open_interest = scope.spawn(|| get_text(client, OPEN_INTEREST));
            let node = scope.spawn(|| get_text(client, NODE));
            let chain = scope.spawn(|| get_text(client, CHAIN));
            let market = scope.spawn(|| get_text(client, MARKET));
            let history =
                scope.spawn(|| crate::readouts::coinmetrics_prices(client, "zec", "2016-10-28"));
            let cyph = scope.spawn(|| crate::readouts::yahoo_daily(client, "CYPH", "1y"));
            (
                daily.join(),
                ratio.join(),
                ratio_hist.join(),
                funding.join(),
                open_interest.join(),
                node.join(),
                chain.join(),
                market.join(),
                history.join().ok().flatten(),
                cyph.join().ok().flatten(),
            )
        });
    let mut next = prev.clone();
    let mut notes = Vec::new();
    let mut feeds = Vec::new();
    match cyph {
        Some(points) => {
            next.cyph_daily = points;
            feeds.push("yahoo-cyph");
        }
        None => notes.push("Cypherpunk price from Yahoo Finance was unavailable.".into()),
    }
    match history {
        Some(points) if points.len() >= 365 => {
            next.history = points;
            feeds.push("coinmetrics-zec");
        }
        _ => notes.push("Zcash full history from Coin Metrics was unavailable.".into()),
    }
    let mut take = |label: &str,
                    feed: &'static str,
                    result: thread::Result<Result<String, String>>,
                    apply: &mut dyn FnMut(&str) -> bool| match flatten(result) {
        Ok(body) => {
            if apply(&body) {
                if !feeds.contains(&feed) {
                    feeds.push(feed);
                }
            } else {
                notes.push(format!("Zcash {label} response was not readable."));
            }
        }
        Err(err) => notes.push(format!("Zcash {label}: {err}")),
    };
    take("daily history", "binance-daily", daily, &mut |body| {
        let points = parse::klines(body);
        let usable = points.len() >= 30;
        if usable {
            next.daily = points;
        }
        usable
    });
    take("ZEC/BTC", "binance-spot", ratio, &mut |body| {
        let row = parse::binance_tickers(body).into_iter().next();
        if let Some((_, price, change, _, _)) = &row {
            next.btc_ratio = Some(*price);
            next.btc_ratio_change = Some(*change);
        }
        row.is_some()
    });
    take(
        "ZEC/BTC history",
        "binance-daily",
        ratio_hist,
        &mut |body| {
            let points = parse::klines(body);
            let usable = points.len() >= 2;
            if usable {
                next.btc_ratio_daily = points;
            }
            usable
        },
    );
    take("funding", "binance-funding", funding, &mut |body| {
        let rates = parse::funding_rates(body);
        let latest = rates.last().copied();
        if latest.is_some() {
            next.funding = latest;
            next.funding_hist = rates;
        }
        latest.is_some()
    });
    take(
        "open interest",
        "binance-funding",
        open_interest,
        &mut |body| {
            let value = serde_json::from_str::<Value>(body)
                .ok()
                .and_then(|json| num(&json["openInterest"]));
            if value.is_some() {
                next.open_interest = value;
            }
            value.is_some()
        },
    );
    take("node", "zcashd", node, &mut |body| {
        let Some(info) = parse_node(body) else {
            return false;
        };
        next.height = Some(info.height);
        next.supply = info.supply.or(next.supply);
        if !info.pools.is_empty() {
            next.pools = info.pools;
        }
        if let Some((name, height)) = info.upgrade {
            next.upgrade = name;
            next.upgrade_height = Some(height);
        }
        true
    });
    take("network stats", "blockchair", chain, &mut |body| {
        apply_chain(&mut next, body)
    });
    take("market data", "coingecko-zec", market, &mut |body| {
        apply_market(&mut next, body)
    });
    (next, notes, feeds)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "calls live Binance, zcashd node, Blockchair and CoinGecko endpoints"]
    fn live_refresh_reads_every_source() {
        let client = Client::builder()
            .user_agent("Mozilla/5.0 (compatible; dollar-dashboard/0.1)")
            .build()
            .unwrap();
        let (book, notes, feeds) = refresh(&client, &ZecBook::default());
        println!(
            "{notes:?} feeds={feeds:?} height={:?} shielded={:?} rank={:?} funding={:?} oi={:?} ratio={:?}",
            book.height,
            book.shielded_share(),
            book.rank,
            book.funding,
            book.open_interest,
            book.btc_ratio
        );
        println!("{}", readout(&book));
        assert!(notes.is_empty(), "{notes:?}");
        assert_eq!(feeds.len(), 8);
    }

    #[test]
    fn cyph_ratio_changes_with_the_share_count() {
        let ratio = |shares| cyph_ratio(shares, 2.87, 1_300.0).unwrap();
        assert!((ratio(CYPH_COMMON) - 0.749).abs() < 0.001);
        assert!((ratio(CYPH_ISSUED_AND_PREFUNDED) - 1.671).abs() < 0.001);
        assert!((ratio(CYPH_FULLY_DILUTED) - 2.359).abs() < 0.001);
        assert_eq!(cyph_ratio(CYPH_COMMON, 0.0, 1_300.0), None);
        assert_eq!(cyph_ratio(CYPH_COMMON, 2.87, 0.0), None);
    }

    #[test]
    fn cyph_readout_flips_when_paid_up_shares_fall_below_holdings() {
        assert!(cyph_readout(2.87, 1_300.0)
            .unwrap()
            .contains("more ZEC per share"));
        assert!(cyph_readout(1.00, 1_300.0).unwrap().contains("buybacks"));
        assert_eq!(cyph_readout(2.87, 0.0), None);
    }

    #[test]
    fn cyph_history_joins_by_day_and_starts_at_the_mining_deal() {
        let day = 86_400;
        let start = CYPH_RATIO_FROM;
        let point = |t, v| Point { t, v };
        let cyph = [
            point(start - day + 48_600, 1.0),
            point(start + 48_600, 2.0),
            point(start + day + 48_600, 3.0),
            point(start + 2 * day + 48_600, 3.0),
        ];
        let zec = [
            point(start - day, 500.0),
            point(start, 500.0),
            point(start + 2 * day, 1_000.0),
        ];
        let series = cyph_ratio_history(&cyph, &zec, CYPH_COMMON);
        assert_eq!(series.len(), 2);
        assert_eq!(series[0].t, start + 48_600);
        assert_eq!(series[0].v, cyph_ratio(CYPH_COMMON, 2.0, 500.0).unwrap());
        assert_eq!(series[1].v, cyph_ratio(CYPH_COMMON, 3.0, 1_000.0).unwrap());
    }

    #[test]
    fn halving_schedule_matches_protocol_heights() {
        assert_eq!(block_subsidy(900_000), 6.25);
        assert_eq!(block_subsidy(1_046_400), 3.125);
        assert_eq!(block_subsidy(2_726_399), 3.125);
        assert_eq!(block_subsidy(2_726_400), 1.5625);
        assert_eq!(next_halving(2_726_399), 2_726_400);
        assert_eq!(next_halving(3_511_919), 6_132_390);
        assert_eq!(next_halving(6_132_389), 6_132_390);
        assert_eq!(next_halving(6_132_390), 11_172_390);
    }

    #[test]
    fn nu7_matches_zip_218_and_keeps_halving_dates_and_issuance() {
        // ZIP 218 worked values, in zatoshi.
        assert_eq!(block_subsidy(NU7_HEIGHT - 1), 1.5625);
        assert_eq!(block_subsidy(NU7_HEIGHT), 0.52083333);
        assert_eq!(block_subsidy(THIRD_HALVING - 1), 0.52083333);
        assert_eq!(block_subsidy(THIRD_HALVING), 0.26041666);
        assert_eq!(THIRD_HALVING, 13_219_200 - 2 * NU7_HEIGHT);
        assert_eq!(
            (blocks_per_day(NU7_HEIGHT - 1), blocks_per_day(NU7_HEIGHT)),
            (1_152.0, 3_456.0)
        );

        // The old schedule put this halving at block 4,406,400 with 75 s blocks throughout.
        let height = 3_541_789;
        assert_eq!(
            halving_eta_unix(height, 0),
            (4_406_400 - height as i64) * 75
        );
        assert_eq!(halving_eta_unix(THIRD_HALVING - 100, 0), 100 * 25);
        assert_eq!(
            seconds_until(NU7_HEIGHT - 10, NU7_HEIGHT + 10),
            10 * 75 + 10 * 25
        );

        let before = annual_issuance(NU7_HEIGHT - 1);
        assert!((annual_issuance(NU7_HEIGHT) / before - 1.0).abs() < 1e-6);
    }

    #[test]
    fn era_progress_is_continuous_across_nu7_and_each_era_boundary() {
        let across = era_progress(NU7_HEIGHT) - era_progress(NU7_HEIGHT - 1);
        assert!((across - 75.0 / ERA_SECONDS as f64).abs() < 1e-12);
        assert_eq!(era_progress(SECOND_HALVING), 0.0);
        assert_eq!(era_progress(THIRD_HALVING), 0.0);
        assert!(era_progress(THIRD_HALVING - 1) > 0.999_999);
    }

    #[test]
    fn node_pools_split_into_shielded_and_public() {
        let body = r#"{"blocks":3511919,"chainSupply":{"chainValue":1000.0},
            "valuePools":[{"id":"transparent","chainValue":700.0},{"id":"sapling","chainValue":50.0},
            {"id":"ironwood","chainValue":200.0},{"id":"lockbox","chainValue":50.0}],
            "upgrades":{"a":{"name":"NU6","activationheight":2726400,"status":"active"},
            "b":{"name":"NU6.3","activationheight":3428143,"status":"active"},
            "c":{"name":"NU7","activationheight":3900000,"status":"pending"}}}"#;
        let info = parse_node(body).unwrap();
        assert_eq!(info.upgrade, Some(("NU6.3".into(), 3_428_143)));
        let book = ZecBook {
            supply: info.supply,
            pools: info.pools,
            height: Some(info.height),
            ..Default::default()
        };
        assert_eq!(book.shielded(), Some(250.0));
        assert_eq!(book.shielded_share(), Some(25.0));
        assert!(readout(&book)
            .starts_with("0.00M ZEC (25.0% of supply) sits in shielded pools; Ironwood holds 80%"));
    }

    #[test]
    fn change_needs_a_close_near_the_lookback() {
        let day = 86_400;
        let points: Vec<Point> = (0..10)
            .map(|i| Point {
                t: i * day,
                v: 100.0 + i as f64 * 10.0,
            })
            .collect();
        assert_eq!(change_over(&points, 7), Some((190.0 / 120.0 - 1.0) * 100.0));
        assert_eq!(change_over(&points, 30), None);
    }

    #[test]
    fn blockchair_and_coingecko_fields_parse_strings_and_numbers() {
        let mut book = ZecBook::default();
        assert!(apply_chain(
            &mut book,
            r#"{"data":{"transactions_24h":8014,"hashrate_24h":"29710418226","nodes":978}}"#
        ));
        assert_eq!(book.hashrate, Some(29_710_418_226.0));
        assert_eq!(book.tx_24h, Some(8014.0));
        assert!(apply_market(
            &mut book,
            r#"{"market_cap_rank":10,"market_data":{"market_cap":{"usd":2.0e10},
            "ath":{"usd":3191.93},"ath_date":{"usd":"2016-10-28T16:00:00.000Z"},"circulating_supply":16975986.1}}"#
        ));
        assert_eq!(
            (book.rank, book.ath_date.as_str()),
            (Some(10), "2016-10-28")
        );
        assert_eq!(book.supply, Some(16_975_986.1));
        assert!(!apply_market(&mut book, r#"{"status":{"error_code":429}}"#));
    }
}
