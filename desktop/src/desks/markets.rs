//! Markets desk: every quoted instrument in one sortable table, with a watchlist.

use super::name_of;
use crate::app::{Desk, Desktop};
use crate::theme::{mono, sans, AMBER, FAINT, GAP, GREEN, INPUT, MUTED, TEXT};
use crate::widgets::{self, col, W};
use dollar_dashboard::catalog::{AI_TICKERS, URANIUM_ETFS};
use dollar_dashboard::model::{self, fmt_age, now_unix, Delta, Quote};
use dollar_dashboard::workspace::{instruments, quotes_csv};
use eframe::egui::{Align, CursorIcon, Layout, Sense, TextEdit, Ui};

fn sector(symbol: &str) -> &'static str {
    if matches!(symbol, "BTC" | "ETH" | "ZEC") {
        "Crypto"
    } else if symbol == "STRC" {
        "Bitcoin treasury"
    } else if AI_TICKERS.contains(&symbol) {
        "AI infrastructure"
    } else if URANIUM_ETFS.contains(&symbol) {
        "Uranium ETF"
    } else {
        "Nuclear energy"
    }
}

fn desk_of(symbol: &str) -> Desk {
    match sector(symbol) {
        _ if symbol == "ZEC" => Desk::Zcash,
        _ if symbol == "SPCX" => Desk::SpaceX,
        "Crypto" | "Bitcoin treasury" => Desk::Btc,
        "AI infrastructure" => Desk::Ai,
        _ => Desk::Nuclear,
    }
}

fn rows(app: &Desktop) -> Vec<Quote> {
    let filter = app.filter.trim().to_lowercase();
    let mut rows: Vec<_> = instruments(&app.snap)
        .into_iter()
        .filter(|q| {
            (!app.watch_only || app.prefs.watchlist.contains(&q.symbol))
                && (filter.is_empty()
                    || format!(
                        "{} {} {}",
                        q.symbol,
                        name_of(app, &q.symbol),
                        sector(&q.symbol)
                    )
                    .to_lowercase()
                    .contains(&filter))
        })
        .collect();
    rows.sort_by(|a, b| {
        let order = match app.sort_column {
            4 => a.price.total_cmp(&b.price),
            5 => a.change_pct.total_cmp(&b.change_pct),
            6 => a
                .market_cap
                .unwrap_or(0.0)
                .total_cmp(&b.market_cap.unwrap_or(0.0)),
            3 => sector(&a.symbol).cmp(sector(&b.symbol)),
            2 => name_of(app, &a.symbol).cmp(&name_of(app, &b.symbol)),
            _ => a.symbol.cmp(&b.symbol),
        };
        if app.sort_desc {
            order.reverse()
        } else {
            order
        }
    });
    rows
}

pub(crate) fn show(app: &mut Desktop, ui: &mut Ui) {
    let universe = instruments(&app.snap).len();
    let updated = if app.snap.fetched_unix > 0 {
        format!("source refresh {}", fmt_age(app.snap.fetched_unix, now_unix()))
    } else {
        "no refresh yet".into()
    };
    widgets::desk_header(
        ui,
        "Markets",
        GREEN(),
        "Markets & watchlist",
        &format!("{universe} instruments · crypto changes over 24h, equities over the latest session (delayed) · {updated}"),
    );
    ui.horizontal(|ui| {
        widgets::segmented(
            ui,
            &mut app.watch_only,
            &[("All instruments", false), ("Watchlist", true)],
        );
        ui.add_space(4.0);
        ui.add(
            TextEdit::singleline(&mut app.filter)
                .desired_width(220.0)
                .hint_text("Filter symbol, name or sector")
                .background_color(INPUT()),
        );
        if !app.filter.is_empty() && widgets::link(ui, "Clear", FAINT()).clicked() {
            app.filter.clear();
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ui
                .button("Export CSV")
                .on_hover_text("Write the rows below to ~/Downloads")
                .clicked()
            {
                export(app);
            }
        });
    });
    ui.add_space(GAP);
    let rows = rows(app);
    let mut toggle_star: Option<String> = None;
    let mut open: Option<String> = None;
    let mut sort_click = None;
    widgets::panel(
        ui,
        if app.watch_only {
            "Watchlist"
        } else {
            "All instruments"
        },
        &format!("{} shown · click a header to sort", rows.len()),
        |ui| {
            let wide = ui.available_width() >= 720.0;
            let mut cols = vec![
                col("", W::Px(18.0), Align::Center),
                col("Symbol", W::Px(64.0), Align::Min),
                col("Name", W::Flex(1.0), Align::Min),
                col("Sector", W::Px(120.0), Align::Min),
                col("Last", W::Px(96.0), Align::Max),
                col("Day", W::Px(72.0), Align::Max),
                col("Market cap", W::Px(96.0), Align::Max),
            ];
            if !wide {
                cols.remove(3);
            }
            let sort = Some((
                if wide || app.sort_column < 3 {
                    app.sort_column.max(1)
                } else {
                    app.sort_column - 1
                },
                app.sort_desc,
            ));
            let (layout, clicked) = widgets::table_header(ui, &cols, sort);
            sort_click = clicked.map(|c| if wide || c < 3 { c } else { c + 1 });
            for q in &rows {
                let favorite = app.prefs.watchlist.contains(&q.symbol);
                let (rect, response) = widgets::table_row(ui, 28.0, false, true);
                let star_rect = layout.cell(rect, 0);
                let star =
                    ui.interact(star_rect, ui.id().with(("star", &q.symbol)), Sense::click());
                let painter = ui.painter();
                widgets::cell_text(
                    painter,
                    star_rect,
                    if favorite { "★" } else { "☆" },
                    sans(14.0),
                    if favorite {
                        AMBER()
                    } else if star.hovered() {
                        TEXT()
                    } else {
                        FAINT()
                    },
                    Align::Center,
                );
                let mut c = 1;
                widgets::cell_text(
                    painter,
                    layout.cell(rect, c),
                    &q.symbol,
                    mono(12.5),
                    TEXT(),
                    Align::Min,
                );
                c += 1;
                widgets::cell_text(
                    painter,
                    layout.cell(rect, c),
                    &name_of(app, &q.symbol),
                    sans(12.5),
                    MUTED(),
                    Align::Min,
                );
                if wide {
                    c += 1;
                    widgets::cell_text(
                        painter,
                        layout.cell(rect, c),
                        sector(&q.symbol),
                        sans(11.5),
                        FAINT(),
                        Align::Min,
                    );
                }
                c += 1;
                widgets::cell_text(
                    painter,
                    layout.cell(rect, c),
                    &model::fmt_usd(q.price),
                    mono(12.5),
                    TEXT(),
                    Align::Max,
                );
                c += 1;
                let (day, color) = widgets::delta(Some(Delta::Pct(q.change_pct)));
                widgets::cell_text(
                    painter,
                    layout.cell(rect, c),
                    &day,
                    mono(12.0),
                    color,
                    Align::Max,
                );
                c += 1;
                widgets::cell_text(
                    painter,
                    layout.cell(rect, c),
                    &q.market_cap
                        .map(model::fmt_cap)
                        .unwrap_or_else(|| "—".into()),
                    mono(12.0),
                    MUTED(),
                    Align::Max,
                );
                if star.hovered() {
                    ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                }
                if star
                    .on_hover_text(if favorite {
                        "Remove from watchlist"
                    } else {
                        "Add to watchlist"
                    })
                    .clicked()
                {
                    toggle_star = Some(q.symbol.clone());
                } else if response
                    .on_hover_text(format!(
                        "{} · {}\nOpen the {} desk",
                        app.snap.price_stamp(&q.symbol).map(|stamp| stamp.source.as_str()).unwrap_or("Cached price; observation time unavailable"),
                        app.snap.price_as_of(&q.symbol, now_unix()),
                        desk_of(&q.symbol).title(),
                    ))
                    .clicked()
                {
                    open = Some(q.symbol.clone());
                }
            }
            if rows.is_empty() {
                widgets::empty(
                    ui,
                    48.0,
                    if app.watch_only {
                        "No favorites match. Switch to All instruments and star a name."
                    } else {
                        "No quotes match. Clear the filter or refresh sources."
                    },
                );
            }
        },
    );
    if let Some(col) = sort_click {
        if app.sort_column == col {
            app.sort_desc = !app.sort_desc;
        } else {
            app.sort_column = col;
            app.sort_desc = col >= 4;
        }
    }
    if let Some(symbol) = toggle_star {
        if app.prefs.watchlist.contains(&symbol) {
            app.prefs.watchlist.retain(|s| s != &symbol);
        } else {
            app.prefs.watchlist.push(symbol);
        }
    }
    if let Some(symbol) = open {
        let desk = desk_of(&symbol);
        app.navigate(desk);
        if !matches!(desk, Desk::Btc | Desk::Zcash | Desk::SpaceX) {
            app.filter = symbol;
        }
    }
    widgets::footnote(
        ui,
        "Crypto: Binance spot. Equities and ETFs: TradingView delayed quotes with market caps where reported. STRC: Yahoo Finance daily close.",
    );
}

fn export(app: &mut Desktop) {
    let rows = rows(app);
    let base = std::env::var_os("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| ".".into())
        .join("Downloads");
    let path = base.join(format!(
        "market-monitor-{}.csv",
        chrono::Utc::now().format("%Y%m%d-%H%M%S")
    ));
    let result = std::fs::create_dir_all(&base)
        .and_then(|_| std::fs::write(&path, quotes_csv(&rows, app.snap.fetched_unix)));
    app.notice = match result {
        Ok(()) => format!("Exported {} quotes to {}", rows.len(), path.display()),
        Err(err) => format!("Export failed: {err}"),
    };
}
