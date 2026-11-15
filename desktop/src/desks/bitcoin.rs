//! Bitcoin desk: price and cycle position, valuation, flows and macro context.

use super::{
    funding_bp, funding_status, funding_text, price_panel, signed_int, z_status, PriceSpec,
};
use crate::app::Desktop;
use crate::charts::{self, Line, TimeChart};
use crate::theme::{
    medium, mono, sans, status_color, AMBER, BLUE, CYAN, FAINT, GAP, GREEN, MUTED, RAISED, RED,
    TEXT,
};
use crate::widgets::{self, caps_job, col, Stat, W};
use dollar_dashboard::model::{
    self, fit_power_law, fmt_age, fmt_change_pct, now_unix, power_value, power_z, trailing_mean,
    with_minus, Point, Terminal, LAST_HALVING_UNIX, SECONDS_PER_DAY,
};
use eframe::egui::{Align, Align2, Pos2, Rect, RichText, Sense, Stroke, Ui, Vec2};

pub(crate) fn show(app: &mut Desktop, ui: &mut Ui) {
    hero(app, ui);
    ui.add_space(GAP);
    key_stats(app, ui);
    ui.add_space(GAP);
    let daily = app.snap.btc.daily.clone();
    let source = if daily.len() > 3000 {
        "Coin Metrics"
    } else {
        "Blockchain.com sampled"
    };
    let chart_height = if ui.available_width() < 950.0 {
        220.0
    } else {
        310.0
    };
    price_panel(
        app,
        ui,
        PriceSpec {
            title: "Price history",
            symbol: "BTC",
            points: &daily,
            live: app.snap.btc.price,
            observed_unix: app.snap.price_observed_unix("BTC"),
            color: AMBER(),
            fair_value: true,
            source,
        },
        chart_height,
    );
    ui.add_space(GAP);
    super::log_log_panel(
        ui,
        super::LogLogSpec {
            symbol: "BTC",
            points: &daily,
            live: app.snap.btc.price,
            observed_unix: app.snap.price_observed_unix("BTC"),
            color: AMBER(),
            genesis: model::GENESIS_UNIX,
            cutoff: model::POWER_LAW_CUTOFF,
            day_zero: "genesis block, 3 Jan 2009",
            source,
        },
        340.0,
    );
    ui.add_space(GAP);
    let book = app.snap.terminal.clone();
    let funding_hist = app.snap.btc.funding_hist.clone();
    widgets::masonry(
        ui,
        eframe::egui::Id::new("btc-panels"),
        8,
        900.0,
        |ui, i| match i {
            0 => liquidity(ui, &book),
            1 => funding(ui, &book, &funding_hist),
            2 => etf_flows(ui, &book),
            3 => correlations(ui, &book),
            4 => premium(ui, &book),
            5 => regime(ui, &book),
            6 => emotion(ui, &book),
            _ => strategy(ui, &book),
        },
    );
    ui.add_space(GAP);
    super::dollar::macro_table(
        ui,
        &book,
        "Macro backdrop",
        "a rising print is risk-off for Bitcoin",
    );
    ui.add_space(GAP);
    widgets::footnote(
        ui,
        "Spot: Binance. Daily reference price: Coin Metrics (Blockchain.com sampled history as fallback). On-chain: bitcoin-data.com. Liquidity and macro: FRED, CNBC. Funding: Binance, Bybit, OKX. Premium: Coinbase vs Binance hourly closes. ETF flows: Farside. Strategy: SEC EDGAR 8-K filings and Yahoo Finance. The power-law fit is descriptive, not a forecast.",
    );
}

fn hero(app: &Desktop, ui: &mut Ui) {
    let btc = &app.snap.btc;
    let now = now_unix();
    ui.label(caps_job("Bitcoin · BTC/USDT", 10.5, AMBER()));
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 12.0;
        ui.label(
            RichText::new(btc.price.map(model::fmt_usd).unwrap_or_else(|| "—".into()))
                .font(medium(30.0))
                .color(TEXT()),
        );
        if let Some(change) = btc.change_24h_pct {
            ui.label(
                RichText::new(format!("{} 24h", fmt_change_pct(change)))
                    .font(mono(14.0))
                    .color(if change >= 0.0 { GREEN() } else { RED() }),
            );
        }
        if let (Some(lo), Some(hi), Some(px)) = (btc.low_24h, btc.high_24h, btc.price) {
            range_widget(ui, "24h", lo, hi, px);
        }
        let age = format!(
            "{} · {}",
            btc.price_source,
            app.snap.price_as_of("BTC", now)
        );
        ui.label(RichText::new(age).font(sans(11.5)).color(FAINT()));
    });
}

/// "24h  80,809 ━━━━●━━ 82,941" with the marker at the current price.
pub(crate) fn range_widget(ui: &mut Ui, label: &str, lo: f64, hi: f64, value: f64) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        ui.label(RichText::new(label).font(sans(11.0)).color(FAINT()));
        ui.label(
            RichText::new(model::fmt_price(lo))
                .font(mono(11.5))
                .color(MUTED()),
        );
        let (rect, _) = ui.allocate_exact_size(Vec2::new(90.0, 14.0), Sense::hover());
        charts::range_bar(ui.painter(), rect, lo, hi, value, TEXT());
        ui.label(
            RichText::new(model::fmt_price(hi))
                .font(mono(11.5))
                .color(MUTED()),
        );
    });
}

fn key_stats(app: &Desktop, ui: &mut Ui) {
    let snap = &app.snap;
    let t = &snap.terminal;
    let now = now_unix();
    let price = snap.btc.price;
    let fit = fit_power_law(&snap.btc.daily);
    let z = fit
        .as_ref()
        .and_then(|fit| price.and_then(|px| power_z(fit, now, px)));
    let fair = fit.as_ref().and_then(|fit| power_value(fit, now, 0.0));
    let sma = trailing_mean(&snap.btc.weekly, 200);
    let vs = |level: Option<f64>| match (price, level) {
        (Some(px), Some(level)) if level > 0.0 => {
            let pct = (px / level - 1.0) * 100.0;
            format!(
                "price {} {}",
                fmt_change_pct(pct),
                if pct >= 0.0 { "above" } else { "below" }
            )
        }
        _ => String::new(),
    };
    let dash = || "—".to_string();
    let ath = t.ath.or_else(|| model::ath(&snap.btc.daily).map(|p| p.v));
    let ath_t = t
        .ath_unix
        .or_else(|| model::ath(&snap.btc.daily).map(|p| p.t));
    let since_halving = (now - LAST_HALVING_UNIX) / SECONDS_PER_DAY;
    // 210,000 blocks at ten minutes is about 1,458 days per epoch.
    let next_halving =
        chrono::DateTime::from_timestamp(LAST_HALVING_UNIX + 1458 * SECONDS_PER_DAY, 0)
            .map(|d| d.format("%b %Y").to_string())
            .unwrap_or_default();
    let nupl_zone = |v: f64| match v {
        v if v < 0.0 => "capitulation",
        v if v < 0.25 => "hope / fear",
        v if v < 0.5 => "optimism / anxiety",
        v if v < 0.75 => "belief / denial",
        _ => "euphoria / greed",
    };
    let money_m = |v: f64| {
        let text = if v.abs() >= 1000.0 {
            format!("${:.2}B", v.abs() / 1000.0)
        } else {
            format!("${:.0}M", v.abs())
        };
        if v < 0.0 {
            format!("−{text}")
        } else {
            format!("+{text}")
        }
    };
    let stats = vec![
        Stat::new("Power-law z", z.map(|v| with_minus(format!("{v:+.2}σ"))).unwrap_or_else(dash))
            .tone(z.map(|v| status_color(z_status(v))).unwrap_or(TEXT()))
            .sub(fair.map(|f| format!("fair value {}", model::fmt_usd(f))).unwrap_or_default(), FAINT())
            .hint("Distance of the log price from the power-law trend, in standard deviations of the fit residuals."),
        Stat::new("200-week SMA", sma.map(model::fmt_usd).unwrap_or_else(dash))
            .sub(vs(sma), FAINT())
            .hint("Average of the last 200 weekly closes (Binance)."),
        Stat::new("Mayer multiple", t.mayer.map(|v| format!("{v:.2}")).unwrap_or_else(dash))
            .sub("price / 200-day average", FAINT()),
        Stat::new("MVRV Z-score", t.mvrv_z.map(|v| with_minus(format!("{v:+.2}"))).unwrap_or_else(dash))
            .tone(t.mvrv_z.map(|v| status_color(z_status(v))).unwrap_or(TEXT()))
            .sub("market vs realized value", FAINT()),
        Stat::new("NUPL", t.nupl.map(|v| format!("{v:.2}")).unwrap_or_else(dash))
            .sub(t.nupl.map(nupl_zone).unwrap_or_default(), FAINT())
            .hint("Net unrealized profit or loss as a share of market cap."),
        Stat::new("Realized price", t.realized.map(model::fmt_usd).unwrap_or_else(dash))
            .sub(vs(t.realized), FAINT()),
        Stat::new("All-time high", ath.map(model::fmt_usd).unwrap_or_else(dash)).sub(
            ath_t
                .and_then(|t| chrono::DateTime::from_timestamp(t, 0))
                .map(|d| d.format("%-d %b %Y").to_string())
                .unwrap_or_default(),
            FAINT(),
        ),
        Stat::new(
            "From ATH",
            match (price, ath) {
                (Some(px), Some(ath)) if ath > 0.0 => fmt_change_pct((px / ath - 1.0) * 100.0),
                _ => dash(),
            },
        )
        .tone(RED().gamma_multiply(0.9))
        .sub(
            ath_t
                .map(|t| format!("{} days since", (now - t) / SECONDS_PER_DAY))
                .unwrap_or_default(),
            FAINT(),
        ),
        Stat::new("Halving cycle", format!("day {since_halving}"))
            .sub(format!("next halving ≈ {next_halving}"), FAINT())
            .hint("Days since the April 2024 halving; the next one is estimated at ten-minute blocks."),
        Stat::new("Fear & Greed", snap.btc.fng.map(|v| v.to_string()).unwrap_or_else(dash))
            .tone(snap.btc.fng.map(|v| status_color(model::fng_status(v))).unwrap_or(TEXT()))
            .sub(snap.btc.fng_label.clone(), FAINT()),
        Stat::new("Funding · Binance", funding_text(snap.btc.funding))
            .tone(snap.btc.funding.map(|v| status_color(funding_status(v))).unwrap_or(TEXT()))
            .sub(
                t.funding_ann
                    .map(|v| format!("3-venue 7d {} ann.", fmt_change_pct(v * 100.0)))
                    .unwrap_or_default(),
                FAINT(),
            ),
        Stat::new(
            "Coinbase premium",
            t.premium_bps
                .map(|v| with_minus(format!("{v:+.1} bp")))
                .unwrap_or_else(dash),
        )
        .tone(crate::theme::move_color(t.premium_bps))
        .sub(
            t.premium_avg_bps
                .map(|v| with_minus(format!("24h avg {v:+.1} bp")))
                .unwrap_or_default(),
            FAINT(),
        ),
        Stat::new("ETF flow · latest", t.etf_latest_m.map(money_m).unwrap_or_else(dash))
            .tone(crate::theme::move_color(t.etf_latest_m))
            .sub(
                t.etf_7d_m
                    .map(|v| format!("7d avg {}/day", money_m(v)))
                    .unwrap_or_default(),
                FAINT(),
            ),
        Stat::new(
            "Market emotion",
            if t.mei_state.is_empty() {
                dash()
            } else {
                t.mei_state.clone()
            },
        )
        .tone(AMBER())
        .sub(
            match (t.mei_valence, t.mei_arousal) {
                (Some(v), Some(a)) => format!("valence {v:.0} · arousal {a:.0}"),
                _ => String::new(),
            },
            FAINT(),
        ),
        Stat::new(
            "Macro regime",
            t.regime_score
                .map(|v| signed_int(v as f64))
                .unwrap_or_else(dash),
        )
        .tone(crate::theme::move_color(t.regime_score.map(f64::from)))
        .sub(
            "five inputs · heuristic",
            FAINT(),
        ),
    ];
    widgets::panel(
        ui,
        "Key readings",
        "valuation · cycle · sentiment and flows",
        |ui| {
            widgets::stat_grid(ui, &stats, 150.0);
        },
    );
}

fn liquidity(ui: &mut Ui, t: &Terminal) {
    let meta = match (t.net_liq_b, t.net_liq_30d_b) {
        (Some(level), Some(delta)) => format!(
            "${:.2}T · {} over 30 days",
            level / 1000.0,
            with_minus(format!("{delta:+.0}B"))
        ),
        (Some(level), None) => format!("${:.2}T", level / 1000.0),
        _ => String::new(),
    };
    widgets::panel(ui, "Fed net liquidity", &meta, |ui| {
        let axis = |v: f64| format!("${}", charts::axis_number(v / 1000.0)) + "T";
        let value = |v: f64| format!("${}B", model::fmt_grouped(v, 0));
        charts::time_chart(
            ui,
            TimeChart::new(150.0, &axis, &value)
                .line(Line::new("Net", &t.net_liq_hist, CYAN()).fill()),
        );
        widgets::footnote(ui, "Fed balance sheet (WALCL) minus the Treasury General Account and reverse repo, daily forward-filled.");
    });
}

fn funding(ui: &mut Ui, t: &Terminal, binance_hist: &[f64]) {
    let meta = t
        .funding_ann
        .map(|v| {
            format!(
                "3-venue 7-day average {} annualized",
                fmt_change_pct(v * 100.0)
            )
        })
        .unwrap_or_default();
    widgets::panel(ui, "Perpetual funding", &meta, |ui| {
        let cols = [
            col("Venue", W::Flex(1.0), Align::Min),
            col("Now · 8h", W::Px(84.0), Align::Max),
            col("7-day avg", W::Px(84.0), Align::Max),
            col("Annualized", W::Px(84.0), Align::Max),
        ];
        let (layout, _) = widgets::table_header(ui, &cols, None);
        for venue in &t.venues {
            let (rect, _) = widgets::table_row(ui, 25.0, false, false);
            let painter = ui.painter();
            widgets::cell_text(
                painter,
                layout.cell(rect, 0),
                &venue.name,
                sans(12.5),
                TEXT(),
                Align::Min,
            );
            let now = funding_bp(venue.current);
            widgets::cell_text(
                painter,
                layout.cell(rect, 1),
                &now,
                mono(12.0),
                venue
                    .current
                    .map(|v| status_color(funding_status(v)))
                    .unwrap_or(FAINT()),
                Align::Max,
            );
            widgets::cell_text(
                painter,
                layout.cell(rect, 2),
                &funding_bp(venue.avg_7d),
                mono(12.0),
                MUTED(),
                Align::Max,
            );
            let ann = venue
                .avg_7d
                .map(|v| fmt_change_pct(v * 3.0 * 365.0 * 100.0))
                .unwrap_or_else(|| "—".into());
            widgets::cell_text(
                painter,
                layout.cell(rect, 3),
                &ann,
                mono(12.0),
                MUTED(),
                Align::Max,
            );
        }
        ui.add_space(8.0);
        let points = funding_points(binance_hist, now_unix());
        let axis = |v: f64| charts::axis_number(v);
        let value = |v: f64| with_minus(format!("{v:+.2} bp / 8h"));
        charts::bars(ui, 110.0, &points, &axis, &value);
        widgets::footnote(ui, "Binance BTCUSDT funding per 8-hour interval, basis points. Positive means longs pay shorts.");
    });
}

/// Binance settles funding at 00:00, 08:00 and 16:00 UTC; date the newest-last rates.
pub(crate) fn funding_points(rates: &[f64], now: i64) -> Vec<Point> {
    let step = 8 * 3600;
    let last = now.div_euclid(step) * step;
    let n = rates.len() as i64;
    rates
        .iter()
        .enumerate()
        .map(|(i, r)| Point {
            t: last - (n - 1 - i as i64) * step,
            v: r * 10_000.0,
        })
        .collect()
}

fn premium(ui: &mut Ui, t: &Terminal) {
    let meta = match (t.premium_bps, t.premium_avg_bps) {
        (Some(now), Some(avg)) => with_minus(format!("now {now:+.1} bp · 24h avg {avg:+.1} bp")),
        _ => String::new(),
    };
    widgets::panel(ui, "Coinbase premium", &meta, |ui| {
        let axis = |v: f64| charts::axis_number(v);
        let value = |v: f64| with_minus(format!("{v:+.1} bp"));
        charts::time_chart(
            ui,
            TimeChart::new(140.0, &axis, &value)
                .line(Line::new("Premium", &t.premium_hist, AMBER()))
                .zero_line()
                .hourly(),
        );
        widgets::footnote(ui, "Coinbase BTC-USD minus Binance BTC-USDT hourly close, in basis points. Persistent premium hints at US spot demand.");
    });
}

fn etf_flows(ui: &mut Ui, t: &Terminal) {
    let fmt_m = |v: f64| {
        let text = if v.abs() >= 1000.0 {
            format!("${:.2}B", v.abs() / 1000.0)
        } else {
            format!("${:.0}M", v.abs())
        };
        if v < 0.0 {
            format!("−{text}")
        } else {
            format!("+{text}")
        }
    };
    let latest = t
        .etf_latest_unix
        .and_then(|t| chrono::DateTime::from_timestamp(t, 0))
        .map(|d| d.format("%-d %b").to_string())
        .unwrap_or_default();
    widgets::panel(
        ui,
        "US spot ETF flows",
        &format!("daily net flow · latest {latest}"),
        |ui| {
            let stats = [
                Stat::new(
                    "Latest day",
                    t.etf_latest_m.map(fmt_m).unwrap_or_else(|| "—".into()),
                )
                .tone(crate::theme::move_color(t.etf_latest_m)),
                Stat::new(
                    "7-day avg",
                    t.etf_7d_m
                        .map(|v| fmt_m(v) + "/d")
                        .unwrap_or_else(|| "—".into()),
                )
                .tone(crate::theme::move_color(t.etf_7d_m)),
                Stat::new(
                    "30-day avg",
                    t.etf_30d_m
                        .map(|v| fmt_m(v) + "/d")
                        .unwrap_or_else(|| "—".into()),
                )
                .tone(crate::theme::move_color(t.etf_30d_m)),
                Stat::new(
                    "Since launch",
                    t.etf_cum_m.map(fmt_m).unwrap_or_else(|| "—".into()),
                ),
            ];
            widgets::stat_grid(ui, &stats, 110.0);
            ui.add_space(6.0);
            let recent = &t.etf_flows[t.etf_flows.len().saturating_sub(60)..];
            let axis = |v: f64| charts::axis_number(v);
            let value = |v: f64| fmt_m(v);
            charts::bars(ui, 130.0, recent, &axis, &value);
            widgets::footnote(ui, "Farside daily totals across US spot Bitcoin ETFs, USD millions, last 60 trading days.");
        },
    );
}

fn strategy(ui: &mut Ui, t: &Terminal) {
    let filed = t
        .strc_filed
        .map(|f| format!("latest 8-K {}", fmt_age(f, now_unix())))
        .unwrap_or_default();
    widgets::panel(ui, "Strategy (MSTR) issuance", &filed, |ui| {
        let usd_m = |v: Option<f64>| {
            v.map(|v| format!("${}M", model::fmt_grouped(v, 0)))
                .unwrap_or_else(|| "—".into())
        };
        let flywheel = match t.strc_price {
            Some(p) if p >= 100.0 => "at or above par: ATM issuance can run",
            Some(p) if p >= 95.0 => "below par: issuance slows",
            Some(_) => "well below par",
            None => "",
        };
        let stats = [
            Stat::new(
                "STRC preferred",
                t.strc_price
                    .map(|p| format!("${p:.2}"))
                    .unwrap_or_else(|| "—".into()),
            )
            .tone(match t.strc_price {
                Some(p) if p >= 99.0 => GREEN(),
                Some(p) if p >= 95.0 => AMBER(),
                Some(_) => RED(),
                None => TEXT(),
            })
            .sub(
                t.strc_change.map(fmt_change_pct).unwrap_or_default(),
                crate::theme::move_color(t.strc_change),
            )
            .hint(flywheel),
            Stat::new("STRC last 8-K", usd_m(t.strc_week_m)).sub(
                t.strc_btc
                    .map(|v| format!("≈ {v:.0} BTC at the week's average"))
                    .unwrap_or_default(),
                FAINT(),
            ),
            Stat::new("STRC recent 8-Ks", usd_m(t.strc_recent_m))
                .sub(format!("{} filings read", t.strc_filings), FAINT()),
            Stat::new(
                "ATM remaining",
                t.atm_cap_b
                    .map(|v| format!("${v:.1}B"))
                    .unwrap_or_else(|| "—".into()),
            )
            .sub(format!("STRC {}", usd_m(t.strc_left_m)), FAINT()),
        ];
        widgets::stat_grid(ui, &stats, 120.0);
        ui.add_space(6.0);
        if t.strc_hist.iter().all(|p| p.v == 0.0) {
            widgets::footnote(ui, "No STRC sold in the 8-K tables read so far; issuance pauses while the preferred trades below par.");
        } else {
            let axis = |v: f64| charts::axis_number(v);
            let value = |v: f64| format!("${}M", model::fmt_grouped(v, 1));
            charts::bars(ui, 100.0, &t.strc_hist, &axis, &value);
            widgets::footnote(
                ui,
                "Net STRC proceeds per 8-K at-the-market table, USD millions.",
            );
        }
    });
}

fn correlations(ui: &mut Ui, t: &Terminal) {
    widgets::panel(ui, "Correlation with Bitcoin", "daily log returns", |ui| {
        if t.correlations.is_empty() {
            widgets::empty(ui, 60.0, "Waiting for aligned price histories.");
            return;
        }
        let cols = [
            col("Asset", W::Flex(1.0), Align::Min),
            col("30 days", W::Px(120.0), Align::Center),
            col("90 days", W::Px(120.0), Align::Center),
        ];
        let (layout, _) = widgets::table_header(ui, &cols, None);
        for row in &t.correlations {
            let (rect, _) = widgets::table_row(ui, 27.0, false, false);
            let painter = ui.painter();
            widgets::cell_text(
                painter,
                layout.cell(rect, 0),
                &row.label,
                sans(12.5),
                TEXT(),
                Align::Min,
            );
            for (i, v) in [(1, row.r30), (2, row.r90)] {
                let cell = layout.cell(rect, i).shrink2(Vec2::new(0.0, 3.0));
                if let Some(v) = v {
                    let color = if v >= 0.0 { GREEN() } else { RED() };
                    painter.rect_filled(
                        cell,
                        crate::theme::corner(3),
                        color.gamma_multiply(0.08 + v.abs() as f32 * 0.45),
                    );
                }
                widgets::cell_text(
                    painter,
                    cell,
                    &v.map(|v| with_minus(format!("{v:+.2}")))
                        .unwrap_or_else(|| "—".into()),
                    mono(12.0),
                    TEXT(),
                    Align::Center,
                );
            }
        }
        widgets::footnote(ui, "−1 moves opposite to Bitcoin, +1 moves with it. Broad USD is the FRED trade-weighted index.");
    });
}

fn regime(ui: &mut Ui, t: &Terminal) {
    let meta = t
        .regime_score
        .map(|score| format!("composite {} · heuristic", signed_int(score as f64)))
        .unwrap_or_default();
    widgets::panel(ui, "Macro regime score", &meta, |ui| {
        if t.regime_inputs.is_empty() {
            widgets::empty(
                ui,
                60.0,
                "Needs Fed, M2, MVRV, weekly Bitcoin and dollar history.",
            );
            return;
        }
        let cols = [
            col("Input", W::Px(118.0), Align::Min),
            col("Score", W::Px(44.0), Align::Max),
            col("Reading", W::Flex(1.0), Align::Min),
        ];
        let (layout, _) = widgets::table_header(ui, &cols, None);
        for row in &t.regime_inputs {
            let (rect, _) = widgets::table_row(ui, 25.0, false, false);
            let painter = ui.painter();
            widgets::cell_text(
                painter,
                layout.cell(rect, 0),
                &row.label,
                sans(12.5),
                TEXT(),
                Align::Min,
            );
            widgets::cell_text(
                painter,
                layout.cell(rect, 1),
                &signed_int(row.score as f64),
                mono(12.0),
                crate::theme::move_color(Some(row.score as f64)),
                Align::Max,
            );
            widgets::cell_text(
                painter,
                layout.cell(rect, 2),
                &row.detail,
                sans(11.5),
                MUTED(),
                Align::Min,
            );
        }
        ui.add_space(8.0);
        let axis = |v: f64| signed_int(v);
        let value = |v: f64| signed_int(v);
        charts::time_chart(
            ui,
            TimeChart::new(110.0, &axis, &value)
                .line(Line::new("Score", &t.regime_hist, BLUE()))
                .zero_line(),
        );
        widgets::footnote(ui, "Each input scores −1, 0 or +1. The rows show what moves the score; weekly history covers two years. This heuristic has no validated allocation performance.");
    });
}

fn emotion(ui: &mut Ui, t: &Terminal) {
    let meta = if t.mei_state.is_empty() {
        String::new()
    } else {
        format!(
            "{}{}",
            t.mei_state,
            t.mei_intensity
                .map(|v| format!(" · intensity {v:.2}"))
                .unwrap_or_default()
        )
    };
    widgets::panel(ui, "Market emotion", &meta, |ui| {
        let (rect, _) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), 168.0), Sense::hover());
        let plot = rect.shrink2(Vec2::new(18.0, 4.0));
        let painter = ui.painter();
        let states = [
            ["Panic", "Volatile", "Euphoria"],
            ["Anxiety", "Neutral", "Optimism"],
            ["Despondency", "Apathy", "Complacency"],
        ];
        for (r, row) in states.iter().enumerate() {
            for (c, state) in row.iter().enumerate() {
                let cell = Rect::from_min_size(
                    plot.min
                        + Vec2::new(
                            plot.width() * c as f32 / 3.0,
                            plot.height() * r as f32 / 3.0,
                        ),
                    Vec2::new(plot.width() / 3.0 - 3.0, plot.height() / 3.0 - 3.0),
                );
                let active = t.mei_state.eq_ignore_ascii_case(state);
                painter.rect_filled(
                    cell,
                    crate::theme::corner(4),
                    if active {
                        AMBER().gamma_multiply(0.18)
                    } else {
                        RAISED()
                    },
                );
                painter.text(
                    cell.center(),
                    Align2::CENTER_CENTER,
                    state,
                    sans(11.0),
                    if active { AMBER() } else { FAINT() },
                );
            }
        }
        if let (Some(v), Some(a)) = (t.mei_valence, t.mei_arousal) {
            let p = Pos2::new(
                plot.left() + plot.width() * (v / 100.0).clamp(0.0, 1.0) as f32,
                plot.bottom() - plot.height() * (a / 100.0).clamp(0.0, 1.0) as f32,
            );
            painter.circle_stroke(p, 6.0, Stroke::new(1.5, TEXT()));
            painter.circle_filled(p, 2.5, AMBER());
        }
        widgets::footnote(ui, "Columns run bearish to bullish (trend, momentum, drawdown); rows rise with activity (volatility, range, volume). Percentiles over two years.");
    });
}
