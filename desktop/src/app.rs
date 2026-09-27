use crate::draw::{
    self, AMBER, BG, BLUE, BORDER, CYAN, FAINT, GREEN, MUTED, PURPLE, SURFACE, TEXT,
};
use crate::pages;
use dollar_dashboard::cache;
use dollar_dashboard::catalog::Catalog;
use dollar_dashboard::fetch::{self, refresh};
use dollar_dashboard::model::{fallback_snapshot, now_unix, Snapshot};
use eframe::egui::{
    self, Align2, Color32, CornerRadius, Frame, Key, Layout, Margin, Rect, RichText, Sense, Stroke,
    Vec2,
};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Desk {
    Home,
    Btc,
    Dollar,
    Ai,
    Nuclear,
}

impl Desk {
    fn all() -> [Desk; 5] {
        [Self::Home, Self::Btc, Self::Dollar, Self::Ai, Self::Nuclear]
    }

    fn index(self) -> &'static str {
        match self {
            Self::Home => "00",
            Self::Btc => "01",
            Self::Dollar => "02",
            Self::Ai => "03",
            Self::Nuclear => "04",
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Home => "HOME",
            Self::Btc => "BTC",
            Self::Dollar => "DOLLAR",
            Self::Ai => "AI",
            Self::Nuclear => "NUCLEAR",
        }
    }

    pub(crate) fn accent(self) -> Color32 {
        match self {
            Self::Home => TEXT,
            Self::Btc => AMBER,
            Self::Dollar => BLUE,
            Self::Ai => PURPLE,
            Self::Nuclear => CYAN,
        }
    }
}

pub(crate) struct Desktop {
    pub(crate) desk: Desk,
    pub(crate) snap: Snapshot,
    pub(crate) catalog: Catalog,
    pub(crate) selected: Option<String>,
    pub(crate) filter: String,
    refreshing: bool,
    cache_error: String,
    tick_unix: i64,
    rx: Receiver<Snapshot>,
    tick_rx: Receiver<Vec<dollar_dashboard::parse::TickerPrint>>,
    cmd_tx: Sender<Snapshot>,
    refresh_at: Instant,
}

pub fn launch() -> std::process::ExitCode {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([1100.0, 700.0])
            .with_title("Market Monitor"),
        ..Default::default()
    };
    match eframe::run_native(
        "Market Monitor",
        options,
        Box::new(|cc| {
            draw::apply_style(&cc.egui_ctx);
            Ok(Box::new(Desktop::new()))
        }),
    ) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("Market Monitor could not open a window: {err}");
            std::process::ExitCode::FAILURE
        }
    }
}

impl Desktop {
    fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        let (cmd_tx, cmd_rx) = mpsc::channel::<Snapshot>();
        let (tick_tx, tick_rx) = mpsc::channel();
        thread::spawn(move || {
            while let Ok(prev) = cmd_rx.recv() {
                let snapshot = refresh(&prev);
                if tx.send(snapshot).is_err() {
                    break;
                }
            }
        });
        thread::spawn(move || loop {
            if let Some(rows) = fetch_tick() {
                if tick_tx.send(rows).is_err() {
                    break;
                }
            }
            thread::sleep(Duration::from_secs(2));
        });
        let mut app = Self {
            desk: Desk::Home,
            snap: cache::load().unwrap_or_else(fallback_snapshot),
            catalog: dollar_dashboard::catalog::load(),
            selected: None,
            filter: String::new(),
            refreshing: false,
            cache_error: String::new(),
            tick_unix: 0,
            rx,
            tick_rx,
            cmd_tx,
            refresh_at: Instant::now() - Duration::from_secs(30),
        };
        app.kick();
        app
    }

    fn kick(&mut self) {
        if self.refreshing {
            return;
        }
        self.refreshing = true;
        self.refresh_at = Instant::now();
        let _ = self.cmd_tx.send(self.snap.clone());
    }

    fn pump(&mut self, ctx: &egui::Context) {
        while let Ok(snapshot) = self.rx.try_recv() {
            self.snap = snapshot;
            self.refreshing = false;
            if let Err(err) = cache::save(&self.snap) {
                self.cache_error = err;
            }
        }
        while let Ok(rows) = self.tick_rx.try_recv() {
            fetch::apply_tickers(&mut self.snap.btc, &rows);
            self.tick_unix = now_unix();
        }
        let typing = ctx.egui_wants_keyboard_input();
        if !typing {
            let mut next = None;
            let mut refresh = false;
            let mut clear = false;
            ctx.input(|input| {
                for (key, desk) in [
                    (Key::Num1, Desk::Home),
                    (Key::Num2, Desk::Btc),
                    (Key::Num3, Desk::Dollar),
                    (Key::Num4, Desk::Ai),
                    (Key::Num5, Desk::Nuclear),
                ] {
                    if input.key_pressed(key) {
                        next = Some(desk);
                    }
                }
                refresh = input.key_pressed(Key::R);
                clear = input.key_pressed(Key::Escape);
            });
            if let Some(desk) = next {
                self.desk = desk;
                self.selected = None;
            }
            if clear {
                self.selected = None;
                self.filter.clear();
            }
            if refresh && self.refresh_at.elapsed() > Duration::from_secs(2) {
                self.kick();
            }
        }
        if !self.refreshing && self.refresh_at.elapsed() > Duration::from_secs(15 * 60) {
            self.kick();
        }
        ctx.request_repaint_after(Duration::from_secs(1));
    }
}

impl eframe::App for Desktop {
    fn ui(&mut self, root: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = root.ctx().clone();
        self.pump(&ctx);
        self.ticker(root);
        self.nav(root);
        self.status(root);
        if self.selected.is_some() && self.desk != Desk::Home {
            egui::Panel::right("detail")
                .exact_size(340.0)
                .frame(
                    Frame::new()
                        .fill(BG)
                        .inner_margin(Margin::symmetric(10, 10)),
                )
                .show(root, |ui| pages::detail(self, ui));
        }
        egui::CentralPanel::default()
            .frame(
                Frame::new()
                    .fill(BG)
                    .inner_margin(Margin::symmetric(16, 12)),
            )
            .show(root, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("desk")
                    .auto_shrink([false, false])
                    .show(ui, |ui| pages::desk(self, ui));
            });
    }
}

impl Desktop {
    fn ticker(&self, root: &mut egui::Ui) {
        egui::Panel::top("ticker")
            .exact_size(30.0)
            .frame(Frame::new().fill(BG))
            .show(root, |ui| {
                egui::ScrollArea::horizontal()
                    .id_salt("ticks")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 0.0;
                            let btc = self.snap.btc.price.map(dollar_dashboard::model::fmt_usd);
                            let btc_chg = self.snap.btc.change_24h_pct;
                            tick(ui, "BTC", btc.as_deref().unwrap_or("—"), btc_chg, 0);
                            let eth = self
                                .snap
                                .btc
                                .eth_price
                                .map(dollar_dashboard::model::fmt_usd);
                            tick(
                                ui,
                                "ETH",
                                eth.as_deref().unwrap_or("—"),
                                self.snap.btc.eth_change_24h_pct,
                                0,
                            );
                            tick_obs(ui, "DXY", self.snap.dollar.dxy.as_ref(), 2, false);
                            tick_obs(ui, "US10Y", self.snap.dollar.ust10.as_ref(), 2, true);
                            tick_obs(ui, "VIX", self.snap.dollar.vix.as_ref(), 1, false);
                            tick_obs(ui, "GOLD", self.snap.dollar.gold.as_ref(), 0, false);
                            tick_obs(ui, "SPX", self.snap.dollar.spx.as_ref(), 0, false);
                            let fng = self
                                .snap
                                .btc
                                .fng
                                .map(|v| format!("{v} {}", self.snap.btc.fng_label))
                                .unwrap_or_else(|| "—".into());
                            tick(ui, "F&G", &fng, None, 0);
                        });
                    });
            });
    }

    fn nav(&mut self, root: &mut egui::Ui) {
        egui::Panel::top("nav")
            .exact_size(44.0)
            .frame(Frame::new().fill(SURFACE).stroke(Stroke::new(1.0, BORDER)))
            .show(root, |ui| {
                let full = ui.available_width();
                let brand_w = if full >= 1100.0 { 200.0 } else { 52.0 };
                let tab_w = ((full - brand_w) / 5.0).clamp(72.0, 140.0);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    let brand = ui.allocate_response(Vec2::new(brand_w, 44.0), Sense::click());
                    draw::label(
                        ui.painter(),
                        brand.rect.left_center() + Vec2::new(12.0, 0.0),
                        Align2::LEFT_CENTER,
                        if brand_w > 80.0 {
                            "MARKET MONITOR"
                        } else {
                            "MM"
                        },
                        12.0,
                        TEXT,
                        true,
                    );
                    if brand.clicked() {
                        self.desk = Desk::Home;
                        self.selected = None;
                    }
                    ui.painter().line_segment(
                        [brand.rect.right_top(), brand.rect.right_bottom()],
                        Stroke::new(1.0, BORDER),
                    );
                    for desk in Desk::all() {
                        let active = self.desk == desk;
                        let response = ui.allocate_response(Vec2::new(tab_w, 44.0), Sense::click());
                        if active || response.hovered() {
                            ui.painter()
                                .rect_filled(response.rect, CornerRadius::ZERO, BG);
                        }
                        if tab_w >= 100.0 {
                            draw::label(
                                ui.painter(),
                                response.rect.left_center() + Vec2::new(10.0, 0.0),
                                Align2::LEFT_CENTER,
                                desk.index(),
                                11.0,
                                FAINT,
                                true,
                            );
                        }
                        draw::label(
                            ui.painter(),
                            response.rect.left_center()
                                + Vec2::new(if tab_w >= 100.0 { 32.0 } else { 8.0 }, 0.0),
                            Align2::LEFT_CENTER,
                            desk.name(),
                            12.0,
                            if active { TEXT } else { MUTED },
                            true,
                        );
                        if active {
                            ui.painter().rect_filled(
                                Rect::from_min_max(
                                    response.rect.left_bottom() + Vec2::new(0.0, -2.0),
                                    response.rect.right_bottom(),
                                ),
                                CornerRadius::ZERO,
                                desk.accent(),
                            );
                        }
                        ui.painter().line_segment(
                            [response.rect.right_top(), response.rect.right_bottom()],
                            Stroke::new(1.0, BORDER),
                        );
                        if response.clicked() {
                            self.desk = desk;
                            self.selected = None;
                        }
                    }
                });
            });
    }

    fn status(&self, root: &mut egui::Ui) {
        egui::Panel::bottom("status")
            .exact_size(28.0)
            .frame(
                Frame::new()
                    .fill(SURFACE)
                    .inner_margin(Margin::symmetric(12, 4)),
            )
            .show(root, |ui| {
                ui.horizontal(|ui| {
                    let age = if self.snap.fetched_unix == 0 {
                        "no refresh yet".to_string()
                    } else {
                        let secs = (now_unix() - self.snap.fetched_unix).max(0);
                        if secs < 90 {
                            format!("{secs}s ago")
                        } else {
                            format!("{}m ago", secs / 60)
                        }
                    };
                    ui.label(
                        RichText::new(format!("1–5 desks   R refresh   Esc clear   {age}"))
                            .monospace()
                            .size(11.0)
                            .color(FAINT),
                    );
                    if !self.cache_error.is_empty() {
                        ui.label(
                            RichText::new(&self.cache_error)
                                .monospace()
                                .size(11.0)
                                .color(AMBER),
                        );
                    } else if self.snap.notes.is_empty() {
                        ui.label(
                            RichText::new("sources ok")
                                .monospace()
                                .size(11.0)
                                .color(GREEN),
                        );
                    } else {
                        ui.label(
                            RichText::new(draw::truncate(&self.snap.notes.join(" · "), 110))
                                .monospace()
                                .size(11.0)
                                .color(AMBER),
                        );
                    }
                    ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                        let clock = chrono::Utc::now().format("%H:%M:%S UTC").to_string();
                        ui.label(RichText::new(clock).monospace().size(11.0).color(MUTED));
                        let (state, color) = if self.refreshing {
                            ("REFRESH", AMBER)
                        } else if self.tick_unix > 0 && now_unix() - self.tick_unix < 8 {
                            ("LIVE", GREEN)
                        } else if self.snap.fetched_unix == 0 {
                            ("SNAPSHOT", MUTED)
                        } else {
                            ("CACHED", MUTED)
                        };
                        ui.label(RichText::new(state).monospace().size(11.0).color(color));
                        ui.label(
                            RichText::new("not advice")
                                .monospace()
                                .size(11.0)
                                .color(FAINT),
                        );
                    });
                });
            });
    }
}

fn tick(ui: &mut egui::Ui, symbol: &str, value: &str, change: Option<f64>, _digits: usize) {
    let response = ui.allocate_response(Vec2::new(132.0, 30.0), Sense::hover());
    let value_color = match change {
        Some(change) if change >= 0.05 => GREEN,
        Some(change) if change <= -0.05 => draw::RED,
        _ => TEXT,
    };
    draw::label(
        ui.painter(),
        response.rect.left_center() + Vec2::new(10.0, 0.0),
        Align2::LEFT_CENTER,
        symbol,
        11.0,
        FAINT,
        true,
    );
    draw::label(
        ui.painter(),
        response.rect.left_center() + Vec2::new(48.0, 0.0),
        Align2::LEFT_CENTER,
        value,
        12.0,
        value_color,
        true,
    );
    ui.painter().line_segment(
        [response.rect.right_top(), response.rect.right_bottom()],
        Stroke::new(1.0, BORDER),
    );
}

fn tick_obs(
    ui: &mut egui::Ui,
    symbol: &str,
    obs: Option<&dollar_dashboard::model::Obs>,
    digits: usize,
    percent: bool,
) {
    let text = obs
        .map(|obs| {
            if percent {
                format!("{:.digits$}%", obs.value)
            } else if digits == 0 {
                dollar_dashboard::model::fmt_grouped(obs.value, 0)
            } else {
                format!("{:.digits$}", obs.value)
            }
        })
        .unwrap_or_else(|| "—".into());
    tick(ui, symbol, &text, None, digits);
}

fn fetch_tick() -> Option<Vec<dollar_dashboard::parse::TickerPrint>> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(6))
        .user_agent("dollar-dashboard/0.1")
        .build()
        .ok()?;
    let response = client
        .get("https://api.binance.com/api/v3/ticker/24hr?symbols=%5B%22BTCUSDT%22,%22ETHUSDT%22%5D")
        .send()
        .ok()?;
    if !response.status().is_success() {
        return None;
    }
    let body = response.text().ok()?;
    let rows = dollar_dashboard::parse::binance_tickers(&body);
    (!rows.is_empty()).then_some(rows)
}
