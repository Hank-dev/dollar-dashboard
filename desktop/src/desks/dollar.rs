//! Dollar and macro desk: rule-based stress readings, the Treasury curve,
//! the dollar index and the macro backdrop.

use crate::app::Desktop;
use crate::charts::{self, nice_ticks, Line, TimeChart};
use crate::theme::{
    mono, sans, status_color, AMBER, BLUE, BORDER, BORDER_STRONG, FAINT, GAP, MUTED, TEXT,
};
use crate::widgets::{self, caps_job, col, W};
use dollar_dashboard::board::with_observation;
use dollar_dashboard::model::{
    self, dollar_metrics, fmt_change_bps, now_unix, quote, CurveDay, Delta, Group, Status,
    Terminal, STANDING_NOTE,
};
use eframe::egui::{Align, Align2, Id, Pos2, Rect, Sense, Shape, Stroke, Ui, Vec2};

pub(crate) fn show(app: &mut Desktop, ui: &mut Ui) {
    let metrics = dollar_metrics(&app.snap);
    let count = |s: Status| metrics.iter().filter(|m| m.status == s).count();
    widgets::desk_header(
        ui,
        "Macro & rates",
        BLUE(),
        "Dollar and the global funding system",
        &format!(
            "Treasury close {} · {} stressed · {} elevated · {} rules",
            model::pretty_date(&app.snap.dollar.curve_date),
            count(Status::Stressed),
            count(Status::Elevated),
            metrics.len()
        ),
    );
    widgets::quote_block(ui, BLUE(), "Rule reading", &model::rule_reading(&metrics));
    ui.add_space(GAP);
    widgets::split(ui, 0.56, 1080.0, |ui, side| {
        if side == 0 {
            rules_table(app, ui, &metrics);
        } else {
            yield_curve(app, ui);
            ui.add_space(GAP);
            dollar_index(app, ui);
        }
    });
    ui.add_space(GAP);
    macro_table(
        ui,
        &app.snap.terminal,
        "Macro backdrop",
        "a rising print is risk-off",
    );
    widgets::footnote(ui, STANDING_NOTE);
    widgets::footnote(
        ui,
        "Sources: U.S. Treasury daily par curve, FRED (target range, broad dollar, real yield, HY OAS), CBOE, TradingView delayed quotes, CNBC daily closes, Gold API, Binance. Status bands are transparent rules of thumb, not signals.",
    );
}

fn rules_table(app: &mut Desktop, ui: &mut Ui, metrics: &[model::Metric]) {
    let mut chosen = None;
    widgets::panel(
        ui,
        "Stress rules",
        "click a row for the rule and source",
        |ui| {
            let wide = ui.available_width() >= 560.0;
            let mut cols = vec![
                col("", W::Px(8.0), Align::Min),
                col("Metric", W::Flex(1.0), Align::Min),
                col("Value", W::Px(92.0), Align::Max),
                col("Day", W::Px(68.0), Align::Max),
                col("State", W::Px(64.0), Align::Min),
            ];
            if wide {
                cols.push(col("Context", W::Flex(1.1), Align::Min));
            }
            let (layout, _) = widgets::table_header(ui, &cols, None);
            for group in [Group::Rates, Group::Dollar, Group::Risk] {
                widgets::table_group(ui, group.title());
                for m in metrics.iter().filter(|m| m.group == group) {
                    let selected = app.selected.as_deref() == Some(m.id);
                    let (rect, response) = widgets::table_row(ui, 27.0, selected, true);
                    let painter = ui.painter();
                    let tone = status_color(m.status);
                    widgets::dot(painter, layout.cell(rect, 0).center(), tone);
                    widgets::cell_text(
                        painter,
                        layout.cell(rect, 1),
                        m.label,
                        sans(12.5),
                        TEXT(),
                        Align::Min,
                    );
                    widgets::cell_text(
                        painter,
                        layout.cell(rect, 2),
                        &m.value,
                        mono(12.5),
                        TEXT(),
                        Align::Max,
                    );
                    let (day, color) = widgets::delta(m.change);
                    widgets::cell_text(
                        painter,
                        layout.cell(rect, 3),
                        &day,
                        mono(11.5),
                        color,
                        Align::Max,
                    );
                    widgets::cell_text(
                        painter,
                        layout.cell(rect, 4),
                        m.status.label(),
                        sans(11.5),
                        tone,
                        Align::Min,
                    );
                    if wide {
                        widgets::cell_text(
                            painter,
                            layout.cell(rect, 5),
                            &m.context,
                            sans(11.5),
                            FAINT(),
                            Align::Min,
                        );
                    }
                    if response.on_hover_text(m.rule).clicked() {
                        chosen = Some(m.id.to_string());
                    }
                }
            }
            ui.add_space(8.0);
            legend(ui);
        },
    );
    if let Some(id) = chosen {
        app.selected = Some(id);
    }
}

fn legend(ui: &mut Ui) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 14.0;
        for status in [
            Status::Calm,
            Status::Neutral,
            Status::Elevated,
            Status::Stressed,
        ] {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 5.0;
                let (rect, _) = ui.allocate_exact_size(Vec2::new(6.0, 12.0), Sense::hover());
                widgets::dot(ui.painter(), rect.center(), status_color(status));
                ui.label(
                    eframe::egui::RichText::new(status.label())
                        .font(sans(11.0))
                        .color(MUTED()),
                );
            });
        }
    });
}

fn yield_curve(app: &Desktop, ui: &mut Ui) {
    let book = &app.snap.dollar;
    let id = Id::new("curve-compare");
    let mut days = ui.ctx().data_mut(|d| d.get_temp::<i64>(id)).unwrap_or(30);
    let target = chrono::NaiveDate::parse_from_str(&book.curve_date, "%Y-%m-%d")
        .ok()
        .map(|d| (d - chrono::Duration::days(days)).to_string());
    let past: Option<&CurveDay> = target
        .as_ref()
        .and_then(|date| book.curve_history.iter().rev().find(|d| d.date <= *date));
    let prev: Option<&CurveDay> = book
        .curve_history
        .iter()
        .rev()
        .find(|d| d.date < book.curve_date);
    let meta = match past {
        Some(p) => format!(
            "{} vs {}",
            model::pretty_date(&book.curve_date),
            model::pretty_date(&p.date)
        ),
        None => model::pretty_date(&book.curve_date),
    };
    widgets::panel(ui, "Treasury yield curve", &meta, |ui| {
        ui.horizontal(|ui| {
            ui.label(
                eframe::egui::RichText::new("Compare with")
                    .font(sans(11.5))
                    .color(FAINT()),
            );
            widgets::segmented(
                ui,
                &mut days,
                &[("1 week", 7), ("1 month", 30), ("3 months", 90)],
            );
        });
        ui.ctx().data_mut(|d| d.insert_temp(id, days));
        ui.add_space(6.0);
        if book.curve.len() < 2 {
            widgets::empty(ui, 180.0, "Waiting for Treasury closes.");
            return;
        }
        curve_chart(ui, &book.curve, past);
        ui.add_space(6.0);
        tenor_grid(ui, &book.curve, prev, past, days);
        widgets::footnote(
            ui,
            "Official daily par yields. History covers the current calendar year of the Treasury feed.",
        );
    });
}

fn curve_chart(ui: &mut Ui, curve: &[model::CurvePoint], past: Option<&CurveDay>) {
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 190.0), Sense::hover());
    let painter = ui.painter_at(rect);
    let old = |label: &str| {
        past.and_then(|d| d.points.iter().find(|p| p.label == label))
            .map(|p| p.yield_pct)
    };
    let values: Vec<f64> = curve
        .iter()
        .map(|p| p.yield_pct)
        .chain(curve.iter().filter_map(|p| old(&p.label)))
        .collect();
    let lo = values.iter().copied().fold(f64::INFINITY, f64::min) - 0.1;
    let hi = values.iter().copied().fold(f64::NEG_INFINITY, f64::max) + 0.1;
    let plot = Rect::from_min_max(
        rect.min + Vec2::new(0.0, 8.0),
        rect.max - Vec2::new(46.0, 20.0),
    );
    let y = |v: f64| plot.bottom() - plot.height() * ((v - lo) / (hi - lo)) as f32;
    let x =
        |i: usize| plot.left() + 8.0 + (plot.width() - 16.0) * i as f32 / (curve.len() - 1) as f32;
    for v in nice_ticks(lo, hi, 4) {
        painter.hline(plot.x_range(), y(v), Stroke::new(1.0, BORDER()));
        painter.text(
            Pos2::new(plot.right() + 8.0, y(v)),
            Align2::LEFT_CENTER,
            format!("{v:.2}%"),
            mono(10.5),
            FAINT(),
        );
    }
    let now_line: Vec<Pos2> = curve
        .iter()
        .enumerate()
        .map(|(i, p)| Pos2::new(x(i), y(p.yield_pct)))
        .collect();
    let old_line: Vec<Pos2> = curve
        .iter()
        .enumerate()
        .filter_map(|(i, p)| old(&p.label).map(|v| Pos2::new(x(i), y(v))))
        .collect();
    if old_line.len() > 1 {
        painter.extend(Shape::dashed_line(
            &old_line,
            Stroke::new(1.3, AMBER()),
            5.0,
            4.0,
        ));
    }
    painter.add(Shape::line(now_line.clone(), Stroke::new(2.0, BLUE())));
    for (i, p) in curve.iter().enumerate() {
        painter.circle_filled(now_line[i], 2.8, BLUE());
        painter.text(
            Pos2::new(x(i), plot.bottom() + 5.0),
            Align2::CENTER_TOP,
            &p.label,
            mono(10.5),
            FAINT(),
        );
    }
    if let Some(mouse) = response
        .hover_pos()
        .filter(|p| plot.expand(6.0).contains(*p))
    {
        let i = (((mouse.x - plot.left() - 8.0) / (plot.width() - 16.0) * (curve.len() - 1) as f32)
            .round()
            .max(0.0) as usize)
            .min(curve.len() - 1);
        let p = &curve[i];
        painter.vline(x(i), plot.y_range(), Stroke::new(1.0, BORDER_STRONG()));
        let mut rows = vec![
            (format!("{} Treasury", p.label), FAINT()),
            (format!("now {:.2}%", p.yield_pct), BLUE()),
        ];
        if let Some(v) = old(&p.label) {
            rows.push((format!("then {v:.2}%"), AMBER()));
            rows.push((fmt_change_bps(p.yield_pct - v), MUTED()));
        }
        charts::tooltip(&painter, plot, Pos2::new(x(i), y(p.yield_pct)), &rows);
    }
    ui.horizontal(|ui| {
        key(ui, "Latest close", BLUE());
        if let Some(p) = past {
            key(ui, &model::pretty_date(&p.date), AMBER());
        }
    });
}

fn key(ui: &mut Ui, text: &str, color: eframe::egui::Color32) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 5.0;
        let (rect, _) = ui.allocate_exact_size(Vec2::new(14.0, 12.0), Sense::hover());
        ui.painter()
            .hline(rect.x_range(), rect.center().y, Stroke::new(2.0, color));
        ui.label(
            eframe::egui::RichText::new(text)
                .font(sans(11.0))
                .color(MUTED()),
        );
    });
}

/// Tenors across, with the level and the moves since yesterday and the comparison date.
fn tenor_grid(
    ui: &mut Ui,
    curve: &[model::CurvePoint],
    prev: Option<&CurveDay>,
    past: Option<&CurveDay>,
    days: i64,
) {
    let width = ui.available_width();
    let label_w = 58.0;
    let cell = (width - label_w) / curve.len() as f32;
    let rows: [(&str, Option<&CurveDay>); 2] = [
        ("1 day", prev),
        (
            match days {
                7 => "1 week",
                30 => "1 month",
                _ => "3 months",
            },
            past,
        ),
    ];
    let moved = |day: Option<&CurveDay>, p: &model::CurvePoint| {
        day.and_then(|d| d.points.iter().find(|o| o.label == p.label))
            .map(|o| Delta::Pp(p.yield_pct - o.yield_pct))
    };
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 20.0 * 4.0), Sense::hover());
    let painter = ui.painter();
    let small = cell < 40.0;
    for (r, title) in ["Tenor", "Yield", rows[0].0, rows[1].0].iter().enumerate() {
        let y = rect.top() + 20.0 * r as f32;
        let row_rect = Rect::from_min_size(Pos2::new(rect.left(), y), Vec2::new(width, 20.0));
        if r > 0 {
            painter.hline(
                row_rect.x_range(),
                row_rect.top(),
                Stroke::new(1.0, BORDER()),
            );
        }
        painter.galley(
            Pos2::new(row_rect.left(), row_rect.center().y - 6.0),
            painter.layout_job(caps_job(title, 9.0, FAINT())),
            FAINT(),
        );
        for (i, p) in curve.iter().enumerate() {
            let c = Rect::from_min_size(
                Pos2::new(rect.left() + label_w + cell * i as f32, y),
                Vec2::new(cell, 20.0),
            );
            let (text, color) = match r {
                0 => (p.label.clone(), MUTED()),
                1 => (format!("{:.2}", p.yield_pct), TEXT()),
                _ => {
                    let delta = moved(rows[r - 2].1, p);
                    let (text, color) = widgets::delta(delta);
                    (text.trim_end_matches(" bp").to_string(), color)
                }
            };
            widgets::cell_text(
                painter,
                c,
                &text,
                mono(if small { 9.5 } else { 10.5 }),
                color,
                Align::Max,
            );
        }
    }
}

fn dollar_index(app: &Desktop, ui: &mut Ui) {
    let history = app.snap.histories.get("DXY").cloned().unwrap_or_default();
    let live = quote(&app.snap.quotes, "DXY").map(|q| q.price);
    let points = with_observation(&history, live, app.snap.price_observed_unix("DXY"));
    let recent = &points[points.len().saturating_sub(260)..];
    let meta = match (live, quote(&app.snap.quotes, "DXY")) {
        (Some(v), Some(q)) => format!(
            "{v:.2} · {} session · {}",
            model::fmt_change_pct(q.change_pct),
            app.snap.price_as_of("DXY", now_unix())
        ),
        _ => String::new(),
    };
    widgets::panel(ui, "US dollar index (ICE DXY)", &meta, |ui| {
        let axis = |v: f64| format!("{v:.0}");
        let value = |v: f64| format!("{v:.2}");
        charts::time_chart(
            ui,
            TimeChart::new(150.0, &axis, &value).line(Line::new("DXY", recent, BLUE()).fill()),
        );
        widgets::footnote(ui, "CNBC dated daily closes over the last year. A delayed TradingView quote is shown in the header; only a dated observation can extend the chart.");
    });
}

/// The macro strip as a table; shared with the Bitcoin desk.
pub(crate) fn macro_table(ui: &mut Ui, t: &Terminal, title: &str, meta: &str) {
    let rows: Vec<_> = t
        .macro_strip
        .iter()
        .filter(|m| m.current.is_some())
        .collect();
    widgets::panel(ui, title, meta, |ui| {
        if rows.is_empty() {
            widgets::empty(ui, 40.0, "Macro series load on the first refresh.");
            return;
        }
        let wide = ui.available_width() >= 700.0;
        let mut cols = vec![
            col("Indicator", W::Flex(1.0), Align::Min),
            col("Level", W::Px(84.0), Align::Max),
            col("Day", W::Px(70.0), Align::Max),
            col("30 days", W::Px(76.0), Align::Max),
            col("Trend", W::Px(110.0), Align::Min),
        ];
        if wide {
            cols.push(col("As of", W::Px(84.0), Align::Max));
            cols.push(col("Source", W::Px(150.0), Align::Min));
        }
        let (layout, _) = widgets::table_header(ui, &cols, None);
        for m in rows {
            let current = m.current.unwrap_or_default();
            let rate = m.unit == "%";
            let level = if rate {
                format!("{current:.2}%")
            } else {
                format!("{current:.2}")
            };
            let day = m.prev.map(|p| {
                if rate {
                    Delta::Pp(current - p)
                } else {
                    Delta::Pct((current / p - 1.0) * 100.0)
                }
            });
            let month = m.delta_30d.map(|d| {
                if rate {
                    Delta::Pp(d)
                } else {
                    Delta::Pct(d / (current - d) * 100.0)
                }
            });
            let (rect, _) = widgets::table_row(ui, 26.0, false, false);
            let painter = ui.painter();
            widgets::cell_text(
                painter,
                layout.cell(rect, 0),
                &m.label,
                sans(12.5),
                TEXT(),
                Align::Min,
            );
            widgets::cell_text(
                painter,
                layout.cell(rect, 1),
                &level,
                mono(12.5),
                TEXT(),
                Align::Max,
            );
            let (text, color) = widgets::delta(day);
            widgets::cell_text(
                painter,
                layout.cell(rect, 2),
                &text,
                mono(11.5),
                color,
                Align::Max,
            );
            let (text, color) = widgets::delta(month);
            widgets::cell_text(
                painter,
                layout.cell(rect, 3),
                &text,
                mono(11.5),
                color,
                Align::Max,
            );
            let trend = match m.hist.as_slice() {
                [first, .., last] if last >= first => crate::theme::GREEN(),
                [_, _, ..] => crate::theme::RED(),
                _ => MUTED(),
            };
            charts::sparkline(
                painter,
                layout.cell(rect, 4).shrink2(Vec2::new(0.0, 5.0)),
                &m.hist[m.hist.len().saturating_sub(90)..],
                trend.gamma_multiply(0.85),
            );
            if wide {
                widgets::cell_text(
                    painter,
                    layout.cell(rect, 5),
                    &model::pretty_date(&m.as_of),
                    mono(11.0),
                    FAINT(),
                    Align::Max,
                );
                widgets::cell_text(
                    painter,
                    layout.cell(rect, 6),
                    &m.source,
                    sans(11.5),
                    FAINT(),
                    Align::Min,
                );
            }
        }
    });
}
