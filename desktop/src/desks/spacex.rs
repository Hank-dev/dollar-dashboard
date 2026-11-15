//! SpaceX desk: the SPCX listing (quote, history since the IPO, supply calendar)
//! and the dated research note on Starlink, launch and the AI segment.

use super::ai::notes_panel;
use super::{price_panel, PriceSpec};
use crate::app::Desktop;
use crate::theme::{move_color, BLUE, FAINT, GAP};
use crate::widgets::{self, Stat};
use dollar_dashboard::board::with_observation;
use dollar_dashboard::catalog::{SPACEX_IPO_PRICE, SPACEX_LOCKUP_END};
use dollar_dashboard::model::{self, change_since, fmt_change_pct, now_unix, quote, Point};
use eframe::egui::Ui;

pub(crate) fn show(app: &mut Desktop, ui: &mut Ui) {
    let now = now_unix();
    let q = quote(&app.snap.quotes, "SPCX").cloned();
    let observed = app.snap.price_observed_unix("SPCX");
    let history = app.snap.histories.get("SPCX").cloned().unwrap_or_default();
    let series = with_observation(&history, q.as_ref().map(|q| q.price), observed);

    let (evidence, tone) = super::evidence_line(&app.catalog.spacex_as_of, now);
    let listing = q
        .as_ref()
        .map(|q| {
            format!(
                " · SPCX {} {}",
                model::fmt_usd(q.price),
                fmt_change_pct(q.change_pct)
            )
        })
        .unwrap_or_default();
    widgets::desk_header_tone(
        ui,
        "SpaceX",
        BLUE(),
        "Launch, Starlink and the AI segment",
        &format!("{evidence}{listing}"),
        tone,
    );
    widgets::research_summary(
        ui,
        "spacex-research-note",
        BLUE(),
        &app.catalog.spacex_verdict,
    );
    ui.add_space(GAP);
    widgets::panel(
        ui,
        "Key readings",
        "delayed TradingView quote · CNBC daily closes since the Jun 12 listing",
        |ui| widgets::stat_grid(ui, &stats(q.as_ref(), &series, now), 130.0),
    );
    ui.add_space(GAP);
    price_panel(
        app,
        ui,
        PriceSpec {
            title: "SPCX since IPO",
            symbol: "SPCX",
            points: &history,
            live: q.as_ref().map(|q| q.price),
            observed_unix: observed,
            color: BLUE(),
            fair_value: false,
            source: "CNBC",
        },
        260.0,
    );
    ui.add_space(GAP);
    widgets::split(ui, 0.5, 900.0, |ui, side| {
        if side == 0 {
            let notes = app.catalog.spacex_metrics.clone();
            notes_panel(app, ui, "Operating facts", "spxm", &notes, true);
        } else {
            let notes = app.catalog.spacex_signals.clone();
            notes_panel(app, ui, "What to watch", "spxs", &notes, false);
        }
    });
    widgets::footnote(
        ui,
        "The SPCX price is a delayed TradingView quote. Results, subscriber counts and the lockup schedule come from the dated research snapshot; capex split and segment revenue are secondary reports until checked against the 10-Q.",
    );
}

fn stats(q: Option<&model::Quote>, series: &[Point], now: i64) -> Vec<Stat> {
    let dash = || "—".to_string();
    let price = q.map(|q| q.price);
    let high = series.iter().max_by(|a, b| a.v.total_cmp(&b.v));
    let low = series.iter().min_by(|a, b| a.v.total_cmp(&b.v));
    let date = |p: &Point| {
        chrono::DateTime::from_timestamp(p.t, 0)
            .map(|t| t.format("%-d %b").to_string())
            .unwrap_or_default()
    };
    let vs_ipo = price.map(|p| (p / SPACEX_IPO_PRICE - 1.0) * 100.0);
    let from_high = match (price, high) {
        (Some(p), Some(h)) if h.v > 0.0 => Some((p / h.v - 1.0) * 100.0),
        _ => None,
    };
    let month = change_since(series, 30);
    let lockup_days = chrono::NaiveDate::parse_from_str(SPACEX_LOCKUP_END, "%Y-%m-%d")
        .ok()
        .and_then(|d| d.and_hms_opt(0, 0, 0))
        .map(|d| (d.and_utc().timestamp() - now).div_euclid(86_400));
    vec![
        Stat::new("SPCX", price.map(model::fmt_usd).unwrap_or_else(dash))
            .sub(
                q.map(|q| format!("{} today", fmt_change_pct(q.change_pct)))
                    .unwrap_or_else(|| "quote unavailable".into()),
                move_color(q.map(|q| q.change_pct)),
            )
            .hint("Nasdaq SPCX, delayed TradingView quote."),
        Stat::new(
            "Market cap",
            q.and_then(|q| q.market_cap)
                .map(model::fmt_cap)
                .unwrap_or_else(dash),
        )
        .sub("TradingView, all share classes", FAINT()),
        Stat::new("vs IPO", vs_ipo.map(fmt_change_pct).unwrap_or_else(dash))
            .tone(move_color(vs_ipo))
            .sub("offer $135 · Jun 11", FAINT()),
        Stat::new(
            "Closing high",
            high.map(|p| model::fmt_usd(p.v)).unwrap_or_else(dash),
        )
        .sub(high.map(date).unwrap_or_default(), FAINT())
        .hint("Highest daily close since listing; intraday highs run above it."),
        Stat::new(
            "Closing low",
            low.map(|p| model::fmt_usd(p.v)).unwrap_or_else(dash),
        )
        .sub(low.map(date).unwrap_or_default(), FAINT()),
        Stat::new("From high", from_high.map(fmt_change_pct).unwrap_or_else(dash))
            .tone(move_color(from_high))
            .sub("drawdown from closing high", FAINT()),
        Stat::new("30 days", month.map(fmt_change_pct).unwrap_or_else(dash))
            .tone(move_color(month))
            .sub("daily closes", FAINT()),
        Stat::new(
            "Lockup ends",
            match lockup_days {
                Some(d) if d > 0 => format!("{d} days"),
                Some(d) if d > -2 => "now".into(),
                Some(_) => "passed".into(),
                None => dash(),
            },
        )
        .sub(
            format!("{} · 180-day", model::pretty_date(SPACEX_LOCKUP_END)),
            FAINT(),
        )
        .hint("Standard 180-day IPO lockup; trackers give Dec 8 or Dec 9. Most insider shares unlocked earlier in stages, and Musk and major holders stay locked until June 2027."),
    ]
}
