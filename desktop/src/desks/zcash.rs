//! Zcash desk: price and ZEC/BTC, shielded supply, issuance, network and derivatives.

use super::bitcoin::funding_points;
use super::{funding_status, funding_text, price_panel, PriceSpec};
use crate::app::Desktop;
use crate::charts::{self, Line, TimeChart};
use crate::theme::{
    medium, mono, sans, status_color, BORDER, FAINT, GAP, GREEN, MUTED, ORANGE, RED, TEXT,
};
use crate::widgets::{self, caps_job, col, Stat, W};
use dollar_dashboard::model::{self, fmt_change_pct, now_unix, Delta};
use dollar_dashboard::zcash::{self, ZecBook};
use eframe::egui::{Align, Pos2, Rect, RichText, Sense, Ui, Vec2};

pub(crate) fn show(app: &mut Desktop, ui: &mut Ui) {
    let zec = app.snap.zec.clone();
    let price = app.snap.btc.zec_price;
    hero(app, ui, &zec);
    ui.add_space(GAP);
    price_panel(
        app,
        ui,
        PriceSpec {
            title: "Price history",
            symbol: "ZEC",
            points: &zec.daily,
            live: price,
            observed_unix: app.snap.price_observed_unix("ZEC"),
            color: ORANGE(),
            fair_value: false,
            source: "Binance",
        },
        290.0,
    );
    ui.add_space(GAP);
    super::log_log_panel(
        ui,
        super::LogLogSpec {
            symbol: "ZEC",
            points: &zec.history,
            live: price,
            observed_unix: app.snap.price_observed_unix("ZEC"),
            color: ORANGE(),
            genesis: model::ZEC_GENESIS_UNIX,
            cutoff: model::ZEC_POWER_LAW_CUTOFF,
            day_zero: "genesis block, 28 Oct 2016",
            source: "Coin Metrics",
        },
        320.0,
    );
    ui.add_space(GAP);
    widgets::panel(ui, "Key readings", "market · supply · derivatives", |ui| {
        widgets::stat_grid(ui, &stats(&zec, price), 140.0);
    });
    ui.add_space(GAP);
    widgets::quote_block(ui, ORANGE(), "Shielded supply", &zcash::readout(&zec));
    ui.add_space(GAP);
    cypherpunk(ui, &zec, price);
    ui.add_space(GAP);
    widgets::masonry(
        ui,
        eframe::egui::Id::new("zec-panels"),
        4,
        900.0,
        |ui, i| match i {
            0 => pools(ui, &zec),
            1 => issuance(ui, &zec),
            2 => derivatives(ui, &zec, price),
            _ => network(ui, &zec),
        },
    );
    widgets::footnote(
        ui,
        "Spot, ZEC/BTC, perp funding and open interest: Binance. Height, supply, value pools and upgrades: a public zcashd node (mainnet.zcashexplorer.app). Network activity: Blockchair. Market cap, rank and ATH: CoinGecko. CYPH close: Yahoo Finance. Cypherpunk holdings and shares: SEC filings and cypherpunk.com. Halving dates assume target spacing: 75 seconds until NU7 at block 3,543,405, then 25 seconds.",
    );
}

fn hero(app: &Desktop, ui: &mut Ui, zec: &ZecBook) {
    let now = now_unix();
    ui.label(caps_job("Zcash · ZEC/USDT", 10.5, ORANGE()));
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 12.0;
        ui.label(
            RichText::new(
                app.snap
                    .btc
                    .zec_price
                    .map(model::fmt_usd)
                    .unwrap_or_else(|| "—".into()),
            )
            .font(medium(30.0))
            .color(TEXT()),
        );
        if let Some(change) = app.snap.btc.zec_change_24h_pct {
            ui.label(
                RichText::new(format!("{} 24h", fmt_change_pct(change)))
                    .font(mono(14.0))
                    .color(if change >= 0.0 { GREEN() } else { RED() }),
            );
        }
        let mut facts = Vec::new();
        if let Some(ratio) = zec.btc_ratio {
            let change = zec
                .btc_ratio_change
                .map(|v| format!(" ({})", fmt_change_pct(v)))
                .unwrap_or_default();
            facts.push(format!("ZEC/BTC {ratio:.5}{change}"));
        }
        if let Some(rank) = zec.rank {
            facts.push(format!("rank #{rank}"));
        }
        if let Some(height) = zec.height {
            facts.push(format!("block {}", model::fmt_grouped(height as f64, 0)));
        }
        facts.push(app.snap.price_as_of("ZEC", now));
        ui.label(
            RichText::new(facts.join(" · "))
                .font(sans(11.5))
                .color(FAINT()),
        );
    });
}

fn stats(zec: &ZecBook, price: Option<f64>) -> Vec<Stat> {
    let dash = || "—".to_string();
    let now = now_unix();
    let change = |days: i64| zec.change_pct(days);
    let pct = |v: Option<f64>| v.map(fmt_change_pct).unwrap_or_else(dash);
    vec![
        Stat::new(
            "Market cap",
            zec.market_cap.map(model::fmt_cap).unwrap_or_else(dash),
        )
        .sub(
            zec.rank.map(|r| format!("rank #{r}")).unwrap_or_default(),
            FAINT(),
        ),
        Stat::new(
            "Volume 24h",
            zec.volume_24h.map(model::fmt_cap).unwrap_or_else(dash),
        )
        .sub("CoinGecko, all venues", FAINT()),
        Stat::new("7 days", pct(change(7))).tone(crate::theme::move_color(change(7))),
        Stat::new("30 days", pct(change(30))).tone(crate::theme::move_color(change(30))),
        Stat::new("1 year", pct(change(365))).tone(crate::theme::move_color(change(365))),
        Stat::new(
            "All-time high",
            zec.ath.map(model::fmt_usd).unwrap_or_else(dash),
        )
        .sub(
            match (price, zec.ath) {
                (Some(px), Some(ath)) if ath > 0.0 => format!(
                    "{} from ATH · {}",
                    fmt_change_pct((px / ath - 1.0) * 100.0),
                    zec.ath_date.get(0..4).unwrap_or("")
                ),
                _ => String::new(),
            },
            FAINT(),
        ),
        Stat::new(
            "Shielded",
            zec.shielded_share()
                .map(|v| format!("{v:.1}%"))
                .unwrap_or_else(dash),
        )
        .tone(ORANGE())
        .sub(
            zec.shielded()
                .map(|v| format!("{:.2}M ZEC", v / 1e6))
                .unwrap_or_default(),
            FAINT(),
        ),
        Stat::new(
            "Inflation",
            zec.inflation_pct()
                .map(|v| format!("{v:.2}% / yr"))
                .unwrap_or_else(dash),
        )
        .sub("protocol issuance", FAINT()),
        Stat::new(
            "Next halving",
            zec.height
                .map(|h| format!("{} days", (zcash::halving_eta_unix(h, now) - now) / 86_400))
                .unwrap_or_else(dash),
        )
        .sub(
            zec.height
                .and_then(|h| chrono::DateTime::from_timestamp(zcash::halving_eta_unix(h, now), 0))
                .map(|d| format!("≈ {}", d.format("%b %Y")))
                .unwrap_or_default(),
            FAINT(),
        ),
        Stat::new("Funding · Binance", funding_text(zec.funding))
            .tone(
                zec.funding
                    .map(|v| status_color(funding_status(v)))
                    .unwrap_or(TEXT()),
            )
            .sub(
                zec.funding
                    .map(|v| format!("{} annualized", fmt_change_pct(v * 3.0 * 365.0 * 100.0)))
                    .unwrap_or_default(),
                FAINT(),
            ),
        Stat::new("Open interest", open_interest(zec, price)).sub("Binance USDT perp", FAINT()),
    ]
}

fn cypherpunk(ui: &mut Ui, zec: &ZecBook, price: Option<f64>) {
    let close = zec.cyph_daily.last().copied();
    let meta = format!("holdings to {}", zcash::CYPH_ZEC_AS_OF);
    widgets::panel(ui, "Cypherpunk (CYPH) equity ÷ ZEC held", &meta, |ui| {
        let ratio = |shares: f64| {
            close
                .zip(price)
                .and_then(|(close, px)| zcash::cyph_ratio(shares, close.v, px))
                .map(|v| format!("{v:.2}×"))
                .unwrap_or_else(|| "—".into())
        };
        let millions = |shares: f64| format!("{:.1}M shares", shares / 1e6);
        let supply_share = zec
            .supply
            .filter(|s| *s > 0.0)
            .map(|s| format!("{:.2}% of supply", zcash::CYPH_ZEC / s * 100.0))
            .unwrap_or_default();
        let stats = [
            Stat::new(
                "Issued + pre-funded",
                ratio(zcash::CYPH_ISSUED_AND_PREFUNDED),
            )
            .tone(ORANGE())
            .sub(millions(zcash::CYPH_ISSUED_AND_PREFUNDED), FAINT()),
            Stat::new("Common only", ratio(zcash::CYPH_COMMON))
                .sub(millions(zcash::CYPH_COMMON), FAINT()),
            Stat::new("Fully diluted", ratio(zcash::CYPH_FULLY_DILUTED))
                .sub(millions(zcash::CYPH_FULLY_DILUTED), FAINT()),
            Stat::new("ZEC held", model::fmt_grouped(zcash::CYPH_ZEC, 0))
                .sub(supply_share, FAINT()),
            Stat::new(
                "Holdings value",
                price
                    .map(|px| model::fmt_cap(zcash::CYPH_ZEC * px))
                    .unwrap_or_else(|| "—".into()),
            )
            .sub(
                price
                    .map(|px| {
                        format!(
                            "{:.1}× cost of ${:.0}",
                            px / zcash::CYPH_AVG_COST,
                            zcash::CYPH_AVG_COST
                        )
                    })
                    .unwrap_or_default(),
                FAINT(),
            ),
            Stat::new(
                "CYPH close",
                close
                    .map(|c| format!("${:.2}", c.v))
                    .unwrap_or_else(|| "—".into()),
            )
            .sub(
                close
                    .and_then(|c| chrono::DateTime::from_timestamp(c.t, 0))
                    .map(|d| d.format("%-d %b %Y").to_string())
                    .unwrap_or_default(),
                FAINT(),
            ),
        ];
        widgets::stat_grid(ui, &stats, 130.0);
        if let Some(text) = close
            .zip(price)
            .and_then(|(close, px)| zcash::cyph_readout(close.v, px))
        {
            ui.add_space(8.0);
            ui.label(RichText::new(text).font(sans(12.5)).color(TEXT()));
        }
        let paid_up = zcash::cyph_ratio_history(
            &zec.cyph_daily,
            &zec.daily,
            zcash::CYPH_ISSUED_AND_PREFUNDED,
        );
        let common = zcash::cyph_ratio_history(&zec.cyph_daily, &zec.daily, zcash::CYPH_COMMON);
        if let (Some(first), Some(last)) = (paid_up.first(), paid_up.last()) {
            let parity = [
                model::Point { t: first.t, v: 1.0 },
                model::Point { t: last.t, v: 1.0 },
            ];
            let axis = |v: f64| format!("{v:.1}×");
            let value = |v: f64| format!("{v:.2}×");
            ui.add_space(8.0);
            charts::time_chart(
                ui,
                TimeChart::new(130.0, &axis, &value)
                    .line(Line::new("Issued + pre-funded", &paid_up, ORANGE()))
                    .line(Line::new("Common only", &common, MUTED()))
                    .line(Line::new("Parity", &parity, BORDER()).width(1.0)),
            );
        }
        widgets::footnote(
            ui,
            "Today's share and ZEC counts applied to each close since the mining deal on 18 Aug 2026. Counts ZEC only, not cash, the $5M ZODL stake or mining assets. Last purchase listed by the company: $1M on 12 Aug 2026; mined ZEC since. Inputs are set by hand from filings.",
        );
    });
}

fn open_interest(zec: &ZecBook, price: Option<f64>) -> String {
    match (zec.open_interest, price) {
        (Some(oi), Some(px)) => model::fmt_cap(oi * px),
        (Some(oi), None) => format!("{} ZEC", model::fmt_compact(oi)),
        _ => "—".into(),
    }
}

fn pools(ui: &mut Ui, zec: &ZecBook) {
    widgets::panel(ui, "Value pools", "zcashd node · chain value", |ui| {
        let Some(supply) = zec.supply.filter(|s| *s > 0.0) else {
            widgets::empty(ui, 80.0, "The node feed has not answered yet.");
            return;
        };
        let mut pools = zec.pools.clone();
        // Shielded pools first, largest first, then the public balances.
        pools.sort_by(|a, b| {
            zcash::is_shielded(&b.id)
                .cmp(&zcash::is_shielded(&a.id))
                .then(b.zec.total_cmp(&a.zec))
        });
        let palette = [
            ORANGE(),
            ORANGE().gamma_multiply(0.7),
            ORANGE().gamma_multiply(0.5),
            ORANGE().gamma_multiply(0.35),
        ];
        let color_of = |i: usize, id: &str| {
            if zcash::is_shielded(id) {
                palette[i.min(palette.len() - 1)]
            } else if id == "lockbox" {
                MUTED()
            } else {
                BORDER()
            }
        };
        let (bar, _) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), 14.0), Sense::hover());
        let mut x = bar.left();
        for (i, pool) in pools.iter().enumerate() {
            let w = bar.width() * (pool.zec / supply).clamp(0.0, 1.0) as f32;
            ui.painter().rect_filled(
                Rect::from_min_size(Pos2::new(x, bar.top()), Vec2::new(w.max(0.0), bar.height())),
                eframe::egui::CornerRadius::ZERO,
                color_of(i, &pool.id),
            );
            x += w;
        }
        ui.add_space(8.0);
        let cols = [
            col("", W::Px(10.0), Align::Min),
            col("Pool", W::Flex(1.0), Align::Min),
            col("ZEC", W::Px(84.0), Align::Max),
            col("Share", W::Px(56.0), Align::Max),
        ];
        let (layout, _) = widgets::table_header(ui, &cols, None);
        for (i, pool) in pools.iter().enumerate() {
            let (rect, response) = widgets::table_row(ui, 25.0, false, false);
            let painter = ui.painter();
            painter.rect_filled(
                Rect::from_center_size(layout.cell(rect, 0).center(), Vec2::splat(8.0)),
                crate::theme::corner(2),
                color_of(i, &pool.id),
            );
            widgets::cell_text(
                painter,
                layout.cell(rect, 1),
                &zcash::pool_name(&pool.id),
                sans(12.5),
                TEXT(),
                Align::Min,
            );
            widgets::cell_text(
                painter,
                layout.cell(rect, 2),
                &model::fmt_compact(pool.zec),
                mono(12.0),
                TEXT(),
                Align::Max,
            );
            widgets::cell_text(
                painter,
                layout.cell(rect, 3),
                &format!("{:.1}%", pool.zec / supply * 100.0),
                mono(12.0),
                MUTED(),
                Align::Max,
            );
            response.on_hover_text(zcash::pool_note(&pool.id));
        }
        ui.add_space(6.0);
        ui.label(
            RichText::new(format!(
                "Total supply {} ZEC · {:.1}% of the 21M cap issued",
                model::fmt_grouped(supply, 0),
                supply / zcash::MAX_SUPPLY * 100.0
            ))
            .font(sans(11.5))
            .color(FAINT()),
        );
    });
}

fn issuance(ui: &mut Ui, zec: &ZecBook) {
    let meta = zec
        .height
        .map(|h| format!("{}-second blocks", zcash::block_seconds(h)))
        .unwrap_or_default();
    widgets::panel(ui, "Issuance and halving", &meta, |ui| {
        let Some(height) = zec.height else {
            widgets::empty(ui, 80.0, "The node feed has not answered yet.");
            return;
        };
        let now = now_unix();
        let next = zcash::next_halving(height);
        let eta = zcash::halving_eta_unix(height, now);
        let era = zcash::era_progress(height) as f32;
        let stats = [
            Stat::new("Block height", model::fmt_grouped(height as f64, 0)).sub(
                match (zec.upgrade.as_str(), zec.upgrade_height) {
                    ("", _) => String::new(),
                    (name, _) => format!("upgrade {name} active"),
                },
                FAINT(),
            ),
            Stat::new(
                "Block subsidy",
                format!("{} ZEC", zcash::block_subsidy(height)),
            )
            .sub("halves about every 4 years", FAINT()),
            Stat::new("Halving block", model::fmt_grouped(next as f64, 0)).sub(
                chrono::DateTime::from_timestamp(eta, 0)
                    .map(|d| format!("≈ {}", d.format("%-d %b %Y")))
                    .unwrap_or_default(),
                ORANGE(),
            ),
            Stat::new(
                "Annual issuance",
                format!("{} ZEC", model::fmt_compact(zcash::annual_issuance(height))),
            )
            .sub(
                zec.inflation_pct()
                    .map(|v| format!("{v:.2}% of supply"))
                    .unwrap_or_default(),
                FAINT(),
            ),
        ];
        widgets::stat_grid(ui, &stats, 130.0);
        ui.add_space(8.0);
        let (rect, _) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), 8.0), Sense::hover());
        ui.painter()
            .rect_filled(rect, crate::theme::corner(3), BORDER());
        ui.painter().rect_filled(
            Rect::from_min_size(
                rect.min,
                Vec2::new(rect.width() * era.clamp(0.0, 1.0), rect.height()),
            ),
            crate::theme::corner(3),
            ORANGE().gamma_multiply(0.85),
        );
        ui.label(
            RichText::new(format!(
                "{:.0}% of the current halving era elapsed · {} blocks to go",
                era * 100.0,
                model::fmt_grouped((next - height) as f64, 0)
            ))
            .font(sans(11.5))
            .color(FAINT()),
        );
    });
}

fn network(ui: &mut Ui, zec: &ZecBook) {
    widgets::panel(ui, "Network", "Blockchair · last 24 hours", |ui| {
        let count = |v: Option<f64>| {
            v.map(|v| model::fmt_grouped(v, 0))
                .unwrap_or_else(|| "—".into())
        };
        let si = |v: f64| match v.abs() {
            v2 if v2 >= 1e12 => format!("{:.2} T", v / 1e12),
            v2 if v2 >= 1e9 => format!("{:.2} G", v / 1e9),
            v2 if v2 >= 1e6 => format!("{:.2} M", v / 1e6),
            v2 if v2 >= 1e3 => format!("{:.2} k", v / 1e3),
            _ => format!("{v:.0} "),
        };
        let rows = [
            ("Transactions", count(zec.tx_24h), String::new()),
            (
                "Blocks",
                count(zec.blocks_24h),
                zec.height
                    .map(|h| {
                        format!(
                            "{} expected",
                            model::fmt_grouped(zcash::blocks_per_day(h), 0)
                        )
                    })
                    .unwrap_or_default(),
            ),
            (
                "Hashrate",
                zec.hashrate
                    .map(|v| format!("{}Sol/s", si(v)))
                    .unwrap_or_else(|| "—".into()),
                "Equihash".into(),
            ),
            (
                "Difficulty",
                zec.difficulty.map(si).unwrap_or_else(|| "—".into()),
                String::new(),
            ),
            (
                "Median fee",
                zec.median_fee_usd
                    .map(|v| format!("${v:.3}"))
                    .unwrap_or_else(|| "—".into()),
                String::new(),
            ),
            ("Mempool", count(zec.mempool_tx), "waiting".into()),
            ("Nodes", count(zec.nodes), "reachable".into()),
        ];
        let cols = [
            col("Measure", W::Flex(1.0), Align::Min),
            col("Value", W::Px(110.0), Align::Max),
            col("Note", W::Px(90.0), Align::Max),
        ];
        let (layout, _) = widgets::table_header(ui, &cols, None);
        for (label, value, note) in rows {
            let (rect, _) = widgets::table_row(ui, 25.0, false, false);
            let painter = ui.painter();
            widgets::cell_text(
                painter,
                layout.cell(rect, 0),
                label,
                sans(12.5),
                TEXT(),
                Align::Min,
            );
            widgets::cell_text(
                painter,
                layout.cell(rect, 1),
                &value,
                mono(12.0),
                TEXT(),
                Align::Max,
            );
            widgets::cell_text(
                painter,
                layout.cell(rect, 2),
                &note,
                sans(11.0),
                FAINT(),
                Align::Max,
            );
        }
    });
}

fn derivatives(ui: &mut Ui, zec: &ZecBook, price: Option<f64>) {
    let meta = format!(
        "funding {} · OI {}",
        funding_text(zec.funding),
        open_interest(zec, price)
    );
    widgets::panel(ui, "Perpetuals and ZEC/BTC", &meta, |ui| {
        let points = funding_points(&zec.funding_hist, now_unix());
        let axis = |v: f64| charts::axis_number(v);
        let value = |v: f64| model::with_minus(format!("{v:+.2} bp / 8h"));
        charts::bars(ui, 100.0, &points, &axis, &value);
        widgets::footnote(
            ui,
            "Binance ZECUSDT funding per 8-hour interval, basis points.",
        );
        ui.add_space(8.0);
        let axis = |v: f64| charts::axis_number(v);
        let value = |v: f64| format!("{v:.5} BTC");
        let ratio_change = zec.btc_ratio_daily.len() > 1;
        charts::time_chart(
            ui,
            TimeChart::new(130.0, &axis, &value)
                .line(Line::new("ZEC/BTC", &zec.btc_ratio_daily, ORANGE()).fill()),
        );
        if ratio_change {
            let (first, last) = (
                zec.btc_ratio_daily[0],
                zec.btc_ratio_daily[zec.btc_ratio_daily.len() - 1],
            );
            let change = (first.v > 0.0).then(|| (last.v / first.v - 1.0) * 100.0);
            let (text, color) = widgets::delta(change.map(Delta::Pct));
            let since = chrono::DateTime::from_timestamp(first.t, 0)
                .map(|d| d.format("%-d %b %Y").to_string())
                .unwrap_or_default();
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!("ZEC/BTC since {since}"))
                        .font(sans(11.0))
                        .color(FAINT()),
                );
                ui.label(RichText::new(text).font(mono(11.5)).color(color));
            });
        }
    });
}
