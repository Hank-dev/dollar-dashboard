//! Overview: regime line, desk tiles, the cross-asset board, watchlist,
//! stress monitor and the rule-based brief.

use crate::app::{Desk, Desktop};
use crate::charts;
use crate::chrome::local_label;
use crate::theme::{
    medium, mono, sans, status_color, AMBER, BORDER, BORDER_STRONG, FAINT, GAP, GREEN, HOVER,
    MUTED, RADIUS, RED, SURFACE, TEXT,
};
use crate::widgets::{self, caps_job, col, Col, W};
use dollar_dashboard::board::{self, Link, Row};
use dollar_dashboard::catalog::{AI_TICKERS, NUCLEAR_TICKERS, SPACEX_IPO_PRICE};
use dollar_dashboard::changes::{self, Target};
use dollar_dashboard::model::{
    self, dollar_metrics, fit_power_law, fmt_change_pct, headline, now_unix, power_z, quote,
    regime, total_cap, weighted_change, Delta, Regime, Status,
};
use dollar_dashboard::session;
use dollar_dashboard::workspace::instruments;
use eframe::egui::{
    Align, CornerRadius, CursorIcon, Pos2, Rect, RichText, Sense, Stroke, StrokeKind, Ui, Vec2,
};

pub(crate) fn show(app: &mut Desktop, ui: &mut Ui) {
    let now = now_unix();
    let rows = board::rows(&app.snap, now);
    regime_line(app, ui, now);
    ui.add_space(GAP);
    attention(app, ui, now);
    ui.add_space(GAP);
    super::pulse::overview(app, ui);
    ui.add_space(GAP);
    tiles(app, ui, &rows);
    ui.add_space(GAP);
    if ui.available_width() < 1050.0 {
        widgets::split(ui, 0.5, 700.0, |ui, side| {
            if side == 0 {
                watchlist(app, ui);
            } else {
                stress(app, ui);
            }
        });
        ui.add_space(GAP);
        eframe::egui::CollapsingHeader::new(
            RichText::new("Full cross-asset board")
                .font(sans(12.5))
                .color(MUTED()),
        )
        .id_salt("overview-compact-board")
        .default_open(false)
        .show(ui, |ui| board_panel(app, ui, &rows));
        ui.add_space(GAP);
        eframe::egui::CollapsingHeader::new(
            RichText::new("Rule-based brief")
                .font(sans(12.5))
                .color(MUTED()),
        )
        .id_salt("overview-compact-brief")
        .default_open(false)
        .show(ui, |ui| brief(app, ui, now));
        return;
    }
    widgets::split(ui, 0.6, 980.0, |ui, side| {
        if side == 0 {
            board_panel(app, ui, &rows);
        } else {
            watchlist(app, ui);
            ui.add_space(GAP);
            stress(app, ui);
            ui.add_space(GAP);
            brief(app, ui, now);
        }
    });
}

fn regime_line(app: &Desktop, ui: &mut Ui, now: i64) {
    let dxy = app.snap.dollar.dxy.as_ref().map(|o| o.value);
    let vix = app.snap.dollar.vix.as_ref().map(|o| o.value);
    let tone = regime(app.snap.btc.change_24h_pct, vix, app.snap.btc.fng, dxy);
    let line = headline(app.snap.btc.change_24h_pct, vix, app.snap.btc.fng, dxy);
    let accent = match tone {
        Regime::RiskOn => GREEN(),
        Regime::RiskOff => RED(),
        Regime::Mixed => AMBER(),
    };
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        let (rect, _) = ui.allocate_exact_size(Vec2::new(8.0, 14.0), Sense::hover());
        ui.painter().circle_filled(rect.center(), 3.5, accent);
        ui.label(caps_job(&format!("{} regime", tone.label()), 10.5, accent))
            .on_hover_text(
                "Counts BTC 24h, VIX, Fear & Greed and ICE DXY against fixed thresholds.",
            );
    });
    ui.add_space(2.0);
    ui.label(
        RichText::new("Market attention")
            .font(medium(21.0))
            .color(TEXT()),
    );
    ui.label(RichText::new(line).font(sans(11.5)).color(MUTED()))
        .on_hover_text("A fixed-threshold BTC, VIX, sentiment and dollar summary. The developments below also include AI, nuclear and source quality.");
    let s = session::us_equities(now);
    let session = if s.open {
        format!("US market open, closes {}", local_label(s.next_change, now))
    } else {
        format!(
            "US market closed, opens {}",
            local_label(s.next_change, now)
        )
    };
    ui.label(
        RichText::new(format!(
            "{} · {session} · crypto trades around the clock",
            chrono::Local::now().format("%A %-d %B %Y")
        ))
        .font(sans(11.5))
        .color(FAINT()),
    );
}

fn attention(app: &mut Desktop, ui: &mut Ui, now: i64) {
    let events = changes::developments(&app.snap, app.visit_baseline.as_ref(), now);
    let meta = match app.visit_baseline.as_ref() {
        Some(b) => format!(
            "compared with {} · notable session moves",
            model::fmt_age(b.captured_unix, now)
        ),
        None => "first visit · notable session moves; comparisons start after this visit".into(),
    };
    let mut target = None;
    widgets::panel(ui, "What needs attention", &meta, |ui| {
        if events.is_empty() {
            ui.label(
                RichText::new("No significant changes in the dated readings received so far.")
                    .font(sans(12.5))
                    .color(MUTED()),
            );
            if app.snap.price_stamps.is_empty() {
                ui.label(
                    RichText::new("Refresh sources to establish dated price observations.")
                        .font(sans(11.5))
                        .color(FAINT()),
                );
            }
        }
        for (index, event) in events.iter().enumerate() {
            if index > 0 {
                ui.add_space(4.0);
            }
            let response = eframe::egui::Frame::new()
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new("•").color(status_color(event.status)));
                        if widgets::link(ui, &event.title, TEXT())
                            .on_hover_text(&event.detail)
                            .clicked()
                        {
                            target = Some(event.target.clone());
                        }
                    });
                    let evidence = event
                        .detail
                        .split_once(" · ")
                        .map(|(values, _)| format!("{values} · {}", event.evidence))
                        .unwrap_or_else(|| event.evidence.clone());
                    ui.label(RichText::new(evidence).font(sans(10.5)).color(FAINT()))
                        .on_hover_text(&event.detail);
                })
                .response;
            if response.on_hover_text(&event.detail).double_clicked() {
                target = Some(event.target.clone());
            }
        }
    });
    if let Some(target) = target {
        match target {
            Target::Bitcoin => app.navigate(Desk::Btc),
            Target::Ai => app.navigate(Desk::Ai),
            Target::Nuclear => app.navigate(Desk::Nuclear),
            Target::Zcash => app.navigate(Desk::Zcash),
            Target::SpaceX => app.navigate(Desk::SpaceX),
            Target::Dollar(metric) => {
                app.navigate(Desk::Dollar);
                app.selected = metric;
            }
            Target::Sources => app.show_sources = true,
        }
    }
}

struct Tile {
    desk: Desk,
    value: String,
    change: Option<Delta>,
    line: String,
    spark: Vec<f64>,
    /// Constituent moves for basket tiles, painted as a strip instead of a sparkline.
    strip: Vec<(String, f64)>,
}

fn tiles(app: &mut Desktop, ui: &mut Ui, rows: &[Row]) {
    let snap = &app.snap;
    let row = |key: &str| rows.iter().find(|r| r.key == key);
    let spark = |key: &str| row(key).map(|r| r.spark.clone()).unwrap_or_default();
    let fit = fit_power_law(&snap.btc.daily);
    let z = fit
        .as_ref()
        .and_then(|fit| snap.btc.price.and_then(|px| power_z(fit, now_unix(), px)));
    let ath = snap
        .terminal
        .ath
        .or_else(|| model::ath(&snap.btc.daily).map(|p| p.v));
    let from_ath = match (snap.btc.price, ath) {
        (Some(px), Some(ath)) if ath > 0.0 => {
            format!("{} from ATH", fmt_change_pct((px / ath - 1.0) * 100.0))
        }
        _ => String::new(),
    };
    let btc_line = [
        from_ath,
        z.map(|z| model::with_minus(format!("power law {z:+.2}σ")))
            .unwrap_or_default(),
    ]
    .into_iter()
    .filter(|s| !s.is_empty())
    .collect::<Vec<_>>()
    .join(" · ");
    let ten = row("US10Y");
    let curve = row("2S10S");
    let dollar_line = [
        ten.filter(|r| r.last.is_some())
            .map(|r| format!("US 10Y {}", r.text)),
        curve
            .filter(|r| r.last.is_some())
            .map(|r| format!("2s10s {}", r.text)),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(" · ");
    let top = snap
        .terminal
        .models
        .iter()
        .find(|m| m.intelligence.is_some());
    let strip = |symbols: &[&str]| -> Vec<(String, f64)> {
        symbols
            .iter()
            .filter_map(|s| quote(&snap.quotes, s).map(|q| (s.to_string(), q.change_pct)))
            .collect()
    };
    let uranium = quote(&snap.quotes, "URANIUM")
        .map(|q| {
            format!(
                "U3O8 ${:.2}/lb {} · UxC futures",
                q.price,
                fmt_change_pct(q.change_pct)
            )
        })
        .unwrap_or_default();
    let zec = &snap.zec;
    let zec_line = [
        zec.shielded_share().map(|v| format!("{v:.1}% shielded")),
        zec.btc_ratio.map(|r| format!("ZEC/BTC {r:.5}")),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(" · ");
    let tiles = [
        Tile {
            desk: Desk::Btc,
            value: snap.btc.price.map(model::fmt_usd).unwrap_or_else(dash),
            change: snap.btc.change_24h_pct.map(Delta::Pct),
            line: btc_line,
            spark: spark("BTC"),
            strip: Vec::new(),
        },
        Tile {
            desk: Desk::Dollar,
            value: row("DXY")
                .filter(|r| r.last.is_some())
                .map(|r| format!("DXY {}", r.text))
                .unwrap_or_else(dash),
            change: row("DXY").and_then(|r| r.day),
            line: dollar_line,
            spark: spark("DXY"),
            strip: Vec::new(),
        },
        Tile {
            desk: Desk::Ai,
            value: total_cap(&snap.quotes, AI_TICKERS)
                .map(model::fmt_cap)
                .unwrap_or_else(dash),
            change: weighted_change(&snap.quotes, AI_TICKERS).map(Delta::Pct),
            line: top
                .map(|m| {
                    format!(
                        "Top model {} · {:.1}",
                        m.label.split(" (").next().unwrap_or(&m.label),
                        m.intelligence.unwrap_or(0.0)
                    )
                })
                .unwrap_or_else(|| "Seven listed AI names, cap-weighted".into()),
            spark: Vec::new(),
            strip: strip(AI_TICKERS),
        },
        Tile {
            desk: Desk::Nuclear,
            value: total_cap(&snap.quotes, NUCLEAR_TICKERS)
                .map(model::fmt_cap)
                .unwrap_or_else(dash),
            change: weighted_change(&snap.quotes, NUCLEAR_TICKERS).map(Delta::Pct),
            line: uranium,
            spark: Vec::new(),
            strip: strip(NUCLEAR_TICKERS),
        },
        Tile {
            desk: Desk::Zcash,
            value: snap.btc.zec_price.map(model::fmt_usd).unwrap_or_else(dash),
            change: snap.btc.zec_change_24h_pct.map(Delta::Pct),
            line: zec_line,
            spark: spark("ZEC"),
            strip: Vec::new(),
        },
        {
            let spcx = model::quote(&snap.quotes, "SPCX");
            Tile {
                desk: Desk::SpaceX,
                value: spcx.map(|q| model::fmt_usd(q.price)).unwrap_or_else(dash),
                change: spcx.map(|q| Delta::Pct(q.change_pct)),
                line: spcx
                    .map(|q| {
                        let ipo = (q.price / SPACEX_IPO_PRICE - 1.0) * 100.0;
                        match q.market_cap {
                            Some(cap) => format!(
                                "{} vs IPO · cap {}",
                                fmt_change_pct(ipo),
                                model::fmt_cap(cap)
                            ),
                            None => format!("{} vs $135 IPO", fmt_change_pct(ipo)),
                        }
                    })
                    .unwrap_or_else(|| "SPCX quote unavailable".into()),
                spark: spark("SPCX"),
                strip: Vec::new(),
            }
        },
    ];
    let width = ui.available_width();
    let compact = width < 1050.0;
    let cols = if width >= 900.0 {
        6
    } else if width >= 420.0 {
        3
    } else if width >= 300.0 {
        2
    } else {
        1
    };
    let tile_w = (width - GAP * (cols - 1) as f32) / cols as f32;
    let height = if compact { 62.0 } else { 112.0 };
    let mut clicked = None;
    for chunk in tiles.chunks(cols) {
        let (row_rect, _) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
        for (i, tile) in chunk.iter().enumerate() {
            let rect = Rect::from_min_size(
                row_rect.min + Vec2::new(i as f32 * (tile_w + GAP), 0.0),
                Vec2::new(tile_w, height),
            );
            let response = ui.interact(
                rect,
                ui.id().with(("tile", tile.desk.key())),
                Sense::click(),
            );
            paint_tile(ui, rect, tile, response.hovered());
            if response.hovered() {
                ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
            }
            if response.clicked() {
                clicked = Some(tile.desk);
            }
        }
        ui.add_space(GAP);
    }
    if let Some(desk) = clicked {
        app.navigate(desk);
    }
}

fn paint_tile(ui: &Ui, rect: Rect, tile: &Tile, hovered: bool) {
    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::same(RADIUS()), SURFACE());
    painter.rect_stroke(
        rect,
        CornerRadius::same(RADIUS()),
        Stroke::new(1.0, if hovered { BORDER_STRONG() } else { BORDER() }),
        StrokeKind::Inside,
    );
    let inner = rect.shrink2(Vec2::new(14.0, 11.0));
    let accent = tile.desk.accent();
    let compact = rect.height() < 80.0;
    let title = if compact {
        match tile.desk {
            Desk::Btc => "Bitcoin",
            Desk::Dollar => "Dollar",
            Desk::Ai => "AI",
            Desk::Nuclear => "Nuclear",
            Desk::Zcash => "Zcash",
            Desk::SpaceX => "SpaceX",
            _ => tile.desk.title(),
        }
    } else {
        tile.desk.title()
    };
    let label = painter.layout_job(caps_job(title, 9.5, accent));
    painter.galley(inner.left_top(), label, accent);
    if !compact {
        painter.text(
            inner.right_top(),
            eframe::egui::Align2::RIGHT_TOP,
            tile.desk.key().to_string(),
            mono(10.0),
            FAINT(),
        );
    }
    let (change, color) = widgets::delta(tile.change);
    if compact {
        let value_rect = Rect::from_min_size(
            inner.left_top() + Vec2::new(0.0, 17.0),
            Vec2::new(inner.width(), 20.0),
        );
        widgets::cell_text(
            painter,
            value_rect,
            &tile.value,
            medium(15.0),
            TEXT(),
            Align::Min,
        );
        painter.text(
            inner.right_top(),
            eframe::egui::Align2::RIGHT_TOP,
            change,
            mono(9.5),
            color,
        );
        return;
    }
    let change_galley = painter.layout_no_wrap(change, mono(12.0), color);
    let value_rect = Rect::from_min_size(
        inner.left_top() + Vec2::new(0.0, 17.0),
        Vec2::new(inner.width() - change_galley.size().x - 8.0, 26.0),
    );
    widgets::cell_text(
        painter,
        value_rect,
        &tile.value,
        medium(19.0),
        TEXT(),
        Align::Min,
    );
    painter.galley(
        Pos2::new(
            inner.right() - change_galley.size().x,
            value_rect.center().y - change_galley.size().y / 2.0 + 1.0,
        ),
        change_galley,
        color,
    );
    let line_rect = Rect::from_min_size(
        inner.left_top() + Vec2::new(0.0, 46.0),
        Vec2::new(inner.width(), 16.0),
    );
    widgets::cell_text(
        painter,
        line_rect,
        &tile.line,
        sans(11.5),
        MUTED(),
        Align::Min,
    );
    let foot = Rect::from_min_max(
        Pos2::new(inner.left(), inner.bottom() - 24.0),
        inner.right_bottom(),
    );
    if !tile.strip.is_empty() {
        strip(ui, foot, &tile.strip);
    } else {
        let color = match tile.spark.as_slice() {
            [first, .., last] if last >= first => GREEN(),
            [_, _, ..] => RED(),
            _ => MUTED(),
        };
        charts::sparkline(painter, foot, &tile.spark, color.gamma_multiply(0.9));
    }
    if hovered {
        painter.rect_filled(
            rect,
            CornerRadius::same(RADIUS()),
            HOVER().gamma_multiply(0.15),
        );
    }
}

/// One bar per constituent's session move, centred on a zero line.
fn strip(ui: &Ui, rect: Rect, moves: &[(String, f64)]) {
    let painter = ui.painter();
    let max = moves.iter().map(|(_, v)| v.abs()).fold(1.0_f64, f64::max);
    let slot = rect.width() / moves.len() as f32;
    let mid = rect.center().y;
    painter.hline(rect.x_range(), mid, Stroke::new(1.0, BORDER_STRONG()));
    let show_labels = slot >= 30.0;
    for (i, (symbol, v)) in moves.iter().enumerate() {
        let x = rect.left() + slot * (i as f32 + 0.5);
        let h = (rect.height() / 2.0 - 2.0) * (v.abs() / max) as f32;
        let bar = if *v >= 0.0 {
            Rect::from_min_max(
                Pos2::new(x - slot * 0.3, mid - h),
                Pos2::new(x + slot * 0.3, mid),
            )
        } else {
            Rect::from_min_max(
                Pos2::new(x - slot * 0.3, mid),
                Pos2::new(x + slot * 0.3, mid + h),
            )
        };
        painter.rect_filled(
            bar,
            crate::theme::corner(1),
            if *v >= 0.0 { GREEN() } else { RED() }.gamma_multiply(0.75),
        );
        if show_labels {
            let anchor = if *v >= 0.0 {
                (Pos2::new(x, mid + 2.0), eframe::egui::Align2::CENTER_TOP)
            } else {
                (Pos2::new(x, mid - 2.0), eframe::egui::Align2::CENTER_BOTTOM)
            };
            painter.text(anchor.0, anchor.1, symbol, mono(8.5), FAINT());
        }
    }
    let tip = moves
        .iter()
        .map(|(s, v)| format!("{s} {}", fmt_change_pct(*v)))
        .collect::<Vec<_>>()
        .join("   ");
    ui.interact(
        rect,
        ui.id().with(("strip", rect.left() as i32)),
        Sense::hover(),
    )
    .on_hover_text(tip);
}

fn desk_for(link: Link) -> Option<Desk> {
    match link {
        Link::Bitcoin => Some(Desk::Btc),
        Link::Dollar => Some(Desk::Dollar),
        Link::Nuclear => Some(Desk::Nuclear),
        Link::Zcash => Some(Desk::Zcash),
        Link::SpaceX => Some(Desk::SpaceX),
        Link::None => None,
    }
}

fn board_panel(app: &mut Desktop, ui: &mut Ui, rows: &[Row]) {
    let mut go = None;
    widgets::panel(
        ui,
        "Cross-asset board",
        "session move · 30 days · 52-week range",
        |ui| {
            let width = ui.available_width();
            let full = width >= 660.0;
            let mid = width >= 470.0;
            let roomy = width >= 1080.0;
            let mut cols: Vec<Col> = vec![
                col("Instrument", W::Flex(1.0), Align::Min),
                col("Last", W::Px(92.0), Align::Max),
                col("Day", W::Px(72.0), Align::Max),
            ];
            if mid {
                cols.push(col("30D", W::Px(72.0), Align::Max));
            }
            if full {
                cols.push(col("52-week", W::Px(80.0), Align::Min));
            }
            if mid {
                cols.push(col("Trend 90D", W::Px(80.0), Align::Min));
            }
            if full {
                cols.push(col("As of", W::Px(54.0), Align::Max));
            }
            if roomy {
                cols.push(col("Source", W::Px(220.0), Align::Min));
            }
            let (layout, _) = widgets::table_header(ui, &cols, None);
            let mut group = "";
            for row in rows {
                if row.group != group {
                    group = row.group;
                    widgets::table_group(ui, group);
                }
                let link = desk_for(row.link);
                let (rect, response) = widgets::table_row(ui, 25.0, false, link.is_some());
                let painter = ui.painter();
                let mut c = 0;
                widgets::cell_text(
                    painter,
                    layout.cell(rect, c),
                    row.label,
                    sans(12.5),
                    TEXT(),
                    Align::Min,
                );
                c += 1;
                widgets::cell_text(
                    painter,
                    layout.cell(rect, c),
                    &row.text,
                    mono(12.5),
                    TEXT(),
                    Align::Max,
                );
                c += 1;
                let (day, day_color) = widgets::delta(row.day);
                widgets::cell_text(
                    painter,
                    layout.cell(rect, c),
                    &day,
                    mono(12.0),
                    day_color,
                    Align::Max,
                );
                if mid {
                    c += 1;
                    let (month, month_color) = widgets::delta(row.month);
                    widgets::cell_text(
                        painter,
                        layout.cell(rect, c),
                        &month,
                        mono(12.0),
                        month_color,
                        Align::Max,
                    );
                }
                if full {
                    c += 1;
                    if let (Some((lo, hi)), Some(last)) = (row.range, row.last) {
                        charts::range_bar(
                            painter,
                            layout.cell(rect, c).shrink2(Vec2::new(2.0, 6.0)),
                            lo,
                            hi,
                            last,
                            TEXT(),
                        );
                    }
                }
                if mid {
                    c += 1;
                    let trend = match row.spark.as_slice() {
                        [first, .., last] if last >= first => GREEN(),
                        [_, _, ..] => RED(),
                        _ => MUTED(),
                    };
                    charts::sparkline(
                        painter,
                        layout.cell(rect, c).shrink2(Vec2::new(0.0, 5.0)),
                        &row.spark,
                        trend.gamma_multiply(0.85),
                    );
                }
                if full {
                    c += 1;
                    widgets::cell_text(
                        painter,
                        layout.cell(rect, c),
                        &row.as_of,
                        mono(10.5),
                        FAINT(),
                        Align::Max,
                    );
                }
                if roomy {
                    c += 1;
                    widgets::cell_text(
                        painter,
                        layout.cell(rect, c),
                        &row.source,
                        sans(11.5),
                        FAINT(),
                        Align::Min,
                    );
                }
                let mut hint = format!("{} · {} · {}", row.label, row.source, row.as_of);
                if let Some((lo, hi)) = row.range {
                    hint.push_str(&format!(
                        "\n52-week range {} – {}",
                        model::fmt_price(lo),
                        model::fmt_price(hi)
                    ));
                }
                if let Some(desk) = link {
                    hint.push_str(&format!("\nOpen the {} desk", desk.title()));
                }
                if response.on_hover_text(hint).clicked() {
                    go = link;
                }
            }
        },
    );
    if let Some(desk) = go {
        app.navigate(desk);
    }
}

fn watchlist(app: &mut Desktop, ui: &mut Ui) {
    let quotes = instruments(&app.snap);
    let symbols = app.prefs.watchlist.clone();
    let mut go: Option<(Desk, Option<String>)> = None;
    widgets::panel(ui, "Watchlist", &format!("{} saved", symbols.len()), |ui| {
        if symbols.is_empty() {
            widgets::empty(
                ui,
                36.0,
                "Star instruments on the Markets desk to follow them here.",
            );
        } else {
            let cols = [
                col("Symbol", W::Px(54.0), Align::Min),
                col("Name", W::Flex(1.0), Align::Min),
                col("Last", W::Px(84.0), Align::Max),
                col("Day", W::Px(64.0), Align::Max),
            ];
            let (layout, _) = widgets::table_header(ui, &cols, None);
            let limit = if ui.available_width() < 420.0 { 3 } else { 10 };
            for symbol in symbols.iter().take(limit) {
                let quote = quotes.iter().find(|q| &q.symbol == symbol);
                let (rect, response) = widgets::table_row(ui, 25.0, false, true);
                let painter = ui.painter();
                widgets::cell_text(
                    painter,
                    layout.cell(rect, 0),
                    symbol,
                    mono(12.0),
                    TEXT(),
                    Align::Min,
                );
                widgets::cell_text(
                    painter,
                    layout.cell(rect, 1),
                    &super::name_of(app, symbol),
                    sans(12.0),
                    MUTED(),
                    Align::Min,
                );
                match quote {
                    Some(q) => {
                        widgets::cell_text(
                            painter,
                            layout.cell(rect, 2),
                            &model::fmt_usd(q.price),
                            mono(12.0),
                            TEXT(),
                            Align::Max,
                        );
                        let (text, color) = widgets::delta(Some(Delta::Pct(q.change_pct)));
                        widgets::cell_text(
                            painter,
                            layout.cell(rect, 3),
                            &text,
                            mono(12.0),
                            color,
                            Align::Max,
                        );
                    }
                    None => widgets::cell_text(
                        painter,
                        layout.cell(rect, 2),
                        "awaiting",
                        sans(11.5),
                        FAINT(),
                        Align::Max,
                    ),
                }
                if response.on_hover_text(format!(
                    "{} · {}",
                    app.snap.price_stamp(symbol).map(|stamp| stamp.source.as_str()).unwrap_or("Observation time unavailable"),
                    app.snap.price_as_of(symbol, now_unix()),
                )).clicked() {
                    go = Some(match symbol.as_str() {
                        "BTC" | "ETH" => (Desk::Btc, None),
                        "ZEC" => (Desk::Zcash, None),
                        "SPCX" => (Desk::SpaceX, None),
                        _ => (Desk::Markets, Some(symbol.clone())),
                    });
                }
            }
        }
        ui.add_space(6.0);
        if widgets::link(ui, "Edit watchlist on the Markets desk", FAINT()).clicked() {
            go = Some((Desk::Markets, None));
        }
    });
    if let Some((desk, filter)) = go {
        app.navigate(desk);
        if desk == Desk::Markets {
            app.watch_only = filter.is_none();
            app.filter = filter.unwrap_or_default();
        }
    }
}

fn stress(app: &mut Desktop, ui: &mut Ui) {
    let metrics: Vec<_> = dollar_metrics(&app.snap)
        .into_iter()
        .filter(|m| m.live)
        .collect();
    let count = |s: Status| metrics.iter().filter(|m| m.status == s).count();
    let mut ranked = metrics.clone();
    ranked.sort_by_key(|m| match m.status {
        Status::Stressed => 0,
        Status::Elevated => 1,
        Status::Neutral => 2,
        Status::Calm => 3,
    });
    let mut go = None;
    widgets::panel(
        ui,
        "Stress monitor",
        &format!(
            "{} stressed · {} elevated of {}",
            count(Status::Stressed),
            count(Status::Elevated),
            metrics.len()
        ),
        |ui| {
            let cols = [
                col("", W::Px(8.0), Align::Min),
                col("Rule", W::Flex(1.0), Align::Min),
                col("Value", W::Px(84.0), Align::Max),
                col("Day", W::Px(64.0), Align::Max),
                col("State", W::Px(64.0), Align::Max),
            ];
            let (layout, _) = widgets::table_header(ui, &cols, None);
            let limit = if ui.available_width() < 420.0 { 3 } else { 7 };
            for m in ranked.iter().take(limit) {
                let (rect, response) = widgets::table_row(ui, 25.0, false, true);
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
                    mono(12.0),
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
                    Align::Max,
                );
                if response.on_hover_text(m.rule).clicked() {
                    go = Some(m.id);
                }
            }
            ui.add_space(6.0);
            if widgets::link(ui, "All rules on the Dollar desk", FAINT()).clicked() {
                go = Some("");
            }
        },
    );
    if let Some(id) = go {
        app.navigate(Desk::Dollar);
        app.selected = (!id.is_empty()).then(|| id.to_string());
    }
}

fn brief(app: &Desktop, ui: &mut Ui, now: i64) {
    let snap = &app.snap;
    let mut lines = vec![model::rule_reading(&dollar_metrics(snap))];
    if let Some(price) = snap.btc.price {
        let mut text = format!("Bitcoin trades at {}", model::fmt_usd(price));
        if let Some(change) = snap.btc.change_24h_pct {
            text.push_str(&format!(" ({} over 24h)", fmt_change_pct(change)));
        }
        if let Some(fng) = snap.btc.fng {
            text.push_str(&format!(
                "; Fear & Greed reads {fng} ({})",
                snap.btc.fng_label
            ));
        }
        if let Some(rate) = snap.btc.funding {
            text.push_str(&format!(
                "; Binance funding {}",
                super::funding_text(Some(rate))
            ));
        }
        text.push('.');
        lines.push(text);
    }
    if let (Some(level), Some(delta)) = (snap.terminal.net_liq_b, snap.terminal.net_liq_30d_b) {
        lines.push(format!(
            "Fed net liquidity is ${:.2}T, {} over 30 days, so liquidity is {}.",
            level / 1000.0,
            model::with_minus(format!("{delta:+.0}B")),
            if delta >= 0.0 {
                "expanding"
            } else {
                "contracting"
            }
        ));
    }
    let baskets: Vec<String> = [("AI", AI_TICKERS), ("nuclear", NUCLEAR_TICKERS)]
        .iter()
        .filter_map(|(name, symbols)| {
            weighted_change(&snap.quotes, symbols)
                .map(|c| format!("the {name} basket moved {}", fmt_change_pct(c)))
        })
        .collect();
    if !baskets.is_empty() {
        let session = session::us_equities(now);
        lines.push(format!(
            "In the {} US session, {} (cap-weighted, delayed quotes).",
            if session.open { "current" } else { "last" },
            baskets.join(" and ")
        ));
    }
    widgets::panel(ui, "Brief", "rule-based · not an AI summary", |ui| {
        for line in lines {
            ui.label(RichText::new(line).font(sans(12.5)).color(TEXT()));
            ui.add_space(4.0);
        }
    });
}

fn dash() -> String {
    "—".into()
}
