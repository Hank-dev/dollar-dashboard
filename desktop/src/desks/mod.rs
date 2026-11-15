//! Research desks, the detail panel, and helpers they share.

mod ai;
mod bitcoin;
mod detail;
mod dollar;
mod markets;
mod nuclear;
mod overview;
mod pulse;
mod spacex;
mod zcash;

use crate::app::{Desk, Desktop};
use crate::charts::{self, Line, TimeChart};
use crate::theme::{mono, sans, BLUE, FAINT, GREEN, INPUT, RED};
use crate::widgets;
use dollar_dashboard::board::with_observation;
use dollar_dashboard::model::{
    self, fit_power_law, fmt_change_pct, now_unix, power_value, Point, Status, SECONDS_PER_DAY,
};
use eframe::egui::{Color32, RichText, TextEdit, Ui};

pub(crate) use detail::show as detail;

pub(crate) fn show(app: &mut Desktop, ui: &mut Ui) {
    match app.desk {
        Desk::Home => overview::show(app, ui),
        Desk::Btc => bitcoin::show(app, ui),
        Desk::Dollar => dollar::show(app, ui),
        Desk::Ai => ai::show(app, ui),
        Desk::Nuclear => nuclear::show(app, ui),
        Desk::Markets => markets::show(app, ui),
        Desk::Zcash => zcash::show(app, ui),
        Desk::SpaceX => spacex::show(app, ui),
        Desk::Pulse => pulse::show(app, ui),
    }
}

/// Research snapshots older than this are flagged as stale.
pub(crate) const RESEARCH_STALE_DAYS: i64 = 30;

/// Whole days since a YYYY-MM-DD evidence date.
pub(crate) fn evidence_days(as_of: &str, now: i64) -> Option<i64> {
    let date = chrono::NaiveDate::parse_from_str(as_of.get(0..10)?, "%Y-%m-%d").ok()?;
    let then = date.and_hms_opt(0, 0, 0)?.and_utc().timestamp();
    Some((now - then).div_euclid(86_400).max(0))
}

/// "Evidence 17 Oct 2026 · 3 days old" and the colour for that line.
pub(crate) fn evidence_line(as_of: &str, now: i64) -> (String, Color32) {
    let date = model::pretty_date(as_of);
    match evidence_days(as_of, now) {
        Some(days) if days > RESEARCH_STALE_DAYS => (
            format!("Evidence {date} · {days} days old, due for a refresh"),
            crate::theme::AMBER(),
        ),
        Some(0) => (format!("Evidence {date} · today"), FAINT()),
        Some(1) => (format!("Evidence {date} · 1 day old"), FAINT()),
        Some(days) => (format!("Evidence {date} · {days} days old"), FAINT()),
        None => (format!("Evidence {date}"), FAINT()),
    }
}

/// Number of bundled research snapshots past their refresh age.
pub(crate) fn stale_research(app: &Desktop, now: i64) -> usize {
    [
        &app.catalog.ai_as_of,
        &app.catalog.nuclear_as_of,
        &app.catalog.spacex_as_of,
    ]
        .iter()
        .filter(|as_of| evidence_days(as_of, now).is_some_and(|d| d > RESEARCH_STALE_DAYS))
        .count()
}

/// Desks whose rows open the detail panel.
pub(crate) fn has_detail(desk: Desk) -> bool {
    matches!(desk, Desk::Dollar | Desk::Ai | Desk::Nuclear | Desk::SpaceX)
}

/// Display name for a ticker in the instrument universe.
pub(crate) fn name_of(app: &Desktop, symbol: &str) -> String {
    let fixed = match symbol {
        "BTC" => Some("Bitcoin"),
        "ETH" => Some("Ether"),
        "ZEC" => Some("Zcash"),
        "STRC" => Some("Strategy STRC preferred"),
        "URA" => Some("Global X Uranium ETF"),
        "URNM" => Some("Sprott Uranium Miners ETF"),
        "NLR" => Some("VanEck Uranium & Nuclear ETF"),
        "SPCX" => Some("SpaceX / xAI"),
        _ => None,
    };
    if let Some(name) = fixed {
        return name.into();
    }
    app.catalog
        .ai_players
        .iter()
        .chain(&app.catalog.nuclear_players)
        .find(|p| p.ticker.as_deref() == Some(symbol))
        .map(|p| p.name.clone())
        .unwrap_or_default()
}

pub(crate) fn filter_box(ui: &mut Ui, filter: &mut String, hint: &str) {
    ui.horizontal(|ui| {
        ui.add(
            TextEdit::singleline(filter)
                .desired_width(220.0)
                .hint_text(hint)
                .background_color(INPUT()),
        );
        if !filter.is_empty() && widgets::link(ui, "Clear", FAINT()).clicked() {
            filter.clear();
        }
    });
}

pub(crate) fn matches(filter: &str, text: &str) -> bool {
    let filter = filter.trim();
    filter.is_empty() || text.to_lowercase().contains(&filter.to_lowercase())
}

pub(crate) fn funding_text(rate: Option<f64>) -> String {
    match rate {
        Some(_) => format!("{} / 8h", funding_bp(rate)),
        None => "—".into(),
    }
}

/// Funding per interval in basis points, without the interval.
pub(crate) fn funding_bp(rate: Option<f64>) -> String {
    rate.map(|r| model::with_minus(format!("{:+.2} bp", r * 10_000.0)))
        .unwrap_or_else(|| "—".into())
}

/// Signed whole number with a typographic minus and a bare zero.
pub(crate) fn signed_int(v: f64) -> String {
    if v.round() == 0.0 {
        "0".into()
    } else {
        model::with_minus(format!("{v:+.0}"))
    }
}

pub(crate) fn funding_status(rate: f64) -> Status {
    let abs = rate.abs();
    if abs >= 0.0005 {
        Status::Stressed
    } else if abs >= 0.0001 {
        Status::Elevated
    } else {
        Status::Calm
    }
}

pub(crate) fn z_status(z: f64) -> Status {
    let abs = z.abs();
    if abs >= 2.0 {
        Status::Stressed
    } else if abs >= 1.0 {
        Status::Elevated
    } else {
        Status::Neutral
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn research_turns_stale_after_thirty_days() {
        let now = chrono::NaiveDate::from_ymd_opt(2026, 10, 17)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap()
            .and_utc()
            .timestamp();
        assert_eq!(evidence_days("2026-10-17", now), Some(0));
        assert_eq!(evidence_days("2026-09-03", now), Some(44));
        let (fresh, fresh_tone) = evidence_line("2026-10-10", now);
        assert_eq!(fresh, "Evidence 10 Oct 2026 · 7 days old");
        assert_eq!(fresh_tone, FAINT());
        let (old, old_tone) = evidence_line("2026-09-03", now);
        assert!(old.contains("44 days old, due for a refresh"));
        assert_eq!(old_tone, crate::theme::AMBER());
        assert_eq!(evidence_days("not a date", now), None);
    }
}

pub(crate) struct PriceSpec<'a> {
    pub title: &'a str,
    pub symbol: &'a str,
    pub points: &'a [Point],
    pub live: Option<f64>,
    pub observed_unix: Option<i64>,
    pub color: Color32,
    pub fair_value: bool,
    pub source: &'a str,
}

const RANGES: &[(&str, i64)] = &[
    ("1M", 30),
    ("3M", 90),
    ("1Y", 365),
    ("3Y", 1095),
    ("All", 0),
];

/// Price history with range, log scale and (for Bitcoin) the power-law fair value.
pub(crate) fn price_panel(app: &mut Desktop, ui: &mut Ui, spec: PriceSpec, height: f32) {
    let now = now_unix();
    let series = with_observation(spec.points, spec.live, spec.observed_unix);
    let days = app.prefs.chart_days;
    let end = series.last().map(|p| p.t).unwrap_or(now);
    let start = if days == 0 {
        i64::MIN
    } else {
        end - days * SECONDS_PER_DAY
    };
    let visible: Vec<Point> = series.iter().filter(|p| p.t >= start).cloned().collect();
    let range = RANGES
        .iter()
        .find(|(_, d)| *d == days)
        .map(|(label, _)| *label)
        .unwrap_or("range");
    let change = match (visible.first(), visible.last()) {
        (Some(a), Some(b)) if a.v > 0.0 && visible.len() > 1 => Some((b.v / a.v - 1.0) * 100.0),
        _ => None,
    };
    let spot = spec
        .observed_unix
        .and_then(|t| chrono::DateTime::from_timestamp(t, 0))
        .map(|t| format!("spot as of {}", t.format("%-d %b %H:%M UTC")))
        .unwrap_or_else(|| "dated history · spot time unavailable".into());
    let meta = format!("{} daily closes · {spot}", spec.source);
    widgets::panel(ui, spec.title, &meta, |ui| {
        ui.horizontal(|ui| {
            widgets::segmented(ui, &mut app.prefs.chart_days, RANGES);
            ui.add_space(6.0);
            widgets::toggle(ui, &mut app.prefs.chart_log, "Log scale");
            if spec.fair_value {
                widgets::toggle(ui, &mut app.prefs.chart_fair, "Power-law fair value ±1σ");
            }
            if let Some(change) = change {
                ui.with_layout(
                    eframe::egui::Layout::right_to_left(eframe::egui::Align::Center),
                    |ui| {
                        ui.label(
                            RichText::new(format!("{range} change"))
                                .font(sans(11.5))
                                .color(FAINT()),
                        );
                        ui.label(
                            RichText::new(fmt_change_pct(change))
                                .font(mono(12.5))
                                .color(if change >= 0.0 { GREEN() } else { RED() }),
                        );
                    },
                );
            }
        });
        ui.add_space(6.0);
        let fit = (spec.fair_value && app.prefs.chart_fair)
            .then(|| fit_power_law(&series))
            .flatten();
        // The fair-value band only needs a few hundred samples to look smooth.
        let stride = (visible.len() / 400).max(1);
        let band = |sigmas: f64| -> Vec<Point> {
            fit.as_ref()
                .map(|fit| {
                    visible
                        .iter()
                        .step_by(stride)
                        .chain(visible.last())
                        .filter_map(|p| power_value(fit, p.t, sigmas).map(|v| Point { t: p.t, v }))
                        .collect()
                })
                .unwrap_or_default()
        };
        let (fair, upper, lower) = (band(0.0), band(1.0), band(-1.0));
        let axis = |v: f64| charts::axis_usd(v);
        let value = |v: f64| model::fmt_usd(v);
        let mut chart = TimeChart::new(height, &axis, &value)
            .log(app.prefs.chart_log)
            .line(
                Line::new(spec.symbol, &visible, spec.color)
                    .fill()
                    .width(1.8),
            );
        if fit.is_some() {
            chart = chart
                .line(Line::new("Fair", &fair, BLUE()).width(1.3))
                .line(Line::new("+1σ", &upper, BLUE().gamma_multiply(0.35)).width(1.0))
                .line(Line::new("−1σ", &lower, BLUE().gamma_multiply(0.35)).width(1.0));
        }
        charts::time_chart(ui, chart);
    });
}

pub(crate) struct LogLogSpec<'a> {
    pub symbol: &'a str,
    pub points: &'a [Point],
    pub live: Option<f64>,
    pub observed_unix: Option<i64>,
    pub color: Color32,
    /// Day zero of the x axis.
    pub genesis: i64,
    /// First date used in the fit.
    pub cutoff: i64,
    /// Human description of day zero, e.g. "genesis block, 3 Jan 2009".
    pub day_zero: &'a str,
    pub source: &'a str,
}

/// Log price against log days since genesis, with the OLS power-law line and ±1σ, ±2σ bands.
pub(crate) fn log_log_panel(ui: &mut Ui, spec: LogLogSpec, height: f32) {
    eframe::egui::CollapsingHeader::new(
        RichText::new("Log-log power-law research")
            .font(sans(12.5))
            .color(FAINT()),
    )
    .id_salt((spec.symbol, "log-log-research"))
    .default_open(false)
    .show(ui, |ui| log_log_chart(ui, spec, height));
}

fn log_log_chart(ui: &mut Ui, spec: LogLogSpec, height: f32) {
    let series = with_observation(spec.points, spec.live, spec.observed_unix);
    let meta = format!(
        "log price vs log days since {} · {}",
        spec.day_zero, spec.source
    );
    widgets::panel(ui, "Log-log power law", &meta, |ui| {
        let Some(fit) = model::fit_log_log(&series, spec.genesis, spec.cutoff) else {
            widgets::empty(ui, 80.0, "The fit needs at least 100 daily prices. Refresh sources to load the full history.");
            return;
        };
        let day = |t: i64| {
            ((t - spec.genesis) as f64 / SECONDS_PER_DAY as f64)
                .max(1.0)
                .log10()
        };
        let usable: Vec<&Point> = series
            .iter()
            .filter(|p| p.t > spec.genesis + SECONDS_PER_DAY)
            .collect();
        let (Some(first), Some(last)) = (
            usable.first().copied().cloned(),
            usable.last().copied().cloned(),
        ) else {
            return;
        };
        let (d0, d1) = (day(first.t), day(last.t));
        // A log-time axis stretches the early days and crushes the late ones, so thin the
        // series by position on that axis (about 1,200 buckets), not by a fixed day stride.
        let mut sampled: Vec<Point> = Vec::new();
        let mut bucket = -1_i64;
        for p in &usable {
            let b = ((day(p.t) - d0) / (d1 - d0).max(1e-9) * 1200.0) as i64;
            if b != bucket {
                sampled.push(**p);
                bucket = b;
            }
        }
        if sampled.last().map(|p| p.t) != Some(last.t) {
            sampled.push(last);
        }
        // Bands are straight lines in log-log space; sample them evenly in log days.
        let band = |sigmas: f64| -> Vec<Point> {
            (0..=160)
                .map(|i| {
                    let d = d0 + (d1 - d0) * i as f64 / 160.0;
                    spec.genesis + (10f64.powf(d) * SECONDS_PER_DAY as f64) as i64
                })
                .filter_map(|t| power_value(&fit, t, sigmas).map(|v| Point { t, v }))
                .collect()
        };
        let (fair, up1, lo1, up2, lo2) = (band(0.0), band(1.0), band(-1.0), band(2.0), band(-2.0));
        let axis = |v: f64| charts::axis_usd(v);
        let value = |v: f64| model::fmt_usd(v);
        charts::time_chart(
            ui,
            TimeChart::new(height, &axis, &value)
                .log(true)
                .log_time(spec.genesis)
                .line(Line::new(spec.symbol, &sampled, spec.color).width(1.4))
                .line(Line::new("Fair", &fair, BLUE()).width(1.4))
                .line(Line::new("+1σ", &up1, BLUE().gamma_multiply(0.45)).width(1.0))
                .line(Line::new("−1σ", &lo1, BLUE().gamma_multiply(0.45)).width(1.0))
                .line(Line::new("+2σ", &up2, BLUE().gamma_multiply(0.22)).width(1.0))
                .line(Line::new("−2σ", &lo2, BLUE().gamma_multiply(0.22)).width(1.0)),
        );
        ui.add_space(4.0);
        let z = model::power_z(&fit, last.t, last.v);
        let fair_now = power_value(&fit, last.t, 0.0);
        let days_now = (last.t - spec.genesis) / SECONDS_PER_DAY;
        let mut line = format!(
            "log10(price) = {:.3} {} {:.3} · log10(days) · slope {:.2} · residual σ {:.3} · n {}",
            fit.a,
            if fit.b < 0.0 { "−" } else { "+" },
            fit.b.abs(),
            fit.b,
            fit.sigma,
            fit.n
        );
        if let (Some(z), Some(f)) = (z, fair_now) {
            line.push_str(&format!(
                " · day {days_now}: fair {} · {}",
                model::fmt_usd(f),
                model::with_minus(format!("{z:+.2}σ"))
            ));
        }
        ui.label(
            RichText::new(model::with_minus(line))
                .font(mono(11.0))
                .color(FAINT()),
        );
        widgets::footnote(
            ui,
            "Ordinary least squares on log10 price against log10 days since day zero. A straight line here is a power law. It describes the past and makes no claim about the future.",
        );
    });
}
