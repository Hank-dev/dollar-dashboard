//! A compact, dated comparison with the previous visit, plus notable session moves.

use crate::board::{self, Link};
use crate::catalog::{AI_TICKERS, NUCLEAR_TICKERS};
use crate::feeds::{self, Freshness, FEEDS};
use crate::model::{self, Delta, Snapshot, Status};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct VisitBaseline {
    pub captured_unix: i64,
    pub prices: BTreeMap<String, PriceReading>,
    pub rules: BTreeMap<String, RuleReading>,
    pub feeds: BTreeMap<String, i64>,
    pub issues: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PriceReading {
    pub value: f64,
    /// Observation time when known; receipt time for undated delayed quotes.
    pub sample_unix: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RuleReading {
    pub value: String,
    pub status: Status,
    pub observed_unix: i64,
}

impl VisitBaseline {
    pub fn from_snapshot(snapshot: &Snapshot, now: i64) -> Self {
        let prices = board::rows(snapshot, now)
            .into_iter()
            .filter_map(|row| {
                let at = row.observed_unix.or(row.acquired_unix)?;
                let value = row.last.filter(|v| v.is_finite() && *v > 0.0)?;
                (at > 0).then(|| {
                    (
                        row.key.to_string(),
                        PriceReading {
                            value,
                            sample_unix: at,
                        },
                    )
                })
            })
            .collect();
        let rules = model::dollar_metrics(snapshot)
            .into_iter()
            .filter_map(|metric| {
                let (at, _) = rule_evidence(snapshot, metric.id)?;
                (at > 0).then(|| {
                    (
                        metric.id.to_string(),
                        RuleReading {
                            value: metric.value,
                            status: metric.status,
                            observed_unix: at,
                        },
                    )
                })
            })
            .collect();
        Self {
            captured_unix: now,
            prices,
            rules,
            feeds: snapshot.feeds.clone(),
            issues: snapshot.notes.len(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Target {
    Bitcoin,
    Dollar(Option<String>),
    Ai,
    Nuclear,
    Zcash,
    SpaceX,
    Sources,
}

#[derive(Clone, Debug)]
pub struct Development {
    pub id: String,
    pub title: String,
    pub detail: String,
    pub evidence: String,
    pub status: Status,
    pub target: Target,
    priority: f64,
}

/// At most three events. Missing or unchanged observations never invent a comparison.
pub fn developments(
    snapshot: &Snapshot,
    baseline: Option<&VisitBaseline>,
    now: i64,
) -> Vec<Development> {
    let mut out = Vec::new();
    for row in board::rows(snapshot, now) {
        let max_age = if row.group == "Crypto" {
            2 * 3600
        } else {
            4 * 86_400
        };
        let Some(at) = row
            .observed_unix
            .or(row.acquired_unix)
            .filter(|t| *t > 0 && *t <= now + 60 && now - *t <= max_age)
        else {
            continue;
        };
        let target = match row.link {
            Link::Bitcoin => Target::Bitcoin,
            Link::Dollar => Target::Dollar(None),
            Link::Nuclear => Target::Nuclear,
            Link::Zcash => Target::Zcash,
            Link::SpaceX => Target::SpaceX,
            Link::None => Target::Sources,
        };
        let evidence = format!(
            "{} · {} {}",
            row.source,
            if row.observed_unix.is_some() {
                "observed"
            } else {
                "received"
            },
            timestamp(at)
        );
        let typical = typical_daily_move(&row.spark);
        let previous = baseline.and_then(|b| b.prices.get(row.key));
        if let (Some(old), Some(value)) = (previous, row.last) {
            if at > old.sample_unix && old.value > 0.0 {
                let change = (value / old.value - 1.0) * 100.0;
                if change.abs() >= typical.unwrap_or(0.0).max(1.0)
                    && !matches!(
                        row.key,
                        "US02Y" | "US10Y" | "US30Y" | "JP10Y" | "2S10S" | "HY"
                    )
                {
                    out.push(Development {
                        id: format!("price:{}", row.key),
                        title: format!(
                            "{} {} since your last visit",
                            row.label,
                            model::fmt_change_pct(change)
                        ),
                        detail: format!(
                            "{} → {} · previous reading {}{}",
                            model::fmt_price(old.value),
                            row.text,
                            timestamp(old.sample_unix),
                            relative_size(change, typical)
                        ),
                        evidence: evidence.clone(),
                        status: move_status(row.key, change, typical),
                        target: target.clone(),
                        priority: 80.0 + rank_bonus(change, typical),
                    });
                    continue;
                }
            }
        }
        let Some(Delta::Pct(change)) = row.day else {
            continue;
        };
        if change.abs() >= (1.5 * typical.unwrap_or(0.0)).max(2.0) {
            out.push(Development {
                id: format!("price:{}", row.key),
                title: format!(
                    "{} {} {}",
                    row.label,
                    model::fmt_change_pct(change),
                    if row.group == "Crypto" {
                        "over 24h"
                    } else {
                        "this session"
                    }
                ),
                detail: format!(
                    "Latest {} · session move{}",
                    row.text,
                    relative_size(change, typical)
                ),
                evidence,
                status: move_status(row.key, change, typical),
                target,
                priority: 65.0 + rank_bonus(change, typical),
            });
        }
    }
    for (name, symbols, target) in [
        ("AI basket", AI_TICKERS, Target::Ai),
        ("Nuclear basket", NUCLEAR_TICKERS, Target::Nuclear),
    ] {
        // Exclude retained/undated constituents from both the value and its weight.
        let quotes: Vec<_> = snapshot
            .quotes
            .iter()
            .filter(|q| {
                symbols.contains(&q.symbol.as_str())
                    && snapshot
                        .price_observed_unix(&q.symbol)
                        .or(snapshot.price_acquired_unix(&q.symbol))
                        .is_some_and(|t| t > 0 && t <= now + 60 && now - t <= 4 * 86_400)
            })
            .cloned()
            .collect();
        if let Some(change) = model::weighted_change(&quotes, symbols).filter(|c| c.abs() >= 2.0) {
            let at = quotes
                .iter()
                .filter_map(|q| snapshot.price_acquired_unix(&q.symbol))
                .min();
            out.push(Development {
                id: format!("basket:{name}"),
                title: format!("{name} {} this session", model::fmt_change_pct(change)),
                detail: format!(
                    "Cap-weighted · {} of {} names with dated receipts",
                    quotes.len(),
                    symbols.len()
                ),
                evidence: format!(
                    "TradingView delayed quotes · oldest receipt {}",
                    at.map(timestamp).unwrap_or_default()
                ),
                status: move_status("", change, None),
                target,
                priority: 75.0 + change.abs().min(20.0),
            });
        }
    }
    if let Some(baseline) = baseline {
        for metric in model::dollar_metrics(snapshot) {
            let Some(old) = baseline.rules.get(metric.id) else {
                continue;
            };
            let Some((at, source)) =
                rule_evidence(snapshot, metric.id).filter(|(t, _)| now - *t <= 4 * 86_400)
            else {
                continue;
            };
            if metric.status != old.status && at >= old.observed_unix && metric.value != old.value {
                out.push(Development {
                    id: format!("rule:{}", metric.id),
                    title: format!(
                        "{}: {} → {}",
                        metric.label,
                        old.status.label(),
                        metric.status.label()
                    ),
                    detail: format!("{} → {} · {}", old.value, metric.value, metric.rule),
                    evidence: format!("{source} · data date {}", date_label(at)),
                    status: metric.status,
                    target: Target::Dollar(Some(metric.id.to_string())),
                    priority: if metric.status == Status::Stressed {
                        115.0
                    } else {
                        95.0
                    },
                });
            }
        }
        let mut lost = Vec::new();
        let mut recovered = Vec::new();
        for feed in FEEDS {
            let before = feeds::freshness(
                feed,
                baseline.feeds.get(feed.id).copied(),
                baseline.captured_unix,
            );
            let after = feeds::freshness(feed, snapshot.feeds.get(feed.id).copied(), now);
            if before == Freshness::Fresh && after != Freshness::Fresh {
                lost.push(feed.provider);
            }
            if before == Freshness::Stale && after == Freshness::Fresh {
                recovered.push(feed.provider);
            }
        }
        if !lost.is_empty() {
            lost.sort_unstable();
            lost.dedup();
            out.push(Development {
                id: "sources:stale".into(),
                title: format!(
                    "{} source{} became stale",
                    lost.len(),
                    if lost.len() == 1 { "" } else { "s" }
                ),
                detail: lost.join(", "),
                evidence:
                    "Compared with your previous visit · open Sources for per-feed response times"
                        .into(),
                status: Status::Elevated,
                target: Target::Sources,
                priority: 50.0,
            });
        } else if !recovered.is_empty() {
            recovered.sort_unstable();
            recovered.dedup();
            out.push(Development {
                id: "sources:recovered".into(),
                title: "Previously stale sources are responding again".into(),
                detail: recovered.join(", "),
                evidence: "Compared with your previous visit · open Sources for response times"
                    .into(),
                status: Status::Calm,
                target: Target::Sources,
                priority: 30.0,
            });
        }
        if snapshot.notes.len() > baseline.issues {
            out.push(Development {
                id: "sources:issues".into(),
                title: format!(
                    "{} issue{} in the latest refresh",
                    snapshot.notes.len(),
                    if snapshot.notes.len() == 1 { "" } else { "s" }
                ),
                detail: snapshot.notes.first().cloned().unwrap_or_default(),
                evidence: "Previous values may be retained · open Sources for the affected feeds"
                    .into(),
                status: Status::Elevated,
                target: Target::Sources,
                priority: 45.0,
            });
        }
    }
    out.sort_by(|a, b| b.priority.total_cmp(&a.priority).then(a.id.cmp(&b.id)));
    out.truncate(3);
    out
}

/// Rows where a rise, not a fall, is the stress direction.
const RISE_IS_STRESS: &[&str] = &["VIX", "MOVE", "DXY", "USDJPY"];

/// A move against the row's stress direction is at least Elevated; it is
/// Stressed when it is twice a typical day (or 5% when no history exists).
fn move_status(key: &str, change: f64, typical: Option<f64>) -> Status {
    let adverse = if RISE_IS_STRESS.contains(&key) {
        change
    } else {
        -change
    };
    if adverse <= 0.0 {
        return Status::Neutral;
    }
    let severe = match typical {
        Some(sd) => adverse >= (2.0 * sd).max(2.0),
        None => adverse >= 5.0,
    };
    if severe {
        Status::Stressed
    } else {
        Status::Elevated
    }
}

/// Standard deviation of daily percentage changes over the cached closes.
/// None when there is too little history to call any move typical.
pub fn typical_daily_move(closes: &[f64]) -> Option<f64> {
    let moves: Vec<f64> = closes
        .windows(2)
        .filter(|w| w[0].is_finite() && w[1].is_finite() && w[0] > 0.0 && w[1] > 0.0)
        .map(|w| (w[1] / w[0] - 1.0) * 100.0)
        .collect();
    if moves.len() < 20 {
        return None;
    }
    let mean = moves.iter().sum::<f64>() / moves.len() as f64;
    let var = moves.iter().map(|m| (m - mean).powi(2)).sum::<f64>() / (moves.len() - 1) as f64;
    let sd = var.sqrt();
    (sd.is_finite() && sd > 0.0).then_some(sd)
}

/// Ranking uses the move relative to the instrument's own typical day, so a
/// routine swing in a volatile row does not outrank a rare one in a quiet row.
fn rank_bonus(change: f64, typical: Option<f64>) -> f64 {
    match typical {
        Some(sd) => (change.abs() / sd * 5.0).min(20.0),
        None => change.abs().min(20.0),
    }
}

fn relative_size(change: f64, typical: Option<f64>) -> String {
    match typical {
        Some(sd) => format!(" · {:.1}× a typical day", change.abs() / sd),
        None => String::new(),
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RuleChange {
    pub id: &'static str,
    pub label: &'static str,
    pub from: Status,
    pub to: Status,
    pub value: String,
    pub rule: &'static str,
}

/// Rules whose status moved between two snapshots on a newer dated reading.
/// Embedded fallback values and stale evidence never count as a change.
pub fn rule_changes(before: &Snapshot, after: &Snapshot, now: i64) -> Vec<RuleChange> {
    let old = model::dollar_metrics(before);
    model::dollar_metrics(after)
        .into_iter()
        .filter_map(|metric| {
            let was = old.iter().find(|m| m.id == metric.id)?;
            let (old_at, _) = rule_evidence(before, metric.id)?;
            let (at, _) = rule_evidence(after, metric.id).filter(|(t, _)| now - *t <= 4 * 86_400)?;
            (metric.status != was.status && metric.value != was.value && at >= old_at).then_some(
                RuleChange {
                    id: metric.id,
                    label: metric.label,
                    from: was.status,
                    to: metric.status,
                    value: metric.value,
                    rule: metric.rule,
                },
            )
        })
        .collect()
}

fn timestamp(t: i64) -> String {
    chrono::DateTime::from_timestamp(t, 0)
        .map(|d| d.format("%-d %b %H:%M UTC").to_string())
        .unwrap_or_else(|| "unknown".into())
}

fn date_label(t: i64) -> String {
    chrono::DateTime::from_timestamp(t, 0)
        .map(|d| d.format("%-d %b %Y").to_string())
        .unwrap_or_default()
}

fn rule_evidence(snapshot: &Snapshot, id: &str) -> Option<(i64, String)> {
    let d = &snapshot.dollar;
    let obs = match id {
        "ust30" => d.ust30.as_ref(),
        "ust10" => d.ust10.as_ref(),
        "ust2" | "curve" => d.ust2.as_ref(),
        "fed" => d.fed_hi.as_ref(),
        "dxy" => d.dxy.as_ref(),
        "usdjpy" => d.usdjpy.as_ref(),
        "jgb10" => d.jgb10.as_ref(),
        "boj" => d.boj.as_ref(),
        "vix" => d.vix.as_ref(),
        "spx" => d.spx.as_ref(),
        "gold" => d.gold.as_ref(),
        "brent" => d.brent.as_ref(),
        _ => None,
    }?;
    if obs.source.to_ascii_lowercase().contains("snapshot") {
        return None;
    }
    let at = chrono::NaiveDate::parse_from_str(obs.date.get(..10)?, "%Y-%m-%d")
        .ok()?
        .and_hms_opt(0, 0, 0)?
        .and_utc()
        .timestamp();
    Some((at, obs.source.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Point, PriceStamp, Quote};

    fn now() -> i64 {
        chrono::NaiveDate::from_ymd_opt(2026, 11, 5)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap()
            .and_utc()
            .timestamp()
    }

    fn stamp(snapshot: &mut Snapshot, symbol: &str, at: i64) {
        snapshot.price_stamps.insert(
            symbol.into(),
            PriceStamp {
                acquired_unix: at,
                observed_unix: Some(at),
                source: "test feed".into(),
            },
        );
    }

    #[test]
    fn first_visit_surfaces_nuclear_move_without_inventing_comparisons() {
        let mut snapshot = model::fallback_snapshot();
        snapshot.quotes = vec![Quote {
            symbol: "CEG".into(),
            price: 100.0,
            change_pct: -7.3,
            market_cap: Some(1_000.0),
            change_abs: None,
        }];
        stamp(&mut snapshot, "CEG", now());
        let events = developments(&snapshot, None, now());
        assert_eq!(events[0].target, Target::Nuclear);
        assert!(events[0].title.contains("−7.30% this session"));
        assert!(!events.iter().any(|e| e.title.contains("last visit")));
        assert!(events[0].evidence.contains("receipt"));
    }

    #[test]
    fn comparison_requires_a_newer_received_reading() {
        let mut snapshot = model::fallback_snapshot();
        snapshot.btc.price = Some(100.0);
        stamp(&mut snapshot, "BTC", now() - 60);
        let baseline = VisitBaseline::from_snapshot(&snapshot, now() - 60);
        snapshot.btc.price = Some(105.0);
        assert!(developments(&snapshot, Some(&baseline), now()).is_empty());
        stamp(&mut snapshot, "BTC", now());
        let events = developments(&snapshot, Some(&baseline), now());
        assert_eq!(events[0].id, "price:BTC");
        assert!(events[0].title.contains("+5.00% since your last visit"));
        assert!(events[0].detail.contains("100"));
    }

    #[test]
    fn baseline_excludes_undated_and_embedded_fallback_readings_and_round_trips() {
        let mut snapshot = model::fallback_snapshot();
        let baseline = VisitBaseline::from_snapshot(&snapshot, now());
        assert!(baseline.prices.is_empty());
        assert!(baseline.rules.is_empty());
        assert!(developments(&snapshot, None, now()).is_empty());
        stamp(&mut snapshot, "BTC", now());
        let baseline = VisitBaseline::from_snapshot(&snapshot, now());
        let prefs = crate::workspace::Preferences {
            last_visit: Some(baseline),
            ..Default::default()
        };
        let restored: crate::workspace::Preferences =
            serde_json::from_str(&serde_json::to_string(&prefs).unwrap()).unwrap();
        assert_eq!(
            restored.last_visit.unwrap().prices["BTC"].sample_unix,
            now()
        );
        let legacy: crate::workspace::Preferences = serde_json::from_str("{}").unwrap();
        assert!(legacy.last_visit.is_none());
    }

    #[test]
    fn threshold_crossings_outrank_session_moves_and_link_to_the_rule() {
        let mut snapshot = model::fallback_snapshot();
        snapshot.dollar.ust30 = Some(model::Obs {
            date: "2026-11-04".into(),
            value: 4.9,
            source: "U.S. Treasury".into(),
        });
        let baseline = VisitBaseline::from_snapshot(&snapshot, now() - 86_400);
        snapshot.dollar.ust30 = Some(model::Obs {
            date: "2026-11-05".into(),
            value: 5.1,
            source: "U.S. Treasury".into(),
        });
        snapshot.btc.price = Some(100.0);
        snapshot.btc.change_24h_pct = Some(5.0);
        stamp(&mut snapshot, "BTC", now());
        let events = developments(&snapshot, Some(&baseline), now());
        assert_eq!(events[0].target, Target::Dollar(Some("ust30".into())));
        assert!(events[0].title.contains("Elevated → Stressed"));
        assert!(events[0].detail.contains("5%"));
        assert!(events.len() <= 3);
    }

    #[test]
    fn a_recent_receipt_does_not_make_an_old_observation_new() {
        let mut snapshot = model::fallback_snapshot();
        snapshot.btc.price = Some(100.0);
        stamp(&mut snapshot, "BTC", now() - 60);
        let baseline = VisitBaseline::from_snapshot(&snapshot, now() - 60);
        snapshot.btc.price = Some(105.0);
        snapshot.price_stamps.get_mut("BTC").unwrap().acquired_unix = now();
        assert!(developments(&snapshot, Some(&baseline), now()).is_empty());
        snapshot.btc.change_24h_pct = Some(7.0);
        snapshot.price_stamps.get_mut("BTC").unwrap().observed_unix = Some(now() - 3 * 86_400);
        assert!(developments(&snapshot, Some(&baseline), now()).is_empty());
        let captured = VisitBaseline::from_snapshot(&snapshot, now());
        assert_eq!(captured.prices["BTC"].sample_unix, now() - 3 * 86_400);
    }

    fn closes(step_pct: f64) -> Vec<Point> {
        // Alternating up/down days of a fixed size give a known typical move.
        let mut v = 100.0;
        (0..90)
            .map(|i| {
                v *= 1.0 + if i % 2 == 0 { step_pct } else { -step_pct } / 100.0;
                Point {
                    t: now() - (90 - i) * 86_400,
                    v,
                }
            })
            .collect()
    }

    fn quote(symbol: &str, price: f64, change_pct: f64) -> Quote {
        Quote {
            symbol: symbol.into(),
            price,
            change_pct,
            market_cap: None,
            change_abs: None,
        }
    }

    #[test]
    fn a_volatility_spike_is_stress_and_a_fall_is_not() {
        assert_eq!(move_status("VIX", 25.0, Some(6.0)), Status::Stressed);
        assert_eq!(move_status("VIX", 7.0, Some(6.0)), Status::Elevated);
        assert_eq!(move_status("VIX", -6.0, Some(6.0)), Status::Neutral);
        assert_eq!(move_status("SPX", -3.0, Some(1.0)), Status::Stressed);
        assert_eq!(move_status("SPX", 3.0, Some(1.0)), Status::Neutral);
        assert_eq!(move_status("BTC", -5.0, None), Status::Stressed);
        assert_eq!(move_status("BTC", -1.0, None), Status::Elevated);
    }

    #[test]
    fn routine_swings_in_a_volatile_row_do_not_outrank_a_rare_quiet_move() {
        let mut snapshot = model::fallback_snapshot();
        snapshot.histories.insert("VIX".into(), closes(6.0));
        snapshot.histories.insert("SPX".into(), closes(0.8));
        let vix = snapshot.histories["VIX"].last().unwrap().v;
        let spx = snapshot.histories["SPX"].last().unwrap().v;
        snapshot.quotes = vec![quote("VIX", vix, 7.0), quote("SPX", spx, -2.4)];
        stamp(&mut snapshot, "VIX", now());
        stamp(&mut snapshot, "SPX", now());
        let events = developments(&snapshot, None, now());
        // VIX +7% is about one typical day: not a development. SPX −2.4% is three.
        assert_eq!(events.len(), 1, "{events:?}");
        assert_eq!(events[0].id, "price:SPX");
        assert_eq!(events[0].status, Status::Stressed);
        assert!(events[0].detail.contains("× a typical day"));
        snapshot.quotes[0].change_pct = 25.0;
        let events = developments(&snapshot, None, now());
        let vix = events.iter().find(|e| e.id == "price:VIX").unwrap();
        assert_eq!(vix.status, Status::Stressed);
    }

    #[test]
    fn source_notices_rank_below_market_developments() {
        let mut snapshot = model::fallback_snapshot();
        snapshot.feeds.insert("tv-macro".into(), now() - 3 * 86_400);
        let baseline = VisitBaseline::from_snapshot(&snapshot, now() - 3 * 86_400);
        snapshot.notes.push("One feed failed.".into());
        snapshot.btc.price = Some(100.0);
        snapshot.btc.change_24h_pct = Some(-6.0);
        stamp(&mut snapshot, "BTC", now());
        let events = developments(&snapshot, Some(&baseline), now());
        assert_eq!(events[0].id, "price:BTC");
        assert!(events.iter().any(|e| e.id == "sources:stale"));
    }

    #[test]
    fn rule_changes_need_a_dated_reading_on_both_sides() {
        let obs = |date: &str, value: f64| {
            Some(model::Obs {
                date: date.into(),
                value,
                source: "U.S. Treasury".into(),
            })
        };
        let fallback = model::fallback_snapshot();
        let mut before = fallback.clone();
        before.dollar.ust30 = obs("2026-11-04", 4.9);
        let mut after = before.clone();
        after.dollar.ust30 = obs("2026-11-05", 5.1);
        let changes = rule_changes(&before, &after, now());
        let ust30 = changes.iter().find(|c| c.id == "ust30").unwrap();
        assert_eq!((ust30.from, ust30.to), (Status::Elevated, Status::Stressed));
        // First dated reading after the embedded fallback is not a crossing.
        assert!(!rule_changes(&fallback, &after, now())
            .iter()
            .any(|c| c.id == "ust30"));
        // Unchanged, or too old to act on.
        assert!(rule_changes(&after, &after, now()).is_empty());
        assert!(!rule_changes(&before, &after, now() + 10 * 86_400)
            .iter()
            .any(|c| c.id == "ust30"));
    }

    #[test]
    fn newly_stale_provider_is_a_dated_data_quality_change() {
        let mut snapshot = model::fallback_snapshot();
        snapshot.feeds.insert("tv-macro".into(), now() - 3 * 86_400);
        let baseline = VisitBaseline::from_snapshot(&snapshot, now() - 3 * 86_400);
        let events = developments(&snapshot, Some(&baseline), now());
        assert_eq!(events[0].id, "sources:stale");
        assert_eq!(events[0].target, Target::Sources);
        assert!(events[0].detail.contains("TradingView"));
    }
}
