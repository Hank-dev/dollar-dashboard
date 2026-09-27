use chrono::Timelike;

use crate::app::{Desk, Desktop};
use crate::draw::{
    self, AMBER, BLUE, BORDER, CYAN, FAINT, GREEN, HOVER, MUTED, PURPLE, RED, SURFACE, TEXT,
};
use dollar_dashboard::catalog::{NoteCard, PlayerCard, NUCLEAR_TICKERS, URANIUM_ETFS};
use dollar_dashboard::model::{
    self, dollar_metrics, fit_power_law, headline, now_unix, power_value, power_z, quote, regime,
    trailing_mean, weighted_change, Group, Metric, PowerFit, Status, LAST_HALVING_UNIX,
    STANDING_NOTE,
};
use eframe::egui::{
    self, Align, Align2, Color32, CornerRadius, Frame, Layout, Margin, Rect, RichText, ScrollArea,
    Sense, Stroke, StrokeKind, TextEdit, Ui, Vec2,
};

pub(crate) fn desk(app: &mut Desktop, ui: &mut Ui) {
    match app.desk {
        Desk::Home => home(app, ui),
        Desk::Btc => btc(app, ui),
        Desk::Dollar => dollar(app, ui),
        Desk::Ai => ai(app, ui),
        Desk::Nuclear => nuclear(app, ui),
    }
}

fn home(app: &mut Desktop, ui: &mut Ui) {
    let dxy = app.snap.dollar.dxy.as_ref().map(|o| o.value);
    let vix = app.snap.dollar.vix.as_ref().map(|o| o.value);
    let tone = regime(app.snap.btc.change_24h_pct, vix, app.snap.btc.fng, dxy);
    let line = headline(app.snap.btc.change_24h_pct, vix, app.snap.btc.fng, dxy);
    let accent = match tone {
        model::Regime::RiskOn => GREEN,
        model::Regime::RiskOff => RED,
        model::Regime::Mixed => AMBER,
    };
    Frame::new()
        .fill(SURFACE)
        .stroke(Stroke::new(1.0, BORDER))
        .inner_margin(Margin::symmetric(16, 14))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(tone.label())
                        .monospace()
                        .size(12.0)
                        .color(accent),
                );
                ui.label(
                    RichText::new(session_line())
                        .monospace()
                        .size(12.0)
                        .color(MUTED),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(
                        RichText::new(chrono::Utc::now().format("%A %d %b").to_string())
                            .monospace()
                            .size(12.0)
                            .color(FAINT),
                    );
                });
            });
            ui.add_space(6.0);
            ui.label(RichText::new(line).size(22.0).color(TEXT));
        });
    ui.add_space(12.0);
    let width = ui.available_width();
    if width < 80.0 {
        return;
    }
    let cols = if width >= 1080.0 {
        4
    } else if width >= 680.0 {
        2
    } else {
        1
    };
    let gap = 10.0;
    let card_w = (width - gap * (cols as f32 - 1.0)) / cols as f32;
    let cards = home_cards(app);
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = Vec2::splat(gap);
        for card in cards {
            let (rect, response) = ui.allocate_exact_size(Vec2::new(card_w, 214.0), Sense::click());
            paint_home_card(ui, rect, response.hovered(), &card);
            if response.clicked() {
                app.desk = card.desk;
                app.selected = None;
            }
        }
    });
    ui.add_space(8.0);
    ui.label(
        RichText::new("Click a desk, or press 1–5. Public prints refresh in the background. AI and nuclear notes stay dated snapshots.")
            .size(12.0)
            .color(FAINT),
    );
}

struct HomeCard {
    desk: Desk,
    kicker: String,
    value: String,
    sub: String,
    lines: [String; 3],
    spark: Vec<f64>,
    spark_color: Color32,
}

fn home_cards(app: &Desktop) -> Vec<HomeCard> {
    let btc_value = app
        .snap
        .btc
        .price
        .map(model::fmt_usd)
        .unwrap_or_else(|| "—".into());
    let btc_sub = app
        .snap
        .btc
        .change_24h_pct
        .map(|v| format!("{} 24h", model::fmt_signed_pct(v)))
        .unwrap_or_else(|| app.snap.btc.price_source.clone());
    let fit = fit_power_law(&app.snap.btc.daily);
    let z = fit.as_ref().and_then(|fit| {
        app.snap
            .btc
            .price
            .and_then(|px| power_z(fit, now_unix(), px))
    });
    let fng = match (app.snap.btc.fng, app.snap.btc.fng_label.as_str()) {
        (Some(v), label) if !label.is_empty() => format!("F&G {v} {label}"),
        (Some(v), _) => format!("F&G {v}"),
        _ => "F&G —".into(),
    };
    let dxy = app
        .snap
        .dollar
        .dxy
        .as_ref()
        .map(|o| format!("DXY {:.2}", o.value))
        .unwrap_or_else(|| "DXY —".into());
    let y10 = app
        .snap
        .dollar
        .ust10
        .as_ref()
        .map(|o| format!("10Y {:.2}%", o.value))
        .unwrap_or_else(|| "10Y —".into());
    let vix = app
        .snap
        .dollar
        .vix
        .as_ref()
        .map(|o| format!("VIX {:.1}", o.value))
        .unwrap_or_else(|| "VIX —".into());
    let jpy = app
        .snap
        .dollar
        .usdjpy
        .as_ref()
        .map(|o| format!("USDJPY {:.1}", o.value))
        .unwrap_or_else(|| "USDJPY —".into());
    let nvda = quote(&app.snap.quotes, "NVDA");
    let ai_value = nvda
        .map(|q| model::fmt_usd(q.price))
        .unwrap_or_else(|| "NVDA —".into());
    let ai_sub = nvda
        .map(|q| {
            format!(
                "{} {}",
                model::fmt_signed_pct(q.change_pct),
                model::fmt_cap(q.market_cap.unwrap_or(0.0))
            )
        })
        .unwrap_or_else(|| format!("snapshot {}", model::pretty_date(&app.catalog.ai_as_of)));
    let nuc_cap = model::total_cap(&app.snap.quotes, NUCLEAR_TICKERS);
    let nuc_chg = weighted_change(&app.snap.quotes, NUCLEAR_TICKERS);
    let ura = quote(&app.snap.quotes, "URA");
    vec![
        HomeCard {
            desk: Desk::Btc,
            kicker: "01  BITCOIN".into(),
            value: btc_value,
            sub: btc_sub,
            lines: [
                z.map(|z| format!("power law {z:+.2}σ"))
                    .unwrap_or_else(|| "power law —".into()),
                fng,
                funding_line(app.snap.btc.funding),
            ],
            spark: app.snap.btc.spark.clone(),
            spark_color: AMBER,
        },
        HomeCard {
            desk: Desk::Dollar,
            kicker: "02  DOLLAR".into(),
            value: dxy,
            sub: y10,
            lines: [vix, jpy, curve_line(app)],
            spark: Vec::new(),
            spark_color: BLUE,
        },
        HomeCard {
            desk: Desk::Ai,
            kicker: "03  AI".into(),
            value: ai_value,
            sub: ai_sub,
            lines: [
                "NVIDIA is the public compute proxy".into(),
                format!("{} names in the snapshot", app.catalog.ai_players.len()),
                format!("notes {}", model::pretty_date(&app.catalog.ai_as_of)),
            ],
            spark: Vec::new(),
            spark_color: PURPLE,
        },
        HomeCard {
            desk: Desk::Nuclear,
            kicker: "04  NUCLEAR".into(),
            value: nuc_cap
                .map(model::fmt_cap)
                .unwrap_or_else(|| "fleet —".into()),
            sub: nuc_chg
                .map(|v| format!("{} cap-weighted", model::fmt_signed_pct(v)))
                .unwrap_or_else(|| "quotes loading".into()),
            lines: [
                ura.map(|q| {
                    format!(
                        "URA {} {}",
                        model::fmt_usd(q.price),
                        model::fmt_signed_pct(q.change_pct)
                    )
                })
                .unwrap_or_else(|| "URA —".into()),
                "400 GWe operable fleet".into(),
                "HALEU still constrained".into(),
            ],
            spark: Vec::new(),
            spark_color: CYAN,
        },
    ]
}

fn paint_home_card(ui: &Ui, rect: Rect, hovered: bool, card: &HomeCard) {
    let painter = ui.painter();
    painter.rect_filled(
        rect,
        CornerRadius::ZERO,
        if hovered { HOVER } else { SURFACE },
    );
    painter.rect_stroke(
        rect,
        CornerRadius::ZERO,
        Stroke::new(1.0, BORDER),
        StrokeKind::Inside,
    );
    painter.rect_filled(
        Rect::from_min_size(rect.left_top(), Vec2::new(rect.width(), 3.0)),
        CornerRadius::ZERO,
        card.desk.accent(),
    );
    let inner = rect.shrink2(Vec2::new(14.0, 12.0));
    draw::label(
        painter,
        inner.left_top() + Vec2::new(0.0, 8.0),
        Align2::LEFT_TOP,
        &card.kicker,
        11.0,
        card.spark_color,
        true,
    );
    draw::label(
        painter,
        inner.left_top() + Vec2::new(0.0, 36.0),
        Align2::LEFT_TOP,
        &card.value,
        26.0,
        TEXT,
        false,
    );
    draw::label(
        painter,
        inner.left_top() + Vec2::new(0.0, 70.0),
        Align2::LEFT_TOP,
        &draw::truncate(&card.sub, 32),
        12.0,
        MUTED,
        true,
    );
    for (index, line) in card.lines.iter().enumerate() {
        draw::label(
            painter,
            inner.left_top() + Vec2::new(0.0, 96.0 + index as f32 * 18.0),
            Align2::LEFT_TOP,
            &draw::truncate(line, 36),
            12.0,
            TEXT,
            false,
        );
    }
    let spark = Rect::from_min_max(
        pos2(inner.left(), inner.bottom() - 36.0),
        pos2(inner.right(), inner.bottom() - 4.0),
    );
    draw::sparkline(painter, spark, &card.spark, card.spark_color);
}

fn dollar(app: &mut Desktop, ui: &mut Ui) {
    let metrics = dollar_metrics(&app.snap);
    kicker(ui, "MACRO AND RATES", BLUE);
    ui.label(
        RichText::new("Dollar and the global funding system")
            .size(24.0)
            .color(TEXT),
    );
    ui.label(
        RichText::new(format!(
            "Treasury close {}",
            model::pretty_date(&app.snap.dollar.curve_date)
        ))
        .monospace()
        .size(12.0)
        .color(FAINT),
    );
    ui.add_space(8.0);
    callout(ui, BLUE, "Rule reading", &model::rule_reading(&metrics));
    ui.add_space(6.0);
    ui.label(RichText::new(STANDING_NOTE).size(13.0).color(MUTED));
    ui.add_space(8.0);
    legend(ui);
    for group in [Group::Rates, Group::Dollar, Group::Risk] {
        ui.add_space(10.0);
        ui.label(
            RichText::new(group.title())
                .monospace()
                .size(12.0)
                .color(MUTED),
        );
        ui.add_space(4.0);
        for metric in metrics.iter().filter(|m| m.group == group) {
            if metric_row(ui, metric, app.selected.as_deref() == Some(metric.id), BLUE) {
                app.selected = Some(metric.id.to_string());
            }
        }
    }
    ui.add_space(12.0);
    ui.label(
        RichText::new("US TREASURY YIELD CURVE")
            .monospace()
            .size(12.0)
            .color(MUTED),
    );
    Frame::new()
        .fill(SURFACE)
        .stroke(Stroke::new(1.0, BORDER))
        .inner_margin(Margin::symmetric(8, 8))
        .show(ui, |ui| {
            let (rect, _) =
                ui.allocate_exact_size(Vec2::new(ui.available_width(), 210.0), Sense::hover());
            let points: Vec<(String, f64)> = app
                .snap
                .dollar
                .curve
                .iter()
                .map(|p| (p.label.clone(), p.yield_pct))
                .collect();
            draw::curve(ui.painter(), rect, &points);
        });
    if app.snap.dollar.dxy_spark.len() >= 2 {
        ui.add_space(8.0);
        let broad = app
            .snap
            .dollar
            .dxy_broad
            .as_ref()
            .map(|obs| {
                format!(
                    "BROAD TRADE-WEIGHTED USD {:.1} · FRED, separate from ICE DXY",
                    obs.value
                )
            })
            .unwrap_or_else(|| "BROAD TRADE-WEIGHTED USD".into());
        ui.label(RichText::new(broad).monospace().size(12.0).color(MUTED));
        let (rect, _) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), 56.0), Sense::hover());
        draw::panel(ui.painter(), rect, Some(BLUE));
        draw::sparkline(
            ui.painter(),
            rect.shrink2(Vec2::new(10.0, 8.0)),
            &app.snap.dollar.dxy_spark,
            BLUE,
        );
    }
    ui.add_space(8.0);
    ui.label(
        RichText::new("Sources: U.S. Treasury, FRED, CBOE, TradingView, exchangerate-api, Gold API, Binance. Status bands are transparent rules of thumb, not signals.")
            .size(12.0)
            .color(FAINT),
    );
}

fn btc(app: &mut Desktop, ui: &mut Ui) {
    kicker(ui, "BTC TERMINAL", AMBER);
    let price = app
        .snap
        .btc
        .price
        .map(model::fmt_usd)
        .unwrap_or_else(|| "—".into());
    ui.horizontal(|ui| {
        ui.label(RichText::new(price).size(36.0).color(TEXT));
        if let Some(change) = app.snap.btc.change_24h_pct {
            let color = if change >= 0.0 { GREEN } else { RED };
            ui.label(
                RichText::new(model::fmt_signed_pct(change))
                    .size(20.0)
                    .color(color),
            );
        }
        ui.label(
            RichText::new(&app.snap.btc.price_source)
                .monospace()
                .size(12.0)
                .color(FAINT),
        );
    });
    if let (Some(high), Some(low)) = (app.snap.btc.high_24h, app.snap.btc.low_24h) {
        ui.label(
            RichText::new(format!(
                "24h range {} – {}",
                model::fmt_usd(low),
                model::fmt_usd(high)
            ))
            .monospace()
            .size(12.0)
            .color(MUTED),
        );
    }
    ui.add_space(8.0);
    let fit = fit_power_law(&app.snap.btc.daily);
    let now = now_unix();
    let z = fit
        .as_ref()
        .and_then(|fit| app.snap.btc.price.and_then(|px| power_z(fit, now, px)));
    let fair = fit.as_ref().and_then(|fit| power_value(fit, now, 0.0));
    let sma = trailing_mean(&app.snap.btc.weekly, 200);
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = Vec2::new(8.0, 6.0);
        chip(
            ui,
            "POWER LAW",
            z.map(|v| format!("{v:+.2}σ")).unwrap_or_else(|| "—".into()),
            z.map(z_status),
        );
        chip(
            ui,
            "FAIR",
            fair.map(model::fmt_usd).unwrap_or_else(|| "—".into()),
            None,
        );
        chip(
            ui,
            "200W SMA",
            sma.map(|v| {
                let px = app.snap.btc.price.unwrap_or(v);
                format!("{} ({:+.0}%)", model::fmt_usd(v), (px / v - 1.0) * 100.0)
            })
            .unwrap_or_else(|| "—".into()),
            None,
        );
        chip(
            ui,
            "F&G",
            fng_text(app),
            app.snap.btc.fng.map(model::fng_status),
        );
        chip(
            ui,
            "FUNDING",
            funding_line(app.snap.btc.funding),
            app.snap.btc.funding.map(funding_status),
        );
        chip(
            ui,
            "HALVING",
            format!("{}d", (now - LAST_HALVING_UNIX) / 86_400),
            None,
        );
    });
    ui.add_space(8.0);
    Frame::new()
        .fill(SURFACE)
        .stroke(Stroke::new(1.0, BORDER))
        .inner_margin(Margin::symmetric(8, 8))
        .show(ui, |ui| {
            ui.label(
                RichText::new("LOG PRICE AND POWER-LAW FAIR VALUE")
                    .monospace()
                    .size(11.0)
                    .color(FAINT),
            );
            let (rect, _) =
                ui.allocate_exact_size(Vec2::new(ui.available_width(), 280.0), Sense::hover());
            draw::power_law(
                ui.painter(),
                rect,
                &power_samples(&app.snap.btc.daily, fit.as_ref()),
            );
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("amber price")
                        .monospace()
                        .size(11.0)
                        .color(AMBER),
                );
                ui.label(
                    RichText::new("gray fair value")
                        .monospace()
                        .size(11.0)
                        .color(FAINT),
                );
                if let Some(fit) = &fit {
                    ui.label(
                        RichText::new(format!("{} observations since Jul 2010", fit.n))
                            .monospace()
                            .size(11.0)
                            .color(FAINT),
                    );
                }
            });
        });
    ui.add_space(8.0);
    ui.columns(2, |cols| {
        cols[0].label(
            RichText::new("FEAR AND GREED")
                .monospace()
                .size(11.0)
                .color(FAINT),
        );
        let (rect, _) =
            cols[0].allocate_exact_size(Vec2::new(cols[0].available_width(), 72.0), Sense::hover());
        draw::panel(cols[0].painter(), rect, Some(AMBER));
        draw::sparkline(
            cols[0].painter(),
            rect.shrink2(Vec2::new(10.0, 10.0)),
            &app.snap.btc.fng_hist,
            AMBER,
        );
        cols[1].label(
            RichText::new("BINANCE PERP FUNDING")
                .monospace()
                .size(11.0)
                .color(FAINT),
        );
        let (rect, _) =
            cols[1].allocate_exact_size(Vec2::new(cols[1].available_width(), 72.0), Sense::hover());
        draw::panel(cols[1].painter(), rect, Some(CYAN));
        let bps: Vec<f64> = app
            .snap
            .btc
            .funding_hist
            .iter()
            .map(|v| v * 10_000.0)
            .collect();
        draw::sparkline(
            cols[1].painter(),
            rect.shrink2(Vec2::new(10.0, 10.0)),
            &bps,
            CYAN,
        );
    });
    ui.add_space(8.0);
    ui.label(
        RichText::new("History: Blockchain.com sampled daily prices for the fit, Binance weekly closes for the 200-week average, Binance 8h funding, alternative.me fear and greed. The fit is descriptive, not a forecast.")
            .size(12.0)
            .color(FAINT),
    );
}

fn ai(app: &mut Desktop, ui: &mut Ui) {
    kicker(ui, "AI INFRASTRUCTURE", PURPLE);
    ui.label(
        RichText::new("Compute, labs, and agents")
            .size(24.0)
            .color(TEXT),
    );
    ui.label(
        RichText::new(format!(
            "Snapshot {}",
            model::pretty_date(&app.catalog.ai_as_of)
        ))
        .monospace()
        .size(12.0)
        .color(FAINT),
    );
    ui.add_space(6.0);
    callout(ui, PURPLE, "Snapshot verdict", &app.catalog.ai_verdict);
    filter_box(ui, &mut app.filter);
    ui.add_space(8.0);
    section(ui, "PUBLIC AND PRIVATE NAMES");
    for player in app.catalog.ai_players.clone() {
        if !matches_filter(
            &app.filter,
            &format!(
                "{} {} {}",
                player.name,
                player.category,
                player.ticker.as_deref().unwrap_or("")
            ),
        ) {
            continue;
        }
        let id = format!("ai:{}", player.id);
        if player_row(
            ui,
            &player,
            &app.snap.quotes,
            app.selected.as_deref() == Some(&id),
            PURPLE,
        ) {
            app.selected = Some(id);
        }
    }
    ui.add_space(8.0);
    section(ui, "MARKET STRUCTURE");
    for note in app.catalog.ai_metrics.clone() {
        if !matches_filter(&app.filter, &format!("{} {}", note.label, note.value)) {
            continue;
        }
        let id = format!("aim:{}", note.id);
        if note_row(ui, &note, app.selected.as_deref() == Some(&id), PURPLE) {
            app.selected = Some(id);
        }
    }
    ui.add_space(8.0);
    section(ui, "WHAT TO WATCH");
    for note in app.catalog.ai_signals.clone() {
        if !matches_filter(&app.filter, &note.label) {
            continue;
        }
        let id = format!("ais:{}", note.id);
        if note_row(ui, &note, app.selected.as_deref() == Some(&id), PURPLE) {
            app.selected = Some(id);
        }
    }
}

fn nuclear(app: &mut Desktop, ui: &mut Ui) {
    kicker(ui, "NUCLEAR ENERGY", CYAN);
    ui.label(
        RichText::new("Fleet, fuel, and hyperscaler demand")
            .size(24.0)
            .color(TEXT),
    );
    ui.label(
        RichText::new(format!(
            "Evidence {}",
            model::pretty_date(&app.catalog.nuclear_as_of)
        ))
        .monospace()
        .size(12.0)
        .color(FAINT),
    );
    ui.add_space(6.0);
    callout(ui, CYAN, "Snapshot verdict", &app.catalog.nuclear_verdict);
    ui.add_space(6.0);
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = Vec2::new(8.0, 6.0);
        for symbol in URANIUM_ETFS {
            let text = quote(&app.snap.quotes, symbol)
                .map(|q| {
                    format!(
                        "{symbol} {} {}",
                        model::fmt_usd(q.price),
                        model::fmt_signed_pct(q.change_pct)
                    )
                })
                .unwrap_or_else(|| format!("{symbol} —"));
            chip(ui, "ETF", text, None);
        }
    });
    filter_box(ui, &mut app.filter);
    ui.add_space(8.0);
    section(ui, "COMPANIES AND PROJECTS");
    for player in app.catalog.nuclear_players.clone() {
        if !matches_filter(&app.filter, &format!("{} {}", player.name, player.category)) {
            continue;
        }
        let id = format!("nuc:{}", player.id);
        if player_row(
            ui,
            &player,
            &app.snap.quotes,
            app.selected.as_deref() == Some(&id),
            CYAN,
        ) {
            app.selected = Some(id);
        }
    }
    ui.add_space(8.0);
    section(ui, "OPERATING FACTS");
    for note in app.catalog.nuclear_metrics.clone() {
        if !matches_filter(&app.filter, &format!("{} {}", note.label, note.value)) {
            continue;
        }
        let id = format!("nucm:{}", note.id);
        if note_row(ui, &note, app.selected.as_deref() == Some(&id), CYAN) {
            app.selected = Some(id);
        }
    }
    ui.add_space(8.0);
    section(ui, "WHAT TO WATCH");
    for note in app.catalog.nuclear_signals.clone() {
        if !matches_filter(&app.filter, &note.label) {
            continue;
        }
        let id = format!("nucs:{}", note.id);
        if note_row(ui, &note, app.selected.as_deref() == Some(&id), CYAN) {
            app.selected = Some(id);
        }
    }
    ui.add_space(8.0);
    ui.label(
        RichText::new("Equity prices are delayed TradingView quotes. Uranium spot, reactor counts, and HALEU status stay on the dated snapshot.")
            .size(12.0)
            .color(FAINT),
    );
}

pub(crate) fn detail(app: &Desktop, ui: &mut Ui) {
    ScrollArea::vertical()
        .id_salt("detail")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let Some(id) = app.selected.clone() else {
                return;
            };
            Frame::new()
                .fill(SURFACE)
                .stroke(Stroke::new(1.0, BORDER))
                .inner_margin(Margin::symmetric(12, 12))
                .show(ui, |ui| {
                    if let Some(metric) = dollar_metrics(&app.snap).into_iter().find(|m| m.id == id)
                    {
                        metric_detail(ui, &metric);
                        return;
                    }
                    if let Some(rest) = id.strip_prefix("ai:") {
                        if let Some(player) = app.catalog.ai_players.iter().find(|p| p.id == rest) {
                            player_detail(ui, player, &app.snap.quotes);
                        }
                    } else if let Some(rest) = id.strip_prefix("aim:") {
                        if let Some(note) = app.catalog.ai_metrics.iter().find(|p| p.id == rest) {
                            note_detail(ui, note);
                        }
                    } else if let Some(rest) = id.strip_prefix("ais:") {
                        if let Some(note) = app.catalog.ai_signals.iter().find(|p| p.id == rest) {
                            note_detail(ui, note);
                        }
                    } else if let Some(rest) = id.strip_prefix("nuc:") {
                        if let Some(player) =
                            app.catalog.nuclear_players.iter().find(|p| p.id == rest)
                        {
                            player_detail(ui, player, &app.snap.quotes);
                        }
                    } else if let Some(rest) = id.strip_prefix("nucm:") {
                        if let Some(note) =
                            app.catalog.nuclear_metrics.iter().find(|p| p.id == rest)
                        {
                            note_detail(ui, note);
                        }
                    } else if let Some(rest) = id.strip_prefix("nucs:") {
                        if let Some(note) =
                            app.catalog.nuclear_signals.iter().find(|p| p.id == rest)
                        {
                            note_detail(ui, note);
                        }
                    }
                });
        });
}

fn metric_detail(ui: &mut Ui, metric: &Metric) {
    ui.label(
        RichText::new(metric.status.label())
            .monospace()
            .size(11.0)
            .color(draw::status_color(metric.status)),
    );
    ui.label(RichText::new(metric.label).size(18.0).color(TEXT));
    ui.label(
        RichText::new(&metric.value)
            .monospace()
            .size(28.0)
            .color(TEXT),
    );
    ui.add_space(6.0);
    ui.label(RichText::new(&metric.context).size(13.0).color(MUTED));
    ui.add_space(8.0);
    ui.label(RichText::new("RULE").monospace().size(11.0).color(FAINT));
    ui.label(RichText::new(metric.rule).size(13.0).color(TEXT));
}

fn player_detail(ui: &mut Ui, player: &PlayerCard, quotes: &[model::Quote]) {
    ui.label(
        RichText::new(player.status.label())
            .monospace()
            .size(11.0)
            .color(draw::status_color(player.status)),
    );
    ui.label(RichText::new(&player.name).size(18.0).color(TEXT));
    ui.label(
        RichText::new(live_scale(player, quotes))
            .monospace()
            .size(22.0)
            .color(TEXT),
    );
    if let Some(ticker) = &player.ticker {
        if let Some(q) = quote(quotes, ticker) {
            ui.label(
                RichText::new(format!(
                    "{ticker} {} {}",
                    model::fmt_usd(q.price),
                    model::fmt_signed_pct(q.change_pct)
                ))
                .monospace()
                .size(13.0)
                .color(MUTED),
            );
        }
    }
    ui.add_space(6.0);
    ui.label(RichText::new(&player.category).size(13.0).color(MUTED));
    ui.add_space(8.0);
    ui.label(RichText::new(&player.role).size(13.0).color(TEXT));
    ui.add_space(6.0);
    ui.label(RichText::new(&player.exposure).size(13.0).color(TEXT));
    ui.add_space(8.0);
    ui.label(
        RichText::new(format!(
            "{} · {}",
            player.source,
            model::pretty_date(&player.as_of)
        ))
        .monospace()
        .size(11.0)
        .color(FAINT),
    );
}

fn note_detail(ui: &mut Ui, note: &NoteCard) {
    ui.label(
        RichText::new(note.status.label())
            .monospace()
            .size(11.0)
            .color(draw::status_color(note.status)),
    );
    ui.label(
        RichText::new(note.group.as_str())
            .monospace()
            .size(11.0)
            .color(FAINT),
    );
    ui.label(RichText::new(&note.label).size(18.0).color(TEXT));
    if !note.value.is_empty() {
        ui.label(
            RichText::new(&note.value)
                .monospace()
                .size(22.0)
                .color(TEXT),
        );
    }
    if !note.context.is_empty() {
        ui.label(RichText::new(&note.context).size(13.0).color(MUTED));
    }
    ui.add_space(8.0);
    ui.label(RichText::new(&note.detail).size(13.0).color(TEXT));
    if !note.watch.is_empty() {
        ui.add_space(8.0);
        ui.label(RichText::new("WATCH").monospace().size(11.0).color(FAINT));
        ui.label(RichText::new(&note.watch).size(13.0).color(TEXT));
    }
    ui.add_space(8.0);
    ui.label(
        RichText::new(format!(
            "{} · {}",
            note.source,
            model::pretty_date(&note.as_of)
        ))
        .monospace()
        .size(11.0)
        .color(FAINT),
    );
}

fn metric_row(ui: &mut Ui, metric: &Metric, selected: bool, accent: Color32) -> bool {
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 42.0), Sense::click());
    let painter = ui.painter();
    painter.rect_filled(
        rect,
        CornerRadius::ZERO,
        if selected || response.hovered() {
            HOVER
        } else {
            SURFACE
        },
    );
    painter.rect_stroke(
        rect,
        CornerRadius::ZERO,
        Stroke::new(1.0, BORDER),
        StrokeKind::Inside,
    );
    if selected {
        painter.rect_filled(
            Rect::from_min_size(rect.left_top(), Vec2::new(3.0, rect.height())),
            CornerRadius::ZERO,
            accent,
        );
    }
    draw::dot(
        painter,
        rect.left_center() + Vec2::new(16.0, 0.0),
        draw::status_color(metric.status),
    );
    draw::label(
        painter,
        rect.left_center() + Vec2::new(28.0, 0.0),
        Align2::LEFT_CENTER,
        metric.label,
        14.0,
        TEXT,
        false,
    );
    if rect.width() > 820.0 {
        draw::label(
            painter,
            rect.left_center() + Vec2::new(rect.width() * 0.42, 0.0),
            Align2::LEFT_CENTER,
            &metric.value,
            15.0,
            TEXT,
            true,
        );
        draw::label(
            painter,
            rect.right_center() + Vec2::new(-12.0, 0.0),
            Align2::RIGHT_CENTER,
            &draw::truncate(&metric.context, 32),
            11.0,
            MUTED,
            true,
        );
    } else {
        draw::label(
            painter,
            rect.right_center() + Vec2::new(-12.0, 0.0),
            Align2::RIGHT_CENTER,
            &metric.value,
            15.0,
            TEXT,
            true,
        );
    }
    ui.add_space(4.0);
    response.clicked()
}

fn player_row(
    ui: &mut Ui,
    player: &PlayerCard,
    quotes: &[model::Quote],
    selected: bool,
    accent: Color32,
) -> bool {
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 48.0), Sense::click());
    let painter = ui.painter();
    painter.rect_filled(
        rect,
        CornerRadius::ZERO,
        if selected || response.hovered() {
            HOVER
        } else {
            SURFACE
        },
    );
    painter.rect_stroke(
        rect,
        CornerRadius::ZERO,
        Stroke::new(1.0, BORDER),
        StrokeKind::Inside,
    );
    if selected {
        painter.rect_filled(
            Rect::from_min_size(rect.left_top(), Vec2::new(3.0, rect.height())),
            CornerRadius::ZERO,
            accent,
        );
    }
    draw::dot(
        painter,
        rect.left_center() + Vec2::new(16.0, 0.0),
        draw::status_color(player.status),
    );
    let ticker = player.ticker.as_deref().unwrap_or(player.kind.as_str());
    draw::label(
        painter,
        rect.left_center() + Vec2::new(28.0, -8.0),
        Align2::LEFT_CENTER,
        &player.name,
        14.0,
        TEXT,
        false,
    );
    draw::label(
        painter,
        rect.left_center() + Vec2::new(28.0, 10.0),
        Align2::LEFT_CENTER,
        &draw::truncate(&player.category, 42),
        11.0,
        MUTED,
        false,
    );
    draw::label(
        painter,
        rect.right_center() + Vec2::new(-12.0, -8.0),
        Align2::RIGHT_CENTER,
        &live_scale(player, quotes),
        14.0,
        TEXT,
        true,
    );
    let quote_text = player
        .ticker
        .as_deref()
        .and_then(|t| quote(quotes, t))
        .map(|q| {
            format!(
                "{ticker} {} {}",
                model::fmt_usd(q.price),
                model::fmt_signed_pct(q.change_pct)
            )
        })
        .unwrap_or_else(|| ticker.to_string());
    draw::label(
        painter,
        rect.right_center() + Vec2::new(-12.0, 10.0),
        Align2::RIGHT_CENTER,
        &quote_text,
        11.0,
        MUTED,
        true,
    );
    ui.add_space(4.0);
    response.clicked()
}

fn note_row(ui: &mut Ui, note: &NoteCard, selected: bool, accent: Color32) -> bool {
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 46.0), Sense::click());
    let painter = ui.painter();
    painter.rect_filled(
        rect,
        CornerRadius::ZERO,
        if selected || response.hovered() {
            HOVER
        } else {
            SURFACE
        },
    );
    painter.rect_stroke(
        rect,
        CornerRadius::ZERO,
        Stroke::new(1.0, BORDER),
        StrokeKind::Inside,
    );
    if selected {
        painter.rect_filled(
            Rect::from_min_size(rect.left_top(), Vec2::new(3.0, rect.height())),
            CornerRadius::ZERO,
            accent,
        );
    }
    draw::dot(
        painter,
        rect.left_center() + Vec2::new(16.0, 0.0),
        draw::status_color(note.status),
    );
    draw::label(
        painter,
        rect.left_center() + Vec2::new(28.0, -8.0),
        Align2::LEFT_CENTER,
        &draw::truncate(&note.label, 52),
        14.0,
        TEXT,
        false,
    );
    let sub = if note.context.is_empty() {
        note.group.as_str()
    } else {
        note.context.as_str()
    };
    draw::label(
        painter,
        rect.left_center() + Vec2::new(28.0, 10.0),
        Align2::LEFT_CENTER,
        &draw::truncate(sub, 48),
        11.0,
        MUTED,
        false,
    );
    if !note.value.is_empty() {
        draw::label(
            painter,
            rect.right_center() + Vec2::new(-12.0, 0.0),
            Align2::RIGHT_CENTER,
            &note.value,
            14.0,
            TEXT,
            true,
        );
    }
    ui.add_space(4.0);
    response.clicked()
}

fn kicker(ui: &mut Ui, text: &str, color: Color32) {
    ui.label(RichText::new(text).monospace().size(12.0).color(color));
}

fn section(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).monospace().size(11.0).color(FAINT));
    ui.add_space(4.0);
}

fn callout(ui: &mut Ui, accent: Color32, title: &str, body: &str) {
    let painted = Frame::new()
        .fill(SURFACE)
        .stroke(Stroke::new(1.0, BORDER))
        .inner_margin(Margin::symmetric(14, 12))
        .show(ui, |ui| {
            ui.label(RichText::new(title).monospace().size(11.0).color(accent));
            ui.label(RichText::new(body).size(14.0).color(TEXT));
        });
    let rect = painted.response.rect;
    ui.painter().rect_filled(
        Rect::from_min_size(rect.left_top(), Vec2::new(3.0, rect.height())),
        CornerRadius::ZERO,
        accent,
    );
}

fn legend(ui: &mut Ui) {
    ui.horizontal(|ui| {
        for status in [
            Status::Calm,
            Status::Neutral,
            Status::Elevated,
            Status::Stressed,
        ] {
            let (rect, _) = ui.allocate_exact_size(Vec2::new(92.0, 16.0), Sense::hover());
            draw::dot(
                ui.painter(),
                rect.left_center() + Vec2::new(4.0, 0.0),
                draw::status_color(status),
            );
            draw::label(
                ui.painter(),
                rect.left_center() + Vec2::new(14.0, 0.0),
                Align2::LEFT_CENTER,
                status.label(),
                12.0,
                MUTED,
                true,
            );
        }
    });
}

fn chip(ui: &mut Ui, kicker: &str, value: String, status: Option<Status>) {
    let color = status.map(draw::status_color).unwrap_or(TEXT);
    Frame::new()
        .fill(SURFACE)
        .stroke(Stroke::new(1.0, BORDER))
        .inner_margin(Margin::symmetric(10, 6))
        .show(ui, |ui| {
            ui.set_min_width(118.0);
            ui.vertical(|ui| {
                ui.label(RichText::new(kicker).monospace().size(10.0).color(FAINT));
                ui.label(RichText::new(value).monospace().size(13.0).color(color));
            });
        });
}

fn filter_box(ui: &mut Ui, filter: &mut String) {
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.label(RichText::new("FILTER").monospace().size(11.0).color(FAINT));
        ui.add(
            TextEdit::singleline(filter)
                .desired_width(220.0)
                .hint_text("name or topic"),
        );
    });
}

fn matches_filter(filter: &str, text: &str) -> bool {
    let filter = filter.trim();
    filter.is_empty() || text.to_lowercase().contains(&filter.to_lowercase())
}

fn live_scale(player: &PlayerCard, quotes: &[model::Quote]) -> String {
    if let Some(ticker) = &player.ticker {
        if let Some(cap) = quote(quotes, ticker).and_then(|q| q.market_cap) {
            return model::fmt_cap(cap);
        }
    }
    player.scale.clone()
}

fn funding_line(rate: Option<f64>) -> String {
    match rate {
        Some(rate) => format!("{:+.2} bps / 8h", rate * 10_000.0),
        None => "funding —".into(),
    }
}

fn funding_status(rate: f64) -> Status {
    let abs = rate.abs();
    if abs >= 0.0005 {
        Status::Stressed
    } else if abs >= 0.0001 {
        Status::Elevated
    } else {
        Status::Calm
    }
}

fn z_status(z: f64) -> Status {
    let abs = z.abs();
    if abs >= 2.0 {
        Status::Stressed
    } else if abs >= 1.0 {
        Status::Elevated
    } else {
        Status::Neutral
    }
}

fn fng_text(app: &Desktop) -> String {
    match app.snap.btc.fng {
        Some(v) if !app.snap.btc.fng_label.is_empty() => format!("{v} {}", app.snap.btc.fng_label),
        Some(v) => v.to_string(),
        None => "—".into(),
    }
}

fn curve_line(app: &Desktop) -> String {
    match (&app.snap.dollar.ust2, &app.snap.dollar.ust10) {
        (Some(a), Some(b)) => format!("2s10s {:+.0} bps", (b.value - a.value) * 100.0),
        _ => "2s10s —".into(),
    }
}

fn session_line() -> String {
    let now = chrono::Utc::now();
    let t = now.hour() * 60 + now.minute();
    let flag = |open| if open { "open" } else { "closed" };
    format!(
        "Tokyo {}   London {}   New York {}",
        flag(t < 6 * 60),
        flag((8 * 60..16 * 60 + 30).contains(&t)),
        flag((13 * 60 + 30..20 * 60).contains(&t))
    )
}

fn power_samples(points: &[model::Point], fit: Option<&PowerFit>) -> Vec<(f64, f64, f64)> {
    let Some(fit) = fit else { return Vec::new() };
    let usable: Vec<&model::Point> = points.iter().filter(|p| p.v > 0.0).collect();
    if usable.len() < 2 {
        return Vec::new();
    }
    let max = 220;
    let step = (usable.len() as f64 / max as f64).max(1.0);
    let mut out = Vec::new();
    let mut index = 0.0;
    while (index as usize) < usable.len() {
        let point = usable[index as usize];
        let fair = power_value(fit, point.t, 0.0).unwrap_or(0.0);
        out.push((point.t as f64, point.v, fair));
        index += step;
    }
    out
}

fn pos2(x: f32, y: f32) -> egui::Pos2 {
    egui::Pos2::new(x, y)
}
