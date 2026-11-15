//! Modal dialogs: command palette, sources and freshness, settings, price alerts.

use crate::app::{Desk, Desktop};
use crate::theme::{
    medium, mono, sans, AMBER, BORDER, BORDER_STRONG, FAINT, GREEN, HOVER, MUTED, SURFACE, TEXT,
};
use crate::widgets::{self, caps_job, col, W};
use dollar_dashboard::feeds::{self, Freshness, FEEDS};
use dollar_dashboard::model::{self, dollar_metrics, fmt_age, now_unix};
use dollar_dashboard::workspace::{instruments, Direction, PriceAlert};
use eframe::egui::{
    self, Align, CursorIcon, Frame, Id, Key, Margin, Modal, RichText, ScrollArea,
    Sense, Stroke, TextEdit, Ui, Vec2,
};

pub(crate) fn show(app: &mut Desktop, ctx: &egui::Context) {
    if app.show_search {
        palette(app, ctx);
    } else if app.show_sources {
        sources(app, ctx);
    } else if app.show_settings {
        settings(app, ctx);
    } else if app.show_alerts {
        alerts(app, ctx);
    }
}

fn modal_frame() -> Frame {
    Frame::new()
        .fill(SURFACE())
        .stroke(Stroke::new(1.0, BORDER_STRONG()))
        .corner_radius(crate::theme::corner(8))
        .inner_margin(Margin::same(0))
}

/// Title row and body padding shared by the non-palette dialogs.
fn dialog<R>(
    ctx: &egui::Context,
    id: &str,
    width: f32,
    title: &str,
    meta: &str,
    body: impl FnOnce(&mut Ui) -> R,
) -> bool {
    let max_h = (ctx.content_rect().height() * 0.82).max(300.0);
    let response = Modal::new(Id::new(id)).frame(modal_frame()).show(ctx, |ui| {
        ui.set_width(width.min(ctx.content_rect().width() - 48.0));
        Frame::new()
            .inner_margin(Margin::symmetric(18, 14))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(title).font(medium(16.0)).color(TEXT()));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new(meta).font(sans(11.5)).color(FAINT()));
                    });
                });
            });
        let rect = ui.min_rect();
        ui.painter()
            .hline(rect.x_range(), ui.cursor().top(), Stroke::new(1.0, BORDER()));
        let mut close = false;
        Frame::new()
            .inner_margin(Margin::symmetric(18, 14))
            .show(ui, |ui| {
                ScrollArea::vertical()
                    .max_height(max_h - 120.0)
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        body(ui);
                    });
                ui.add_space(10.0);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Close").on_hover_text("Esc").clicked() {
                        close = true;
                    }
                });
            });
        close
    });
    response.inner || response.should_close()
}

#[derive(Clone)]
enum Target {
    Desk(Desk, Option<String>),
    Filter(Desk, String),
    Sources,
}

struct Hit {
    title: String,
    kind: &'static str,
    detail: String,
    target: Target,
}

fn corpus(app: &Desktop) -> Vec<Hit> {
    let mut hits: Vec<Hit> = Desk::all()
        .into_iter()
        .map(|desk| Hit {
            title: desk.title().into(),
            kind: "Desk",
            detail: format!("press {}", desk.key()),
            target: Target::Desk(desk, None),
        })
        .collect();
    for row in dollar_dashboard::board::rows(&app.snap, now_unix()) {
        if row.last.is_none() {
            continue;
        }
        let desk = match row.link {
            dollar_dashboard::board::Link::Bitcoin => Desk::Btc,
            dollar_dashboard::board::Link::Dollar => Desk::Dollar,
            dollar_dashboard::board::Link::Nuclear => Desk::Nuclear,
            dollar_dashboard::board::Link::Zcash => Desk::Zcash,
            dollar_dashboard::board::Link::SpaceX => Desk::SpaceX,
            dollar_dashboard::board::Link::None => Desk::Home,
        };
        hits.push(Hit {
            title: format!("{} · {}", row.label, row.text),
            kind: "Market",
            detail: format!("{} {}", row.key, row.group),
            target: Target::Desk(desk, None),
        });
    }
    for q in instruments(&app.snap) {
        hits.push(Hit {
            title: format!("{} · {}", q.symbol, model::fmt_usd(q.price)),
            kind: "Quote",
            detail: crate::desks::name_of(app, &q.symbol),
            target: Target::Filter(Desk::Markets, q.symbol.clone()),
        });
    }
    for metric in dollar_metrics(&app.snap) {
        hits.push(Hit {
            title: format!("{} · {}", metric.label, metric.value),
            kind: "Metric",
            detail: metric.group.title().into(),
            target: Target::Desk(Desk::Dollar, Some(metric.id.into())),
        });
    }
    for (desk, prefix, players) in [
        (Desk::Ai, "ai", &app.catalog.ai_players),
        (Desk::Nuclear, "nuc", &app.catalog.nuclear_players),
    ] {
        for p in players {
            hits.push(Hit {
                title: p.name.clone(),
                kind: "Company",
                detail: format!("{} {}", p.ticker.as_deref().unwrap_or(""), p.category),
                target: Target::Desk(desk, Some(format!("{prefix}:{}", p.id))),
            });
        }
    }
    for (desk, prefix, kind, notes) in [
        (Desk::Ai, "aim", "Fact", &app.catalog.ai_metrics),
        (Desk::Ai, "ais", "Signal", &app.catalog.ai_signals),
        (Desk::Nuclear, "nucm", "Fact", &app.catalog.nuclear_metrics),
        (Desk::Nuclear, "nucs", "Signal", &app.catalog.nuclear_signals),
        (Desk::SpaceX, "spxm", "Fact", &app.catalog.spacex_metrics),
        (Desk::SpaceX, "spxs", "Signal", &app.catalog.spacex_signals),
    ] {
        for n in notes {
            hits.push(Hit {
                title: n.label.clone(),
                kind,
                detail: format!("{} {}", n.group, n.value),
                target: Target::Desk(desk, Some(format!("{prefix}:{}", n.id))),
            });
        }
    }
    for feed in FEEDS {
        hits.push(Hit {
            title: format!("{} · {}", feed.provider, feed.covers),
            kind: "Source",
            detail: feed.id.into(),
            target: Target::Sources,
        });
    }
    hits
}

fn palette(app: &mut Desktop, ctx: &egui::Context) {
    let query = app.search.trim().to_lowercase();
    let mut hits: Vec<(usize, Hit)> = corpus(app)
        .into_iter()
        .filter_map(|hit| {
            if query.is_empty() {
                return (hit.kind == "Desk").then_some((0, hit));
            }
            let title = hit.title.to_lowercase();
            let haystack = format!("{title} {}", hit.detail.to_lowercase());
            if title.starts_with(&query) {
                Some((0, hit))
            } else if title.contains(&query) {
                Some((1, hit))
            } else if haystack.contains(&query) {
                Some((2, hit))
            } else {
                None
            }
        })
        .collect();
    hits.sort_by_key(|(rank, _)| *rank);
    let hits: Vec<Hit> = hits.into_iter().map(|(_, hit)| hit).take(40).collect();
    if !hits.is_empty() {
        let (down, up) = ctx.input(|i| (i.key_pressed(Key::ArrowDown), i.key_pressed(Key::ArrowUp)));
        if down {
            app.search_index = (app.search_index + 1).min(hits.len() - 1);
        }
        if up {
            app.search_index = app.search_index.saturating_sub(1);
        }
        app.search_index = app.search_index.min(hits.len() - 1);
    }
    let enter = ctx.input(|i| i.key_pressed(Key::Enter));
    let mut chosen: Option<Target> = None;
    let width = 600.0_f32.min(ctx.content_rect().width() - 48.0);
    let response = Modal::new(Id::new("palette"))
        .frame(modal_frame())
        .area(
            Modal::default_area(Id::new("palette-area"))
                .anchor(egui::Align2::CENTER_TOP, Vec2::new(0.0, 90.0)),
        )
        .show(ctx, |ui| {
            ui.set_width(width);
            Frame::new()
                .inner_margin(Margin::symmetric(16, 12))
                .show(ui, |ui| {
                    let edit = ui.add(
                        TextEdit::singleline(&mut app.search)
                            .hint_text("Search desks, tickers, companies, metrics, sources")
                            .font(sans(15.0))
                            .frame(Frame::new())
                            .desired_width(f32::INFINITY),
                    );
                    edit.request_focus();
                    if edit.changed() {
                        app.search_index = 0;
                    }
                });
            ui.painter().hline(
                ui.min_rect().x_range(),
                ui.cursor().top(),
                Stroke::new(1.0, BORDER()),
            );
            Frame::new()
                .inner_margin(Margin::symmetric(8, 8))
                .show(ui, |ui| {
                    if hits.is_empty() {
                        widgets::empty(ui, 60.0, "No matches. Try a ticker, company, metric or source.");
                        return;
                    }
                    ScrollArea::vertical()
                        .max_height(380.0)
                        .auto_shrink([false, true])
                        .show(ui, |ui| {
                            for (i, hit) in hits.iter().enumerate() {
                                let selected = i == app.search_index;
                                let (rect, response) = ui.allocate_exact_size(
                                    Vec2::new(ui.available_width(), 32.0),
                                    Sense::click(),
                                );
                                if selected || response.hovered() {
                                    ui.painter().rect_filled(
                                        rect,
                                        crate::theme::corner(5),
                                        if selected { HOVER() } else { HOVER().gamma_multiply(0.6) },
                                    );
                                }
                                if selected {
                                    response.scroll_to_me(None);
                                }
                                let painter = ui.painter();
                                let kind_w = 64.0;
                                let kind_rect = egui::Rect::from_min_size(
                                    rect.min + Vec2::new(10.0, 0.0),
                                    Vec2::new(kind_w, rect.height()),
                                );
                                let kind = painter.layout_job(caps_job(hit.kind, 9.5, FAINT()));
                                painter.galley(
                                    egui::Pos2::new(kind_rect.left(), kind_rect.center().y - kind.size().y / 2.0),
                                    kind,
                                    FAINT(),
                                );
                                let detail_w = (rect.width() * 0.32).min(220.0);
                                let title_rect = egui::Rect::from_min_max(
                                    egui::Pos2::new(kind_rect.right() + 6.0, rect.top()),
                                    egui::Pos2::new(rect.right() - detail_w - 16.0, rect.bottom()),
                                );
                                widgets::cell_text(painter, title_rect, &hit.title, sans(13.0), TEXT(), Align::Min);
                                let detail_rect = egui::Rect::from_min_max(
                                    egui::Pos2::new(rect.right() - detail_w - 10.0, rect.top()),
                                    egui::Pos2::new(rect.right() - 10.0, rect.bottom()),
                                );
                                widgets::cell_text(painter, detail_rect, &hit.detail, sans(11.5), FAINT(), Align::Max);
                                if response.hovered() {
                                    ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                                }
                                if response.clicked() {
                                    chosen = Some(hit.target.clone());
                                }
                            }
                        });
                });
            ui.painter().hline(
                ui.min_rect().x_range(),
                ui.cursor().top(),
                Stroke::new(1.0, BORDER()),
            );
            Frame::new()
                .inner_margin(Margin::symmetric(16, 8))
                .show(ui, |ui| {
                    ui.label(
                        RichText::new("↑ ↓ choose · Enter open · Esc close")
                            .font(mono(10.5))
                            .color(FAINT()),
                    );
                });
        });
    if enter {
        chosen = hits.get(app.search_index).map(|hit| hit.target.clone());
    }
    if let Some(target) = chosen {
        match target {
            Target::Desk(desk, id) => {
                app.navigate(desk);
                app.selected = id;
            }
            Target::Filter(desk, filter) => {
                app.navigate(desk);
                app.filter = filter;
            }
            Target::Sources => app.show_sources = true,
        }
        app.show_search = false;
        app.search.clear();
        app.search_index = 0;
    } else if response.should_close() {
        app.show_search = false;
    }
}

fn sources(app: &mut Desktop, ctx: &egui::Context) {
    let now = now_unix();
    let refreshed = if app.snap.fetched_unix > 0 {
        format!("refreshed {}", fmt_age(app.snap.fetched_unix, now))
    } else {
        "no refresh yet".into()
    };
    let mut refresh = false;
    let close = dialog(ctx, "sources", 760.0, "Sources & freshness", &refreshed, |ui| {
        let states: Vec<Freshness> = FEEDS
            .iter()
            .map(|f| feeds::freshness(f, app.snap.feeds.get(f.id).copied(), now))
            .collect();
        let count = |s: Freshness| states.iter().filter(|x| **x == s).count();
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!(
                    "{} feeds · {} fresh · {} stale · {} not seen yet",
                    FEEDS.len(),
                    count(Freshness::Fresh),
                    count(Freshness::Stale),
                    count(Freshness::Never)
                ))
                .font(sans(12.5))
                .color(MUTED()),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add_enabled(!app.refreshing, egui::Button::new("Refresh now"))
                    .clicked()
                {
                    refresh = true;
                }
            });
        });
        ui.add_space(8.0);
        if !app.snap.notes.is_empty() {
            widgets::section(ui, "Issues from the last refresh", "previous values are kept");
            for note in &app.snap.notes {
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("•").color(AMBER()));
                    ui.label(RichText::new(note).font(sans(12.5)).color(TEXT()));
                });
            }
            ui.add_space(10.0);
        }
        widgets::section(ui, "Feeds", "last successful response");
        let cols = [
            col("Provider", W::Px(130.0), Align::Min),
            col("Covers", W::Flex(1.0), Align::Min),
            col("Last response", W::Px(96.0), Align::Max),
            col("State", W::Px(64.0), Align::Max),
        ];
        let (layout, _) = widgets::table_header(ui, &cols, None);
        for (feed, state) in FEEDS.iter().zip(states) {
            let (rect, _) = widgets::table_row(ui, 26.0, false, false);
            let painter = ui.painter();
            widgets::cell_text(painter, layout.cell(rect, 0), feed.provider, sans(12.5), TEXT(), Align::Min);
            widgets::cell_text(painter, layout.cell(rect, 1), feed.covers, sans(12.0), MUTED(), Align::Min);
            let last = app
                .snap
                .feeds
                .get(feed.id)
                .map(|t| fmt_age(*t, now))
                .unwrap_or_else(|| "—".into());
            widgets::cell_text(painter, layout.cell(rect, 2), &last, mono(11.5), MUTED(), Align::Max);
            let (label, color) = match state {
                Freshness::Fresh => ("fresh", GREEN()),
                Freshness::Stale => ("stale", AMBER()),
                Freshness::Never => ("—", FAINT()),
            };
            widgets::cell_text(painter, layout.cell(rect, 3), label, mono(11.5), color, Align::Max);
        }
        ui.add_space(12.0);
        widgets::section(ui, "Research snapshots", "bundled with the app, refreshed by hand");
        let cols = [
            col("Snapshot", W::Px(130.0), Align::Min),
            col("Covers", W::Flex(1.0), Align::Min),
            col("Evidence", W::Px(96.0), Align::Max),
            col("State", W::Px(64.0), Align::Max),
        ];
        let (layout, _) = widgets::table_header(ui, &cols, None);
        for (name, covers, as_of) in [
            (
                "AI infrastructure",
                "Verdict, private valuations, market structure, what to watch",
                &app.catalog.ai_as_of,
            ),
            (
                "Nuclear energy",
                "Verdict, fleet facts, restarts, fuel, hyperscaler deals",
                &app.catalog.nuclear_as_of,
            ),
        ] {
            let (rect, _) = widgets::table_row(ui, 26.0, false, false);
            let painter = ui.painter();
            widgets::cell_text(painter, layout.cell(rect, 0), name, sans(12.5), TEXT(), Align::Min);
            widgets::cell_text(painter, layout.cell(rect, 1), covers, sans(12.0), MUTED(), Align::Min);
            widgets::cell_text(
                painter,
                layout.cell(rect, 2),
                &model::pretty_date(as_of),
                mono(11.5),
                MUTED(),
                Align::Max,
            );
            let stale = crate::desks::evidence_days(as_of, now)
                .is_some_and(|d| d > crate::desks::RESEARCH_STALE_DAYS);
            let (label, color) = if stale { ("stale", AMBER()) } else { ("fresh", GREEN()) };
            widgets::cell_text(painter, layout.cell(rect, 3), label, mono(11.5), color, Align::Max);
        }
        ui.add_space(12.0);
        widgets::section(ui, "How to read the data", "");
        for line in [
            "Crypto spot ticks arrive every 2 seconds from Binance while the app is open.",
            "Equity, index, FX and futures quotes are delayed TradingView prints; daily history comes from CNBC.",
            "Treasury yields on the Dollar desk are official daily closes; the board shows the delayed session yield.",
            "AI and nuclear company facts are dated research snapshots shipped with the app.",
            "Status bands are transparent rules of thumb, not signals or advice.",
        ] {
            ui.label(RichText::new(line).font(sans(12.5)).color(MUTED()));
        }
        ui.add_space(6.0);
        ui.label(
            RichText::new(format!(
                "Cache: {} · Bitcoin history fetched {}",
                dollar_dashboard::cache::cache_path().display(),
                if app.snap.history_unix > 0 {
                    fmt_age(app.snap.history_unix, now)
                } else {
                    "never".into()
                }
            ))
            .font(mono(10.5))
            .color(FAINT()),
        );
    });
    if refresh {
        app.kick();
    }
    if close {
        app.show_sources = false;
    }
}

fn settings(app: &mut Desktop, ctx: &egui::Context) {
    let mut scale_changed = false;
    let close = dialog(ctx, "settings", 460.0, "Display & settings", "saved on this computer", |ui| {
        widgets::section(ui, "Display", "");
        if ui
            .add(egui::Slider::new(&mut app.prefs.scale, 0.85..=1.35).text("Interface scale"))
            .changed()
        {
            scale_changed = true;
        }
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new("Default chart range").font(sans(12.5)).color(MUTED()));
            widgets::segmented(
                ui,
                &mut app.prefs.chart_days,
                &[("1M", 30), ("3M", 90), ("1Y", 365), ("3Y", 1095), ("All", 0)],
            );
        });
        ui.add_space(12.0);
        widgets::section(ui, "Data", "");
        ui.add(
            egui::Slider::new(&mut app.prefs.refresh_minutes, 1..=60)
                .text("Source refresh, minutes"),
        );
        ui.label(
            RichText::new("Crypto spot ticks refresh every 2 seconds, every 10 in the background. Daily histories refresh every 2 hours.")
                .font(sans(11.5))
                .color(FAINT()),
        );
        ui.checkbox(
            &mut app.prefs.rule_alerts,
            "Notify when a dollar rule changes status",
        );
        ui.add_space(12.0);
        widgets::section(ui, "Keyboard", "");
        for (key, what) in [
            ("1 – 7", "switch desks"),
            ("Ctrl K", "search desks, tickers, metrics and sources"),
            ("R", "refresh all sources"),
            ("Esc", "close dialogs and the detail panel"),
        ] {
            ui.horizontal(|ui| {
                let (rect, _) = ui.allocate_exact_size(Vec2::new(64.0, 20.0), Sense::hover());
                ui.painter().text(
                    rect.left_center(),
                    egui::Align2::LEFT_CENTER,
                    key,
                    mono(11.5),
                    TEXT(),
                );
                ui.label(RichText::new(what).font(sans(12.5)).color(MUTED()));
            });
        }
    });
    if scale_changed {
        ctx.set_zoom_factor(app.prefs.scale);
    }
    if close {
        app.show_settings = false;
    }
}

fn alerts(app: &mut Desktop, ctx: &egui::Context) {
    let live = |symbol: &str, app: &Desktop| match symbol {
        "BTC" => app.snap.btc.price,
        "ETH" => app.snap.btc.eth_price,
        "ZEC" => app.snap.btc.zec_price,
        _ => None,
    };
    let close = dialog(
        ctx,
        "alerts",
        600.0,
        "Price alerts",
        "checked on live spot ticks while the app is open",
        |ui| {
            widgets::section(ui, "New alert", "");
            ui.horizontal_wrapped(|ui| {
                egui::ComboBox::from_id_salt("alert-symbol")
                    .selected_text(&app.alert_symbol)
                    .width(70.0)
                    .show_ui(ui, |ui| {
                        for symbol in ["BTC", "ETH", "ZEC"] {
                            ui.selectable_value(&mut app.alert_symbol, symbol.into(), symbol);
                        }
                    });
                widgets::segmented(
                    ui,
                    &mut app.alert_direction,
                    &[("At or above", Direction::Above), ("At or below", Direction::Below)],
                );
                let hint = live(&app.alert_symbol, app)
                    .map(|p| format!("now {}", model::fmt_usd(p)))
                    .unwrap_or_else(|| "USD target".into());
                ui.add(
                    TextEdit::singleline(&mut app.alert_value)
                        .desired_width(120.0)
                        .hint_text(hint),
                );
                let threshold = app
                    .alert_value
                    .trim()
                    .trim_start_matches('$')
                    .replace(',', "")
                    .parse::<f64>()
                    .ok()
                    .filter(|v| v.is_finite() && *v > 0.0);
                if ui
                    .add_enabled(threshold.is_some(), egui::Button::new("Add alert"))
                    .clicked()
                {
                    if let Some(threshold) = threshold {
                        app.prefs.alerts.push(PriceAlert {
                            symbol: app.alert_symbol.clone(),
                            direction: app.alert_direction,
                            threshold,
                            triggered_at: None,
                        });
                        app.alert_value.clear();
                    }
                }
            });
            ui.add_space(12.0);
            widgets::section(ui, "Alerts", &format!("{} saved", app.prefs.alerts.len()));
            if app.prefs.alerts.is_empty() {
                widgets::empty(ui, 40.0, "No alerts yet. Add a level above.");
                return;
            }
            let cols = [
                col("Symbol", W::Px(56.0), Align::Min),
                col("Condition", W::Flex(1.0), Align::Min),
                col("Distance", W::Px(80.0), Align::Max),
                col("State", W::Px(150.0), Align::Min),
                col("", W::Px(124.0), Align::Max),
            ];
            let (layout, _) = widgets::table_header(ui, &cols, None);
            let mut remove = None;
            let prices: Vec<Option<f64>> = app
                .prefs
                .alerts
                .iter()
                .map(|a| live(&a.symbol, app))
                .collect();
            for (i, alert) in app.prefs.alerts.iter_mut().enumerate() {
                let (rect, _) = widgets::table_row(ui, 30.0, false, false);
                let painter = ui.painter();
                widgets::cell_text(painter, layout.cell(rect, 0), &alert.symbol, mono(12.5), TEXT(), Align::Min);
                let condition = format!(
                    "{} {}",
                    if alert.direction == Direction::Above { "≥" } else { "≤" },
                    model::fmt_usd(alert.threshold)
                );
                widgets::cell_text(painter, layout.cell(rect, 1), &condition, mono(12.5), TEXT(), Align::Min);
                let distance = prices[i]
                    .map(|p| model::fmt_change_pct((alert.threshold / p - 1.0) * 100.0))
                    .unwrap_or_else(|| "—".into());
                widgets::cell_text(painter, layout.cell(rect, 2), &distance, mono(12.0), MUTED(), Align::Max);
                let (state, color) = match alert.triggered_at {
                    Some(t) => (
                        format!("Fired {}", crate::chrome::local_label(t, now_unix())),
                        AMBER(),
                    ),
                    None => ("Watching".to_string(), GREEN()),
                };
                widgets::cell_text(painter, layout.cell(rect, 3), &state, sans(12.0), color, Align::Min);
                let actions = layout.cell(rect, 4);
                let mut child = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(actions)
                        .layout(egui::Layout::right_to_left(egui::Align::Center)),
                );
                if child.small_button("Remove").clicked() {
                    remove = Some(i);
                }
                if alert.triggered_at.is_some() && child.small_button("Rearm").clicked() {
                    alert.triggered_at = None;
                }
            }
            if let Some(i) = remove {
                app.prefs.alerts.remove(i);
            }
            ui.add_space(6.0);
            ui.label(
                RichText::new("A triggered alert fires once, shows a banner and a desktop notification, and waits to be rearmed.")
                    .font(sans(11.5))
                    .color(FAINT()),
            );
        },
    );
    if close {
        app.show_alerts = false;
    }
}
