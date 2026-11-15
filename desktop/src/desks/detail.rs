//! Detail panel for a selected metric, company or research note.

use crate::app::Desktop;
use crate::theme::{medium, mono, sans, status_color, FAINT, MUTED, TEXT};
use crate::widgets::{self, caps_job};
use dollar_dashboard::catalog::{NoteCard, PlayerCard};
use dollar_dashboard::model::{self, dollar_metrics, quote, Delta, Metric, Quote};
use eframe::egui::{RichText, ScrollArea, Ui};

pub(crate) fn show(app: &Desktop, ui: &mut Ui) {
    let Some(id) = app.selected.clone() else {
        return;
    };
    ScrollArea::vertical()
        .id_salt("detail")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            if let Some(metric) = dollar_metrics(&app.snap).into_iter().find(|m| m.id == id) {
                metric_detail(ui, &metric);
                return;
            }
            let catalog = &app.catalog;
            let (prefix, rest) = id.split_once(':').unwrap_or(("", ""));
            let player = match prefix {
                "ai" => catalog.ai_players.iter().find(|p| p.id == rest),
                "nuc" => catalog.nuclear_players.iter().find(|p| p.id == rest),
                _ => None,
            };
            if let Some(player) = player {
                player_detail(ui, player, &app.snap.quotes);
                return;
            }
            let note = match prefix {
                "aim" => catalog.ai_metrics.iter().find(|n| n.id == rest),
                "ais" => catalog.ai_signals.iter().find(|n| n.id == rest),
                "nucm" => catalog.nuclear_metrics.iter().find(|n| n.id == rest),
                "nucs" => catalog.nuclear_signals.iter().find(|n| n.id == rest),
                "spxm" => catalog.spacex_metrics.iter().find(|n| n.id == rest),
                "spxs" => catalog.spacex_signals.iter().find(|n| n.id == rest),
                _ => None,
            };
            match note {
                Some(note) => note_detail(ui, note),
                None => {
                    ui.label(
                        RichText::new("Nothing selected.")
                            .font(sans(12.5))
                            .color(FAINT()),
                    );
                }
            }
        });
}

fn heading(ui: &mut Ui, status: model::Status, kicker: &str, title: &str) {
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(
            eframe::egui::Vec2::new(8.0, 12.0),
            eframe::egui::Sense::hover(),
        );
        widgets::dot(ui.painter(), rect.center(), status_color(status));
        ui.label(caps_job(
            &format!("{} · {kicker}", status.label()),
            10.0,
            status_color(status),
        ));
    });
    ui.add_space(2.0);
    ui.label(RichText::new(title).font(medium(17.0)).color(TEXT()));
}

fn block(ui: &mut Ui, label: &str, body: &str) {
    if body.is_empty() {
        return;
    }
    ui.add_space(10.0);
    ui.label(caps_job(label, 9.5, FAINT()));
    ui.add_space(2.0);
    ui.label(RichText::new(body).font(sans(13.0)).color(TEXT()));
}

fn source(ui: &mut Ui, text: &str) {
    ui.add_space(12.0);
    ui.label(RichText::new(text).font(mono(10.5)).color(FAINT()));
}

fn metric_detail(ui: &mut Ui, metric: &Metric) {
    heading(ui, metric.status, metric.group.title(), metric.label);
    ui.horizontal(|ui| {
        ui.label(RichText::new(&metric.value).font(mono(24.0)).color(TEXT()));
        if let Some(change) = metric.change {
            let (text, color) = widgets::delta(Some(change));
            ui.label(
                RichText::new(format!("{text} today"))
                    .font(mono(12.5))
                    .color(color),
            );
        }
    });
    block(ui, "Rule", metric.rule);
    block(ui, "Context", &metric.context);
    if !metric.live {
        block(
            ui,
            "Freshness",
            "This is a dated fact from the research snapshot, not a live feed.",
        );
    }
}

fn player_detail(ui: &mut Ui, player: &PlayerCard, quotes: &[Quote]) {
    heading(ui, player.status, &player.category, &player.name);
    let q = player.ticker.as_deref().and_then(|t| quote(quotes, t));
    ui.horizontal(|ui| {
        let scale = q
            .and_then(|q| q.market_cap)
            .map(model::fmt_cap)
            .unwrap_or_else(|| player.scale.clone());
        ui.label(RichText::new(scale).font(mono(22.0)).color(TEXT()));
        ui.label(
            RichText::new(if q.and_then(|q| q.market_cap).is_some() {
                "market cap"
            } else {
                "snapshot scale"
            })
            .font(sans(11.0))
            .color(FAINT()),
        );
    });
    if let (Some(ticker), Some(q)) = (&player.ticker, q) {
        let (day, color) = widgets::delta(Some(Delta::Pct(q.change_pct)));
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!("{ticker} {}", model::fmt_usd(q.price)))
                    .font(mono(13.0))
                    .color(MUTED()),
            );
            ui.label(RichText::new(day).font(mono(12.5)).color(color));
        });
    }
    block(ui, "Role", &player.role);
    block(ui, "Exposure", &player.exposure);
    source(
        ui,
        &format!("{} · {}", player.source, model::pretty_date(&player.as_of)),
    );
}

fn note_detail(ui: &mut Ui, note: &NoteCard) {
    heading(ui, note.status, &note.group, &note.label);
    if !note.value.is_empty() {
        ui.label(RichText::new(&note.value).font(mono(22.0)).color(TEXT()));
    }
    if !note.context.is_empty() {
        ui.label(RichText::new(&note.context).font(sans(12.5)).color(MUTED()));
    }
    block(ui, "Evidence", &note.detail);
    block(ui, "Watch next", &note.watch);
    source(
        ui,
        &format!("{} · {}", note.source, model::pretty_date(&note.as_of)),
    );
}
