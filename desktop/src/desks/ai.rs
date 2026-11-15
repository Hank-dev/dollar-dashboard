//! AI infrastructure desk: frontier models, public and private names, and the
//! dated research snapshot.

use super::{filter_box, matches};
use crate::app::Desktop;
use crate::charts::{self, log_ticks, nice_ticks};
use crate::theme::{
    mono, sans, status_color, ACCENT, AMBER, BLUE, BORDER, CYAN, FAINT, GAP, GREEN, MUTED, PURPLE,
    RED, TEXT,
};
use crate::widgets::{self, caps_job, col, W};
use dollar_dashboard::catalog::{NoteCard, PlayerCard, AI_TICKERS};
use dollar_dashboard::model::{
    self, fmt_age, now_unix, quote, total_cap, weighted_change, Benchmark, Delta, ModelPrint,
    Quote, BENCHMARKS,
};
use eframe::egui::{Align, Align2, Color32, Pos2, Rect, Sense, Stroke, Ui, Vec2};

pub(crate) fn show(app: &mut Desktop, ui: &mut Ui) {
    let basket = match (
        total_cap(&app.snap.quotes, AI_TICKERS),
        weighted_change(&app.snap.quotes, AI_TICKERS),
    ) {
        (Some(cap), Some(change)) => format!(
            " · listed basket {} {}",
            model::fmt_cap(cap),
            model::fmt_change_pct(change)
        ),
        _ => String::new(),
    };
    let (evidence, tone) = super::evidence_line(&app.catalog.ai_as_of, now_unix());
    widgets::desk_header_tone(
        ui,
        "AI infrastructure",
        PURPLE(),
        "Compute, labs, and agents",
        &format!("{evidence}{basket}"),
        tone,
    );
    widgets::research_summary(ui, "ai-research-note", PURPLE(), &app.catalog.ai_verdict);
    ui.add_space(GAP);
    let models = app.snap.terminal.models.clone();
    let refreshed = app
        .snap
        .feeds
        .get("aa")
        .map(|t| format!("leaderboard {}", fmt_age(*t, now_unix())))
        .unwrap_or_else(|| "leaderboard not loaded yet".into());
    let pick_id = eframe::egui::Id::new("ai-benchmark");
    let mut pick: usize = ui.data_mut(|d| *d.get_persisted_mut_or_default(pick_id));
    pick = pick.min(BENCHMARKS.len() - 1);
    benchmark_picker(ui, &mut pick);
    let bench = &BENCHMARKS[pick];
    ui.add_space(6.0);
    widgets::split(ui, 0.5, 1000.0, |ui, side| {
        if side == 0 {
            widgets::panel(
                ui,
                &format!("{} vs. token cost", bench.short),
                &refreshed,
                |ui| {
                    scatter(ui, &models, bench);
                },
            );
        } else {
            widgets::panel(
                ui,
                "Frontier models",
                &format!("ranked by {} · {}", bench.name, bench.source),
                |ui| {
                    model_table(ui, &models, bench);
                },
            );
        }
    });
    ui.add_space(GAP);
    widgets::panel(
        ui,
        "Benchmark matrix",
        "Artificial Analysis · Epoch AI · Vals.ai · click a column to rank by it",
        |ui| {
            if let Some(col) = benchmark_matrix(ui, &models, pick) {
                pick = col;
            }
        },
    );
    ui.data_mut(|d| d.insert_persisted(pick_id, pick));
    ui.add_space(GAP);
    players(app, ui);
    ui.add_space(GAP);
    widgets::split(ui, 0.5, 900.0, |ui, side| {
        if side == 0 {
            let notes = app.catalog.ai_metrics.clone();
            notes_panel(app, ui, "Market structure", "aim", &notes, true);
        } else {
            let notes = app.catalog.ai_signals.clone();
            notes_panel(app, ui, "What to watch", "ais", &notes, false);
        }
    });
    widgets::footnote(
        ui,
        "Model scores and token prices: Artificial Analysis (OpenRouter prices as fallback). Equity quotes: TradingView, delayed. Company facts are dated research notes, not live data.",
    );
}

pub(crate) fn lab_color(lab: &str) -> Color32 {
    match lab {
        "Anthropic" => AMBER(),
        "OpenAI" => GREEN(),
        "Google" => BLUE(),
        "Meta" => CYAN(),
        "SpaceXAI" | "xAI" => RED(),
        _ => PURPLE(),
    }
}

/// One row of buttons choosing the benchmark that the scatter and the ranking use.
fn benchmark_picker(ui: &mut Ui, pick: &mut usize) {
    let options: Vec<(&str, usize)> = BENCHMARKS
        .iter()
        .enumerate()
        .map(|(i, b)| (b.short, i))
        .collect();
    ui.horizontal_wrapped(|ui| {
        ui.label(caps_job("Benchmark", 9.5, FAINT()));
        widgets::segmented(ui, pick, &options);
        let b = &BENCHMARKS[*pick];
        ui.label(
            eframe::egui::RichText::new(format!("{} · {} · {}", b.name, b.source, b.measures))
                .font(sans(11.5))
                .color(MUTED()),
        );
    });
}

/// Models with a score on `bench`, best first; models without one keep their order at the end.
fn ranked<'a>(models: &'a [ModelPrint], bench: &Benchmark) -> Vec<&'a ModelPrint> {
    let mut out: Vec<&ModelPrint> = models.iter().collect();
    out.sort_by(|a, b| match (a.score(bench.id), b.score(bench.id)) {
        (Some(x), Some(y)) => y.total_cmp(&x),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    });
    out
}

fn scatter(ui: &mut Ui, models: &[ModelPrint], bench: &Benchmark) {
    let scored: Vec<&ModelPrint> = models
        .iter()
        .filter(|m| m.blended.is_finite() && m.blended > 0.0 && m.score(bench.id).is_some())
        .collect();
    if scored.is_empty() {
        widgets::empty(
            ui,
            240.0,
            "No scores on this benchmark yet · refresh sources",
        );
        return;
    }
    let score = |m: &ModelPrint| m.score(bench.id).unwrap_or_default();
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 270.0), Sense::hover());
    let painter = ui.painter_at(rect);
    let plot = Rect::from_min_max(
        rect.min + Vec2::new(40.0, 8.0),
        rect.max - Vec2::new(12.0, 22.0),
    );
    let lo_c = scored
        .iter()
        .map(|m| m.blended)
        .fold(f64::INFINITY, f64::min)
        / 1.5;
    let hi_c = scored
        .iter()
        .map(|m| m.blended)
        .fold(f64::NEG_INFINITY, f64::max)
        * 1.5;
    let lo_raw = scored
        .iter()
        .map(|m| score(m))
        .fold(f64::INFINITY, f64::min);
    let hi_raw = scored
        .iter()
        .map(|m| score(m))
        .fold(f64::NEG_INFINITY, f64::max);
    let pad = ((hi_raw - lo_raw) * 0.12).max(match bench.scale {
        model::Scale::Index => 3.0,
        model::Scale::Share => 0.02,
        model::Scale::Elo => 20.0,
    });
    let (lo_i, hi_i) = (lo_raw - pad, hi_raw + pad);
    let x = |c: f64| {
        plot.left() + plot.width() * ((c.ln() - lo_c.ln()) / (hi_c.ln() - lo_c.ln())) as f32
    };
    let y = |v: f64| plot.bottom() - plot.height() * ((v - lo_i) / (hi_i - lo_i)) as f32;
    for v in nice_ticks(lo_i, hi_i, 4) {
        painter.hline(plot.x_range(), y(v), Stroke::new(1.0, BORDER()));
        painter.text(
            Pos2::new(plot.left() - 6.0, y(v)),
            Align2::RIGHT_CENTER,
            bench.format(v),
            mono(10.5),
            FAINT(),
        );
    }
    for c in log_ticks(lo_c, hi_c) {
        painter.vline(x(c), plot.y_range(), Stroke::new(1.0, BORDER()));
        painter.text(
            Pos2::new(x(c), plot.bottom() + 5.0),
            Align2::CENTER_TOP,
            charts::axis_usd(c),
            mono(10.5),
            FAINT(),
        );
    }
    // Labels avoid every dot as well as each other.
    let mut placed: Vec<Rect> = scored
        .iter()
        .map(|m| Rect::from_center_size(Pos2::new(x(m.blended), y(score(m))), Vec2::splat(9.0)))
        .collect();
    let mut hovered = None;
    for (i, m) in scored.iter().enumerate() {
        let p = Pos2::new(x(m.blended), y(score(m)));
        let color = lab_color(&m.creator);
        let near = response.hover_pos().is_some_and(|h| h.distance(p) < 9.0);
        if near {
            hovered = Some(i);
        }
        painter.circle_filled(p, if near { 6.0 } else { 4.5 }, color);
        let short = m.label.split(" (").next().unwrap_or(&m.label);
        let galley = painter.layout_no_wrap(short.to_string(), sans(10.5), MUTED());
        let size = galley.size();
        let spot = [
            Pos2::new(p.x + 8.0, p.y - size.y / 2.0),
            Pos2::new(p.x - 8.0 - size.x, p.y - size.y / 2.0),
            Pos2::new(p.x - size.x / 2.0, p.y - 8.0 - size.y),
            Pos2::new(p.x - size.x / 2.0, p.y + 7.0),
        ]
        .into_iter()
        .map(|pos| Rect::from_min_size(pos, size))
        .find(|r| plot.contains_rect(*r) && placed.iter().all(|o| !o.expand(2.0).intersects(*r)));
        if let Some(r) = spot {
            placed.push(r);
            painter.galley(r.min, galley, MUTED());
        }
    }
    if let Some(i) = hovered {
        let m = scored[i];
        let p = Pos2::new(x(m.blended), y(score(m)));
        charts::tooltip(
            &painter,
            plot,
            p,
            &[
                (format!("{} · {}", m.label, m.creator), FAINT()),
                (
                    format!("{} {}", bench.short, bench.format(score(m))),
                    TEXT(),
                ),
                (
                    match (m.scores.get("epoch-eci-lo"), m.scores.get("epoch-eci-hi")) {
                        (Some(lo), Some(hi)) if bench.id == "epoch-eci" => {
                            format!("CI {lo:.1}–{hi:.1}")
                        }
                        _ => bench.source.to_string(),
                    },
                    FAINT(),
                ),
                (format!("${:.2} per 1M blended", m.blended), MUTED()),
                (
                    format!("${:.2} in · ${:.2} out", m.input, m.output),
                    MUTED(),
                ),
            ],
        );
    }
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 12.0;
        let mut labs: Vec<&str> = Vec::new();
        for m in &scored {
            if !labs.contains(&m.creator.as_str()) {
                labs.push(&m.creator);
            }
        }
        for lab in labs {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                let (r, _) = ui.allocate_exact_size(Vec2::new(8.0, 12.0), Sense::hover());
                ui.painter().circle_filled(r.center(), 3.5, lab_color(lab));
                ui.label(
                    eframe::egui::RichText::new(lab)
                        .font(sans(11.0))
                        .color(MUTED()),
                );
            });
        }
    });
    widgets::footnote(
        ui,
        "Cost axis is logarithmic: USD per million tokens, blended 3 input : 1 output.",
    );
}

fn model_table(ui: &mut Ui, models: &[ModelPrint], bench: &Benchmark) {
    if models.is_empty() {
        widgets::empty(ui, 120.0, "Scores and token prices load on refresh.");
        return;
    }
    let wide = ui.available_width() >= 520.0;
    let mut cols = vec![
        col("#", W::Px(16.0), Align::Max),
        col("Model", W::Flex(1.0), Align::Min),
        col(bench.short, W::Px(118.0), Align::Min),
        col("Blended", W::Px(64.0), Align::Max),
    ];
    if wide {
        cols.insert(2, col("Lab", W::Px(78.0), Align::Min));
        cols.push(col("In / out", W::Px(84.0), Align::Max));
    }
    let (layout, _) = widgets::table_header(ui, &cols, None);
    let top = models
        .iter()
        .filter_map(|m| m.score(bench.id))
        .fold(f64::NEG_INFINITY, f64::max);
    let floor = models
        .iter()
        .filter_map(|m| m.score(bench.id))
        .fold(f64::INFINITY, f64::min);
    // Elo has no zero, so its bars run from the weakest model; the rest from zero.
    let share = |v: f64| -> f32 {
        let f = match bench.scale {
            model::Scale::Elo if top > floor => 0.15 + 0.85 * (v - floor) / (top - floor),
            _ if top > 0.0 => v / top,
            _ => 0.0,
        };
        f.clamp(0.0, 1.0) as f32
    };
    let money = |v: f64| {
        if v < 1.0 {
            format!("${v:.2}")
        } else {
            format!("${v:.2}")
                .trim_end_matches('0')
                .trim_end_matches('.')
                .to_string()
        }
    };
    for (i, m) in ranked(models, bench).into_iter().enumerate() {
        let (rect, _) = widgets::table_row(ui, 26.0, false, false);
        let painter = ui.painter();
        let mut c = 0;
        widgets::cell_text(
            painter,
            layout.cell(rect, c),
            &(i + 1).to_string(),
            mono(11.0),
            FAINT(),
            Align::Max,
        );
        c += 1;
        let (name, effort) = m
            .label
            .split_once(" (")
            .map(|(name, rest)| (name, rest.trim_end_matches(')')))
            .unwrap_or((m.label.as_str(), ""));
        let effort = if effort.is_empty() {
            String::new()
        } else {
            format!("  {}", effort.to_lowercase())
        };
        widgets::cell_runs(
            painter,
            layout.cell(rect, c),
            &[(name, sans(12.5), TEXT()), (&effort, sans(11.0), FAINT())],
            Align::Min,
        );
        if wide {
            c += 1;
            widgets::cell_text(
                painter,
                layout.cell(rect, c),
                if m.creator.is_empty() {
                    "—"
                } else {
                    &m.creator
                },
                sans(12.0),
                lab_color(&m.creator),
                Align::Min,
            );
        }
        c += 1;
        let cell = layout.cell(rect, c);
        match m.score(bench.id) {
            Some(v) => {
                let text_w = 38.0;
                widgets::cell_text(
                    painter,
                    Rect::from_min_size(cell.min, Vec2::new(text_w, cell.height())),
                    &bench.format(v),
                    mono(12.0),
                    TEXT(),
                    Align::Min,
                );
                let track = Rect::from_min_size(
                    Pos2::new(cell.left() + text_w + 4.0, cell.center().y - 2.5),
                    Vec2::new(cell.width() - text_w - 4.0, 5.0),
                );
                painter.rect_filled(track, crate::theme::corner(2), BORDER());
                let fill = share(v);
                painter.rect_filled(
                    Rect::from_min_size(track.min, Vec2::new(track.width() * fill, 5.0)),
                    crate::theme::corner(2),
                    lab_color(&m.creator).gamma_multiply(0.8),
                );
            }
            None => widgets::cell_text(painter, cell, "—", mono(12.0), FAINT(), Align::Min),
        }
        c += 1;
        widgets::cell_text(
            painter,
            layout.cell(rect, c),
            &money(m.blended),
            mono(12.0),
            TEXT(),
            Align::Max,
        );
        if wide {
            c += 1;
            widgets::cell_text(
                painter,
                layout.cell(rect, c),
                &format!("{} / {}", money(m.input), money(m.output)),
                mono(11.0),
                MUTED(),
                Align::Max,
            );
        }
    }
    widgets::footnote(
        ui,
        "Best effort setting per model, at most three per lab. USD per million tokens.",
    );
}

/// Every benchmark side by side, ranked by the chosen one. Shading is a model's place between
/// the column's weakest and strongest score; the column leader is in the accent colour.
/// Returns the benchmark index when a column header is clicked.
fn benchmark_matrix(ui: &mut Ui, models: &[ModelPrint], pick: usize) -> Option<usize> {
    if models.is_empty() {
        widgets::empty(ui, 120.0, "Benchmark scores load on refresh.");
        return None;
    }
    let wide = ui.available_width() >= 1000.0;
    let mut cols = vec![col("Model", W::Flex(1.0), Align::Min)];
    if wide {
        cols.push(col("Lab", W::Px(70.0), Align::Min));
    }
    let first = cols.len();
    for b in BENCHMARKS {
        cols.push(col(
            b.short,
            W::Px(if wide { 58.0 } else { 50.0 }),
            Align::Max,
        ));
    }
    cols.push(col("$/1M", W::Px(54.0), Align::Max));
    let (layout, clicked) = widgets::table_header(ui, &cols, Some((first + pick, true)));
    let ranges: Vec<Option<(f64, f64)>> = BENCHMARKS
        .iter()
        .map(|b| {
            let v: Vec<f64> = models.iter().filter_map(|m| m.score(b.id)).collect();
            (!v.is_empty()).then(|| {
                (
                    v.iter().copied().fold(f64::INFINITY, f64::min),
                    v.iter().copied().fold(f64::NEG_INFINITY, f64::max),
                )
            })
        })
        .collect();
    for m in ranked(models, &BENCHMARKS[pick]) {
        let (rect, _) = widgets::table_row(ui, 24.0, false, false);
        let painter = ui.painter();
        let (name, effort) = m
            .label
            .split_once(" (")
            .map(|(name, rest)| (name, rest.trim_end_matches(')')))
            .unwrap_or((m.label.as_str(), ""));
        let effort = if effort.is_empty() {
            String::new()
        } else {
            format!("  {}", effort.to_lowercase())
        };
        widgets::cell_runs(
            painter,
            layout.cell(rect, 0),
            &[(name, sans(12.0), TEXT()), (&effort, sans(10.5), FAINT())],
            Align::Min,
        );
        if wide {
            let lab = if m.creator.is_empty() {
                "—"
            } else {
                m.creator.as_str()
            };
            widgets::cell_text(
                painter,
                layout.cell(rect, 1),
                lab,
                sans(11.5),
                lab_color(&m.creator),
                Align::Min,
            );
        }
        for (j, b) in BENCHMARKS.iter().enumerate() {
            let cell = layout.cell(rect, first + j);
            match (m.score(b.id), ranges[j]) {
                (Some(v), Some((lo, hi))) => {
                    let t = if hi > lo {
                        ((v - lo) / (hi - lo)) as f32
                    } else {
                        1.0
                    };
                    painter.rect_filled(
                        Rect::from_min_max(
                            Pos2::new(cell.left() + 6.0, cell.top() + 3.0),
                            Pos2::new(cell.right() + 4.0, cell.bottom() - 3.0),
                        ),
                        crate::theme::corner(2),
                        ACCENT().gamma_multiply(0.05 + 0.22 * t),
                    );
                    let color = if v >= hi {
                        ACCENT()
                    } else if j == pick {
                        TEXT()
                    } else {
                        MUTED()
                    };
                    widgets::cell_text(painter, cell, &b.format(v), mono(11.5), color, Align::Max);
                }
                _ => widgets::cell_text(painter, cell, "—", mono(11.5), FAINT(), Align::Max),
            }
        }
        widgets::cell_text(
            painter,
            layout.cell(rect, first + BENCHMARKS.len()),
            &format!("${:.2}", m.blended),
            mono(11.5),
            MUTED(),
            Align::Max,
        );
    }
    widgets::footnote(
        ui,
        "AA columns score each model's best effort setting on the Intelligence Index. ECI (Epoch AI) and Vals (Vals.ai) are matched by model name, so models they have not rated show a gap. Percentages are accuracy; GDPval is an Elo rating. $/1M is the blended token price (3 input : 1 output).",
    );
    clicked
        .and_then(|c| c.checked_sub(first))
        .filter(|j| *j < BENCHMARKS.len())
}

fn players(app: &mut Desktop, ui: &mut Ui) {
    let players = app.catalog.ai_players.clone();
    let mut chosen = None;
    widgets::panel(
        ui,
        "Public and private names",
        "click a row for the evidence",
        |ui| {
            filter_box(ui, &mut app.filter, "Filter name, ticker or category");
            ui.add_space(6.0);
            player_table(
                ui,
                &players,
                &app.snap.quotes,
                "ai",
                &app.filter,
                app.selected.as_deref(),
                &mut chosen,
            );
        },
    );
    if let Some(id) = chosen {
        app.selected = Some(id);
    }
}

/// Shared with the nuclear desk.
pub(crate) fn player_table(
    ui: &mut Ui,
    players: &[PlayerCard],
    quotes: &[Quote],
    prefix: &str,
    filter: &str,
    selected: Option<&str>,
    chosen: &mut Option<String>,
) {
    let wide = ui.available_width() >= 760.0;
    let mut cols = vec![
        col("", W::Px(8.0), Align::Min),
        col("Name", W::Flex(1.0), Align::Min),
        col("Ticker", W::Px(56.0), Align::Min),
        col("Last", W::Px(84.0), Align::Max),
        col("Day", W::Px(64.0), Align::Max),
        col("Scale", W::Px(if wide { 150.0 } else { 96.0 }), Align::Max),
    ];
    if wide {
        cols.push(col("Role", W::Flex(1.0), Align::Min));
    }
    let (layout, _) = widgets::table_header(ui, &cols, None);
    let mut shown = 0;
    for p in players {
        let text = format!(
            "{} {} {}",
            p.name,
            p.category,
            p.ticker.as_deref().unwrap_or("")
        );
        if !matches(filter, &text) {
            continue;
        }
        shown += 1;
        let id = format!("{prefix}:{}", p.id);
        let (rect, response) = widgets::table_row(ui, 27.0, selected == Some(id.as_str()), true);
        let painter = ui.painter();
        widgets::dot(
            painter,
            layout.cell(rect, 0).center(),
            status_color(p.status),
        );
        widgets::cell_text(
            painter,
            layout.cell(rect, 1),
            &p.name,
            sans(12.5),
            TEXT(),
            Align::Min,
        );
        let q = p.ticker.as_deref().and_then(|t| quote(quotes, t));
        widgets::cell_text(
            painter,
            layout.cell(rect, 2),
            p.ticker.as_deref().unwrap_or(&p.kind),
            mono(11.5),
            if p.ticker.is_some() { TEXT() } else { FAINT() },
            Align::Min,
        );
        match q {
            Some(q) => {
                widgets::cell_text(
                    painter,
                    layout.cell(rect, 3),
                    &model::fmt_usd(q.price),
                    mono(12.0),
                    TEXT(),
                    Align::Max,
                );
                let (day, color) = widgets::delta(Some(Delta::Pct(q.change_pct)));
                widgets::cell_text(
                    painter,
                    layout.cell(rect, 4),
                    &day,
                    mono(11.5),
                    color,
                    Align::Max,
                );
            }
            None => {
                widgets::cell_text(
                    painter,
                    layout.cell(rect, 3),
                    "—",
                    mono(12.0),
                    FAINT(),
                    Align::Max,
                );
            }
        }
        let scale = q
            .and_then(|q| q.market_cap)
            .map(model::fmt_cap)
            .unwrap_or_else(|| p.scale.clone());
        widgets::cell_text(
            painter,
            layout.cell(rect, 5),
            &scale,
            mono(11.5),
            MUTED(),
            Align::Max,
        );
        if wide {
            widgets::cell_text(
                painter,
                layout.cell(rect, 6),
                &p.category,
                sans(11.5),
                FAINT(),
                Align::Min,
            );
        }
        if response.clicked() {
            *chosen = Some(id);
        }
    }
    if shown == 0 {
        widgets::empty(ui, 40.0, "No names match the filter.");
    }
}

/// Dated research notes; shared with the nuclear desk.
pub(crate) fn notes_panel(
    app: &mut Desktop,
    ui: &mut Ui,
    title: &str,
    prefix: &str,
    notes: &[NoteCard],
    with_value: bool,
) {
    let mut chosen = None;
    let as_of = notes
        .iter()
        .map(|n| n.as_of.as_str())
        .max()
        .map(model::pretty_date)
        .unwrap_or_default();
    widgets::panel(ui, title, &format!("dated {as_of}"), |ui| {
        for n in notes {
            if !matches(&app.filter, &format!("{} {} {}", n.label, n.value, n.group)) {
                continue;
            }
            let id = format!("{prefix}:{}", n.id);
            let selected = app.selected.as_deref() == Some(id.as_str());
            let (rect, response) = widgets::table_row(ui, 40.0, selected, true);
            let painter = ui.painter();
            let inner = rect.shrink2(Vec2::new(8.0, 4.0));
            widgets::dot(
                painter,
                Pos2::new(inner.left() + 3.0, inner.top() + 9.0),
                status_color(n.status),
            );
            let value_w = if with_value && !n.value.is_empty() {
                120.0
            } else {
                0.0
            };
            let top = Rect::from_min_size(
                inner.left_top() + Vec2::new(14.0, 0.0),
                Vec2::new(inner.width() - 14.0 - value_w, 18.0),
            );
            widgets::cell_text(painter, top, &n.label, sans(12.5), TEXT(), Align::Min);
            let sub = if n.context.is_empty() {
                &n.detail
            } else {
                &n.context
            };
            let bottom = Rect::from_min_size(
                inner.left_top() + Vec2::new(14.0, 17.0),
                Vec2::new(inner.width() - 14.0 - value_w, 15.0),
            );
            widgets::cell_text(painter, bottom, sub, sans(11.0), FAINT(), Align::Min);
            if value_w > 0.0 {
                let value_rect = Rect::from_min_size(
                    Pos2::new(inner.right() - value_w, inner.top()),
                    Vec2::new(value_w, 18.0),
                );
                widgets::cell_text(
                    painter,
                    value_rect,
                    &n.value,
                    mono(12.0),
                    TEXT(),
                    Align::Max,
                );
            } else {
                let tag = painter.layout_job(caps_job(&n.group, 9.0, FAINT()));
                let pos = Pos2::new(inner.right() - tag.size().x, inner.top() + 3.0);
                painter.galley(pos, tag, FAINT());
            }
            if response.clicked() {
                chosen = Some(id);
            }
        }
    });
    if let Some(id) = chosen {
        app.selected = Some(id);
    }
}
