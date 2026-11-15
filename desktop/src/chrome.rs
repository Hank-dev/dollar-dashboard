//! Window chrome: title bar, ticker tape, desk sidebar, status bar and notices.

use crate::app::{Desk, Desktop};
use crate::theme::{
    medium, mono, sans, AMBER, BG, ACCENT, BORDER, BORDER_STRONG, FAINT, GREEN, INPUT, MUTED,
    RAISED, SURFACE, TEXT,
};
use crate::widgets::{self, caps_job};
use chrono::{Local, TimeZone};
use dollar_dashboard::board::{self, Link};
use dollar_dashboard::feeds::{self, Freshness};
use dollar_dashboard::model::{fmt_age, now_unix, Delta};
use dollar_dashboard::session;
use eframe::egui::{
    self, Align, Align2, Color32, CursorIcon, Frame, Layout, Margin, Pos2, Rect,
    Response, RichText, Sense, Stroke, StrokeKind, Ui, Vec2,
};

pub(crate) fn title_bar(app: &mut Desktop, root: &mut Ui) {
    egui::Panel::top("title")
        .exact_size(46.0)
        .frame(
            Frame::new()
                .fill(SURFACE())
                .inner_margin(Margin::symmetric(16, 0)),
        )
        .show(root, |ui| {
            let width = ui.available_width();
            ui.horizontal_centered(|ui| {
                logo(ui);
                ui.add_space(2.0);
                ui.label(RichText::new("Market Monitor").font(medium(14.0)).color(TEXT()));
                ui.label(RichText::new("/").font(sans(13.0)).color(FAINT()));
                ui.label(RichText::new(app.desk.title()).font(sans(13.0)).color(MUTED()));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    if ui.button("Ask Codex").on_hover_text("Discuss this desk with Codex using the current app data").clicked() {
                        app.codex.open = !app.codex.open;
                    }
                    let refresh = egui::Button::new(
                        RichText::new(if app.refreshing {
                            "Refreshing…"
                        } else {
                            "Refresh"
                        })
                        .font(sans(12.0)),
                    );
                    let next = app.next_refresh_secs();
                    if ui
                        .add_enabled(!app.refreshing, refresh)
                        .on_hover_text(format!(
                            "Refresh public data sources (R) · X Pulse refreshes separately · automatic refresh in {}",
                            minutes(next)
                        ))
                        .clicked()
                    {
                        app.kick();
                    }
                    let armed = app
                        .prefs
                        .alerts
                        .iter()
                        .filter(|a| a.triggered_at.is_none())
                        .count();
                    let fired = app.prefs.alerts.len() - armed;
                    let label = match (armed, fired) {
                        (0, 0) => "Alerts".to_string(),
                        (n, 0) => format!("Alerts · {n}"),
                        (_, n) => format!("Alerts · {n} fired"),
                    };
                    let color = if fired > 0 { AMBER() } else { TEXT() };
                    if ui
                        .button(RichText::new(label).font(sans(12.0)).color(color))
                        .on_hover_text("Price alerts for BTC, ETH and ZEC")
                        .clicked()
                    {
                        app.show_alerts = true;
                    }
                    let field_w = if width >= 1250.0 {
                        270.0
                    } else if width >= 1050.0 {
                        200.0
                    } else {
                        110.0
                    };
                    if search_field(ui, field_w).clicked() {
                        app.show_search = true;
                        app.search_index = 0;
                    }
                    if width >= 1150.0 {
                        session_label(ui);
                    }
                });
            });
        });
}

fn logo(ui: &mut Ui) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(20.0), Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, crate::theme::corner(5), ACCENT().gamma_multiply(0.16));
    painter.rect_stroke(
        rect,
        crate::theme::corner(5),
        Stroke::new(1.0, ACCENT().gamma_multiply(0.55)),
        StrokeKind::Inside,
    );
    let r = rect.shrink(4.5);
    let line = [
        Pos2::new(r.left(), r.bottom() - 2.0),
        Pos2::new(r.left() + r.width() * 0.3, r.center().y + 1.0),
        Pos2::new(r.left() + r.width() * 0.55, r.bottom() - 3.5),
        Pos2::new(r.right(), r.top() + 1.0),
    ];
    painter.add(egui::Shape::line(line.to_vec(), Stroke::new(1.6, ACCENT())));
}

fn search_field(ui: &mut Ui, width: f32) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 28.0), Sense::click());
    let painter = ui.painter();
    painter.rect_filled(rect, crate::theme::corner(5), INPUT());
    painter.rect_stroke(
        rect,
        crate::theme::corner(5),
        Stroke::new(
            1.0,
            if response.hovered() {
                BORDER_STRONG()
            } else {
                BORDER()
            },
        ),
        StrokeKind::Inside,
    );
    let key = painter.layout_no_wrap("Ctrl K".into(), mono(10.0), MUTED());
    let key_rect = Rect::from_min_size(
        Pos2::new(rect.right() - key.size().x - 14.0, rect.center().y - 8.0),
        Vec2::new(key.size().x + 8.0, 16.0),
    );
    painter.rect_filled(key_rect, crate::theme::corner(3), RAISED());
    painter.galley(
        key_rect.center() - key.size() / 2.0,
        key,
        MUTED(),
    );
    let hint = if width >= 200.0 {
        "Search desks, tickers, metrics"
    } else {
        "Search"
    };
    let hint_rect = Rect::from_min_max(
        Pos2::new(rect.left() + 10.0, rect.top()),
        Pos2::new(key_rect.left() - 6.0, rect.bottom()),
    );
    widgets::cell_text(painter, hint_rect, hint, sans(12.0), FAINT(), Align::Min);
    if response.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    response
}

fn session_label(ui: &mut Ui) {
    let now = now_unix();
    let s = session::us_equities(now);
    let when = local_label(s.next_change, now);
    let (color, text) = if s.open {
        (GREEN(), format!("US market open · closes {when}"))
    } else {
        (FAINT(), format!("US market closed · opens {when}"))
    };
    ui.label(RichText::new(text).font(sans(11.5)).color(MUTED()));
    let (rect, _) = ui.allocate_exact_size(Vec2::new(8.0, 12.0), Sense::hover());
    ui.painter().circle_filled(rect.center(), 3.0, color);
}

/// Local wall-clock time: "15:30", "Mon 15:30" within a week, else "3 Nov 15:30".
pub(crate) fn local_label(t: i64, now: i64) -> String {
    let (Some(at), Some(today)) = (
        Local.timestamp_opt(t, 0).single(),
        Local.timestamp_opt(now, 0).single(),
    ) else {
        return String::new();
    };
    if at.date_naive() == today.date_naive() {
        at.format("%H:%M").to_string()
    } else if (t - now) < 6 * 86_400 {
        at.format("%a %H:%M").to_string()
    } else {
        at.format("%-d %b %H:%M").to_string()
    }
}

fn minutes(secs: u64) -> String {
    if secs < 90 {
        format!("{secs}s")
    } else {
        format!("{} min", secs.div_ceil(60))
    }
}

struct Tick {
    symbol: &'static str,
    value: String,
    change: Option<Delta>,
    hint: String,
    desk: Desk,
}

const TAPE: &[(&str, &str)] = &[
    ("BTC", "BTC"),
    ("ETH", "ETH"),
    ("ZEC", "ZEC"),
    ("SPX", "SPX"),
    ("NDX", "NDX"),
    ("SPCX", "SPCX"),
    ("US10Y", "US10Y"),
    ("DXY", "DXY"),
    ("VIX", "VIX"),
    ("EURUSD", "EURUSD"),
    ("USDJPY", "USDJPY"),
    ("GOLD", "GOLD"),
    ("BRENT", "BRENT"),
    ("US02Y", "US2Y"),
    ("WTI", "WTI"),
];

fn desk_for(link: Link) -> Desk {
    match link {
        Link::Bitcoin => Desk::Btc,
        Link::Dollar => Desk::Dollar,
        Link::Nuclear => Desk::Nuclear,
        Link::Zcash => Desk::Zcash,
        Link::SpaceX => Desk::SpaceX,
        Link::None => Desk::Home,
    }
}

pub(crate) fn ticker(app: &mut Desktop, root: &mut Ui) {
    let rows = board::rows(&app.snap, now_unix());
    let mut ticks: Vec<Tick> = TAPE
        .iter()
        .filter_map(|(key, symbol)| {
            let row = rows.iter().find(|r| r.key == *key && r.last.is_some())?;
            Some(Tick {
                symbol,
                value: row.text.trim_start_matches('$').to_string(),
                change: row.day,
                hint: format!("{} · {} · {}", row.label, row.source, row.as_of),
                desk: desk_for(row.link),
            })
        })
        .collect();
    if let Some(fng) = app.snap.btc.fng {
        ticks.insert(
            3.min(ticks.len()),
            Tick {
                symbol: "F&G",
                value: format!("{fng} {}", app.snap.btc.fng_label),
                change: None,
                hint: "Crypto Fear & Greed · Alternative.me".into(),
                desk: Desk::Btc,
            },
        );
    }
    egui::Panel::top("ticker")
        .exact_size(30.0)
        .frame(Frame::new().fill(BG()).inner_margin(Margin::symmetric(8, 0)))
        .show(root, |ui| {
            let full = ui.max_rect();
            let painter = ui.painter().clone();
            let measured: Vec<_> = ticks
                .iter()
                .map(|t| {
                    let symbol = painter.layout_no_wrap(t.symbol.into(), mono(10.0), FAINT());
                    let value = painter.layout_no_wrap(t.value.clone(), mono(12.0), TEXT());
                    let (text, color) = widgets::delta(t.change);
                    let change = t
                        .change
                        .is_some()
                        .then(|| painter.layout_no_wrap(text, mono(11.0), color));
                    let width = symbol.size().x
                        + 6.0
                        + value.size().x
                        + change.as_ref().map(|g| g.size().x + 6.0).unwrap_or(0.0);
                    (symbol, value, change, width)
                })
                .collect();
            let mut used = 0.0;
            let mut count = 0;
            for (_, _, _, w) in &measured {
                if used + w + 28.0 > full.width() {
                    break;
                }
                used += w + 28.0;
                count += 1;
            }
            let extra = if count > 0 {
                ((full.width() - used) / count as f32).min(18.0)
            } else {
                0.0
            };
            let mut x = full.left();
            for (i, (symbol, value, change, w)) in measured.into_iter().take(count).enumerate() {
                let cell_w = w + 28.0 + extra;
                let cell = Rect::from_min_size(Pos2::new(x, full.top()), Vec2::new(cell_w, full.height()));
                let response = ui
                    .interact(cell, ui.id().with(("tick", i)), Sense::click())
                    .on_hover_text(&ticks[i].hint);
                if response.hovered() {
                    painter.rect_filled(cell.shrink2(Vec2::new(0.0, 3.0)), crate::theme::corner(4), RAISED());
                    ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                }
                if response.clicked() {
                    app.navigate(ticks[i].desk);
                }
                let mut cursor = cell.left() + 14.0;
                let cy = cell.center().y;
                let sw = symbol.size();
                painter.galley(Pos2::new(cursor, cy - sw.y / 2.0 + 0.5), symbol, FAINT());
                cursor += sw.x + 6.0;
                let vw = value.size();
                painter.galley(Pos2::new(cursor, cy - vw.y / 2.0), value, TEXT());
                cursor += vw.x + 6.0;
                if let Some(change) = change {
                    let cw = change.size();
                    painter.galley(Pos2::new(cursor, cy - cw.y / 2.0), change, TEXT());
                }
                if i + 1 < count {
                    painter.vline(
                        cell.right(),
                        cell.y_range().shrink(8.0),
                        Stroke::new(1.0, BORDER()),
                    );
                }
                x += cell_w;
            }
        });
}

pub(crate) fn sidebar(app: &mut Desktop, root: &mut Ui) {
    let compact = root.ctx().content_rect().width() < 1100.0;
    egui::Panel::left("desks")
        .exact_size(if compact { 56.0 } else { 214.0 })
        .frame(Frame::new().fill(BG()).inner_margin(Margin {
            left: 10,
            right: 10,
            top: 14,
            bottom: 12,
        }))
        .show(root, |ui| {
            if !compact {
                ui.label(caps_job("Desks", 10.0, FAINT()));
                ui.add_space(4.0);
            }
            for desk in Desk::all() {
                let active = app.desk == desk;
                let key = desk.key().to_string();
                let response = nav_row(
                    ui,
                    compact,
                    if compact { &key } else { desk.title() },
                    &key,
                    active.then(|| desk.accent()),
                    None,
                )
                .on_hover_text(format!("{} · press {key}", desk.title()));
                if response.clicked() {
                    app.navigate(desk);
                }
            }
            ui.add_space(18.0);
            if !compact {
                ui.label(caps_job("Workspace", 10.0, FAINT()));
                ui.add_space(4.0);
            }
            let now = now_unix();
            let issues = app.snap.notes.len();
            let stale = feeds::FEEDS
                .iter()
                .filter(|f| {
                    feeds::freshness(f, app.snap.feeds.get(f.id).copied(), now)
                        == Freshness::Stale
                })
                .count();
            let stale = stale + crate::desks::stale_research(app, now);
            let source_badge = if issues > 0 {
                Some((issues.to_string(), AMBER()))
            } else if stale > 0 {
                Some((stale.to_string(), FAINT()))
            } else {
                None
            };
            let armed = app.prefs.alerts.len();
            for (title, short, badge, which) in [
                ("Sources & freshness", "S", source_badge, 0),
                (
                    "Price alerts",
                    "A",
                    (armed > 0).then(|| (armed.to_string(), FAINT())),
                    1,
                ),
                ("Display & settings", "D", None, 2),
            ] {
                let response = nav_row(
                    ui,
                    compact,
                    if compact { short } else { title },
                    "",
                    None,
                    badge,
                )
                .on_hover_text(title);
                if response.clicked() {
                    match which {
                        0 => app.show_sources = true,
                        1 => app.show_alerts = true,
                        _ => app.show_settings = true,
                    }
                }
            }
            if !compact {
                ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
                    ui.spacing_mut().item_spacing.y = 3.0;
                    let next = app.next_refresh_secs();
                    ui.label(
                        RichText::new(if app.refreshing {
                            "Refreshing sources…".to_string()
                        } else {
                            format!("Next refresh in {}", minutes(next))
                        })
                        .font(sans(11.0))
                        .color(FAINT()),
                    );
                    let sources = if app.snap.fetched_unix > 0 {
                        format!("Sources {}", fmt_age(app.snap.fetched_unix, now))
                    } else {
                        "Sources: dated snapshot".into()
                    };
                    status_line(ui, &sources, if issues > 0 { AMBER() } else { MUTED() });
                    let live = live_spot_count(app, now);
                    let label = match live {
                        3 => "Crypto live".to_string(),
                        0 => "Crypto cached".to_string(),
                        n => format!("Crypto {n}/3 live"),
                    };
                    status_line(
                        ui,
                        &label,
                        if live == 3 { GREEN() } else if live > 0 { AMBER() } else { FAINT() },
                    );
                });
            }
        });
}

fn status_line(ui: &mut Ui, text: &str, color: Color32) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        let (rect, _) = ui.allocate_exact_size(Vec2::new(6.0, 12.0), Sense::hover());
        ui.painter().circle_filled(rect.center(), 3.0, color);
        ui.label(RichText::new(text).font(sans(11.0)).color(MUTED()));
    });
}

/// Sidebar entry: accent rule when active, shortcut or badge on the right.
fn nav_row(
    ui: &mut Ui,
    compact: bool,
    title: &str,
    key: &str,
    active: Option<Color32>,
    badge: Option<(String, Color32)>,
) -> Response {
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 30.0), Sense::click());
    let painter = ui.painter();
    if active.is_some() {
        painter.rect_filled(rect, crate::theme::corner(5), RAISED());
    } else if response.hovered() {
        painter.rect_filled(rect, crate::theme::corner(5), RAISED().gamma_multiply(0.7));
    }
    if let Some(accent) = active {
        painter.rect_filled(
            Rect::from_min_size(
                rect.left_top() + Vec2::new(0.0, 7.0),
                Vec2::new(2.0, rect.height() - 14.0),
            ),
            crate::theme::corner(1),
            accent,
        );
    }
    let color = if active.is_some() || response.hovered() {
        TEXT()
    } else {
        MUTED()
    };
    if compact {
        painter.text(rect.center(), Align2::CENTER_CENTER, title, mono(12.0), color);
    } else {
        widgets::cell_text(
            painter,
            Rect::from_min_max(rect.min + Vec2::new(12.0, 0.0), rect.max - Vec2::new(28.0, 0.0)),
            title,
            sans(13.0),
            color,
            Align::Min,
        );
        if !key.is_empty() {
            painter.text(
                rect.right_center() - Vec2::new(10.0, 0.0),
                Align2::RIGHT_CENTER,
                key,
                mono(10.5),
                FAINT(),
            );
        }
    }
    if let Some((text, tone)) = badge {
        let galley = painter.layout_no_wrap(text, mono(10.0), tone);
        let size = galley.size() + Vec2::new(10.0, 4.0);
        let center = if compact {
            rect.right_top() + Vec2::new(-8.0, 8.0)
        } else {
            rect.right_center() - Vec2::new(10.0 + size.x / 2.0, 0.0)
        };
        let pill = Rect::from_center_size(center, size);
        painter.rect_filled(pill, crate::theme::corner(4), tone.gamma_multiply(0.14));
        painter.galley(pill.center() - galley.size() / 2.0, galley, tone);
    }
    if response.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    response
}

fn live_spot_count(app: &Desktop, now: i64) -> usize {
    ["BTC", "ETH", "ZEC"]
        .into_iter()
        .filter(|symbol| app.snap.price_stamp(symbol).is_some_and(|stamp| stamp.live_spot(now)))
        .count()
}

pub(crate) fn status_bar(app: &mut Desktop, root: &mut Ui) {
    egui::Panel::bottom("status")
        .exact_size(24.0)
        .frame(
            Frame::new()
                .fill(SURFACE())
                .inner_margin(Margin::symmetric(12, 0)),
        )
        .show(root, |ui| {
            ui.horizontal_centered(|ui| {
                ui.spacing_mut().item_spacing.x = 10.0;
                let now = now_unix();
                let live = live_spot_count(app, now);
                let (state, color) = if app.refreshing {
                    ("REFRESHING", AMBER())
                } else if live == 3 {
                    ("CRYPTO LIVE", GREEN())
                } else if live > 0 {
                    ("PARTIAL LIVE", AMBER())
                } else if app.snap.fetched_unix == 0 {
                    ("SNAPSHOT", MUTED())
                } else {
                    ("CACHED", MUTED())
                };
                let (rect, _) = ui.allocate_exact_size(Vec2::new(6.0, 12.0), Sense::hover());
                ui.painter().circle_filled(rect.center(), 3.0, color);
                ui.label(RichText::new(state).font(mono(10.5)).color(color));
                let sources = if app.snap.fetched_unix > 0 {
                    format!(
                        "sources {} · next in {}",
                        fmt_age(app.snap.fetched_unix, now),
                        minutes(app.next_refresh_secs())
                    )
                } else {
                    "no refresh yet".into()
                };
                ui.label(RichText::new(sources).font(mono(10.5)).color(FAINT()));
                let issues = app.snap.notes.len();
                if issues > 0 {
                    let text = if issues == 1 {
                        "1 feed issue".to_string()
                    } else {
                        format!("{issues} feed issues")
                    };
                    if widgets::link(ui, &text, AMBER())
                        .on_hover_text("Open Sources & freshness")
                        .clicked()
                    {
                        app.show_sources = true;
                    }
                }
                if !app.cache_error.is_empty() {
                    ui.label(
                        RichText::new(format!("cache: {}", app.cache_error))
                            .font(mono(10.5))
                            .color(AMBER()),
                    );
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let local = Local::now();
                    let utc = chrono::Utc::now();
                    ui.label(
                        RichText::new(format!(
                            "{} · {} UTC",
                            local.format("%a %-d %b  %H:%M:%S"),
                            utc.format("%H:%M")
                        ))
                        .font(mono(10.5))
                        .color(MUTED()),
                    );
                    ui.label(
                        RichText::new("1–9 desks · R refresh · Ctrl K search")
                            .font(mono(10.5))
                            .color(FAINT()),
                    );
                });
            });
        });
}

pub(crate) fn notice(app: &mut Desktop, ui: &mut Ui) {
    Frame::new()
        .fill(AMBER().gamma_multiply(0.08))
        .stroke(Stroke::new(1.0, AMBER().gamma_multiply(0.35)))
        .corner_radius(crate::theme::corner(5))
        .inner_margin(Margin::symmetric(12, 7))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(RichText::new(&app.notice).font(sans(12.5)).color(AMBER()));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui
                        .add(egui::Button::new(RichText::new("Dismiss").font(sans(11.5))).small())
                        .clicked()
                    {
                        app.notice.clear();
                    }
                });
            });
        });
}
