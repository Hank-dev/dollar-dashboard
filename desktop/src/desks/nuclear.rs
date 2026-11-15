//! Nuclear energy desk: listed names and ETFs, uranium, and the dated fleet evidence.

use super::ai::{notes_panel, player_table};
use super::filter_box;
use crate::app::Desktop;
use crate::charts::{self, Line, TimeChart};
use crate::theme::{CYAN, FAINT, GAP, TEXT};
use crate::widgets::{self, Stat};
use dollar_dashboard::catalog::{NUCLEAR_TICKERS, URANIUM_ETFS};
use dollar_dashboard::model::{self, fmt_change_pct, quote, total_cap, weighted_change, Quote};
use eframe::egui::Ui;

pub(crate) fn show(app: &mut Desktop, ui: &mut Ui) {
    let quotes = app.snap.quotes.clone();
    let basket = match (
        total_cap(&quotes, NUCLEAR_TICKERS),
        weighted_change(&quotes, NUCLEAR_TICKERS),
    ) {
        (Some(cap), Some(change)) => format!(
            " · listed basket {} {}",
            model::fmt_cap(cap),
            fmt_change_pct(change)
        ),
        _ => String::new(),
    };
    let (evidence, tone) = super::evidence_line(&app.catalog.nuclear_as_of, model::now_unix());
    widgets::desk_header_tone(
        ui,
        "Nuclear energy",
        CYAN(),
        "Fleet, fuel, and hyperscaler demand",
        &format!("{evidence}{basket}"),
        tone,
    );
    widgets::research_summary(
        ui,
        "nuclear-research-note",
        CYAN(),
        &app.catalog.nuclear_verdict,
    );
    ui.add_space(GAP);
    let terminal = app.snap.terminal.clone();
    widgets::split(ui, 0.55, 1000.0, |ui, side| {
        if side == 0 {
            widgets::panel(
                ui,
                "Key readings",
                "delayed quotes · UxC uranium futures",
                |ui| {
                    widgets::stat_grid(ui, &stats(&quotes, &terminal), 130.0);
                },
            );
        } else {
            let as_of = model::pretty_date(&terminal.uranium_as_of);
            widgets::panel(
                ui,
                "IMF uranium index",
                &format!("monthly · latest {as_of} · trend only"),
                |ui| {
                    let axis = |v: f64| format!("${v:.0}");
                    let value = |v: f64| format!("${v:.2}/lb");
                    charts::time_chart(
                        ui,
                        TimeChart::new(150.0, &axis, &value)
                            .line(Line::new("IMF", &terminal.uranium_hist, CYAN()).fill()),
                    );
                    widgets::footnote(ui, "IMF global uranium price via FRED (PURANUSDM). It lags by about three months and runs well below UxC spot and futures, so read it for direction, not level.");
                },
            );
        }
    });
    ui.add_space(GAP);
    let players = app.catalog.nuclear_players.clone();
    let mut chosen = None;
    widgets::panel(
        ui,
        "Companies and projects",
        "click a row for the evidence",
        |ui| {
            filter_box(ui, &mut app.filter, "Filter name, ticker or category");
            ui.add_space(6.0);
            player_table(
                ui,
                &players,
                &quotes,
                "nuc",
                &app.filter,
                app.selected.as_deref(),
                &mut chosen,
            );
        },
    );
    if let Some(id) = chosen {
        app.selected = Some(id);
    }
    ui.add_space(GAP);
    widgets::split(ui, 0.5, 900.0, |ui, side| {
        if side == 0 {
            let notes = app.catalog.nuclear_metrics.clone();
            notes_panel(app, ui, "Operating facts", "nucm", &notes, true);
        } else {
            let notes = app.catalog.nuclear_signals.clone();
            notes_panel(app, ui, "What to watch", "nucs", &notes, false);
        }
    });
    widgets::footnote(
        ui,
        "Equity prices are delayed TradingView quotes. Reactor counts, HALEU status and project facts stay on the dated research snapshot.",
    );
}

fn stats(quotes: &[Quote], t: &model::Terminal) -> Vec<Stat> {
    let dash = || "—".to_string();
    let movers: Vec<(&str, f64)> = NUCLEAR_TICKERS
        .iter()
        .filter_map(|s| quote(quotes, s).map(|q| (*s, q.change_pct)))
        .collect();
    let best = movers.iter().copied().max_by(|a, b| a.1.total_cmp(&b.1));
    let worst = movers.iter().copied().min_by(|a, b| a.1.total_cmp(&b.1));
    let mut out = vec![
        Stat::new(
            "Listed cap",
            total_cap(quotes, NUCLEAR_TICKERS)
                .map(model::fmt_cap)
                .unwrap_or_else(dash),
        )
        .sub(
            format!("{} of {} names quoted", movers.len(), NUCLEAR_TICKERS.len()),
            FAINT(),
        ),
        Stat::new(
            "Basket today",
            weighted_change(quotes, NUCLEAR_TICKERS)
                .map(fmt_change_pct)
                .unwrap_or_else(dash),
        )
        .tone(crate::theme::move_color(weighted_change(
            quotes,
            NUCLEAR_TICKERS,
        )))
        .sub("cap-weighted", FAINT()),
        Stat::new(
            "Best mover",
            best.map(|(s, _)| s.to_string()).unwrap_or_else(dash),
        )
        .sub(
            best.map(|(_, c)| fmt_change_pct(c)).unwrap_or_default(),
            crate::theme::move_color(best.map(|(_, c)| c)),
        ),
        Stat::new(
            "Worst mover",
            worst.map(|(s, _)| s.to_string()).unwrap_or_else(dash),
        )
        .sub(
            worst.map(|(_, c)| fmt_change_pct(c)).unwrap_or_default(),
            crate::theme::move_color(worst.map(|(_, c)| c)),
        ),
        {
            let ux = quote(quotes, "URANIUM");
            Stat::new(
                "U3O8 futures",
                ux.map(|q| format!("${:.2}", q.price)).unwrap_or_else(dash),
            )
            .sub(
                ux.map(|q| format!("{} · UxC front month", fmt_change_pct(q.change_pct)))
                    .unwrap_or_else(|| "UxC feed unavailable".into()),
                crate::theme::move_color(ux.map(|q| q.change_pct)),
            )
            .hint("UxC U3O8 front-month futures (COMEX), delayed TradingView quote; tracks spot closely.")
        },
        Stat::new(
            "IMF monthly",
            t.uranium.map(|v| format!("${v:.2}")).unwrap_or_else(dash),
        )
        .sub(
            t.uranium_12m
                .map(|v| {
                    format!(
                        "{} 12m · {}",
                        fmt_change_pct(v),
                        model::pretty_date(&t.uranium_as_of)
                    )
                })
                .unwrap_or_default(),
            FAINT(),
        )
        .hint("IMF uranium price via FRED; a lagging monthly average below spot."),
    ];
    for symbol in URANIUM_ETFS {
        let q = quote(quotes, symbol);
        out.push(
            Stat::new(
                *symbol,
                q.map(|q| model::fmt_usd(q.price)).unwrap_or_else(dash),
            )
            .sub(
                q.map(|q| fmt_change_pct(q.change_pct)).unwrap_or_default(),
                crate::theme::move_color(q.map(|q| q.change_pct)),
            )
            .tone(TEXT()),
        );
    }
    out
}
