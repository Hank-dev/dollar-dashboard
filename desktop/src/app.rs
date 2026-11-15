use crate::theme::{self, AMBER, BG, BLUE, CYAN, GREEN, ORANGE, PURPLE};
use crate::{chrome, desks, dialogs, widgets};
use dollar_dashboard::cache;
use dollar_dashboard::catalog::Catalog;
use dollar_dashboard::fetch::{self, refresh};
use dollar_dashboard::model::{fallback_snapshot, now_unix, Snapshot};
use dollar_dashboard::workspace::{Direction, Preferences, VisitBaseline};
use eframe::egui::{self, Color32, Frame, Key, Margin, RichText, Stroke};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Desk {
    Home,
    Btc,
    Dollar,
    Ai,
    Nuclear,
    Markets,
    Zcash,
    SpaceX,
    Pulse,
}

impl Desk {
    pub(crate) fn all() -> [Desk; 9] {
        [
            Self::Home,
            Self::Btc,
            Self::Dollar,
            Self::Ai,
            Self::Nuclear,
            Self::Markets,
            Self::Zcash,
            Self::SpaceX,
            Self::Pulse,
        ]
    }

    /// The number key that opens this desk.
    pub(crate) fn key(self) -> usize {
        Self::all().iter().position(|d| *d == self).unwrap_or(0) + 1
    }

    pub(crate) fn accent(self) -> Color32 {
        match self {
            Self::Home => theme::ACCENT(),
            Self::Btc => AMBER(),
            Self::Dollar => BLUE(),
            Self::Ai => PURPLE(),
            Self::Nuclear => CYAN(),
            Self::Markets => GREEN(),
            Self::Zcash => ORANGE(),
            Self::SpaceX => BLUE(),
            Self::Pulse => PURPLE(),
        }
    }

    pub(crate) fn title(self) -> &'static str {
        match self {
            Self::Home => "Overview",
            Self::Btc => "Bitcoin",
            Self::Dollar => "Dollar & macro",
            Self::Ai => "AI infrastructure",
            Self::Nuclear => "Nuclear energy",
            Self::Markets => "Markets & watchlist",
            Self::Zcash => "Zcash",
            Self::SpaceX => "SpaceX",
            Self::Pulse => "X Pulse",
        }
    }
}

pub(crate) struct Desktop {
    pub(crate) codex: crate::codex::Chat,
    pub(crate) desk: Desk,
    pub(crate) snap: Snapshot,
    pub(crate) catalog: Catalog,
    pub(crate) selected: Option<String>,
    pub(crate) filter: String,
    pub(crate) refreshing: bool,
    pub(crate) cache_error: String,
    pub(crate) tick_unix: i64,
    rx: Receiver<Snapshot>,
    tick_rx: Receiver<dollar_dashboard::parse::TickerBatch>,
    cmd_tx: Sender<Snapshot>,
    pub(crate) refresh_at: Instant,
    pub(crate) pulse: Option<dollar_dashboard::pulse::Pulse>,
    pub(crate) pulse_busy: bool,
    pub(crate) pulse_error: String,
    pub(crate) pulse_sector: Option<dollar_dashboard::pulse::Sector>,
    pub(crate) pulse_import: String,
    pub(crate) pulse_prompt_at: Option<i64>,
    pulse_rx: Receiver<Result<dollar_dashboard::pulse::Pulse, String>>,
    pulse_tx: Sender<Result<dollar_dashboard::pulse::Pulse, String>>,
    pulse_attempt: Option<Instant>,
    pub(crate) prefs: Preferences,
    pub(crate) visit_baseline: Option<VisitBaseline>,
    /// Shared with the tick poller, which slows down while the window is unfocused.
    focused: Arc<AtomicBool>,
    /// When focus was lost, and the readings on screen at that moment.
    away: Option<(Instant, VisitBaseline)>,
    /// Last notification per rule, so a reading hovering on a threshold stays quiet.
    rule_notified: HashMap<&'static str, i64>,
    pub(crate) search: String,
    pub(crate) search_index: usize,
    pub(crate) show_search: bool,
    pub(crate) show_settings: bool,
    pub(crate) show_sources: bool,
    pub(crate) show_alerts: bool,
    pub(crate) watch_only: bool,
    pub(crate) sort_column: usize,
    pub(crate) sort_desc: bool,
    pub(crate) notice: String,
    pub(crate) alert_symbol: String,
    pub(crate) alert_value: String,
    pub(crate) alert_direction: Direction,
}

pub fn launch() -> std::process::ExitCode {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([900.0, 600.0])
            .with_app_id("io.github.hank-dev.MarketMonitor")
            .with_title("Market Monitor"),
        // Mesa's VSync wait can block swap_buffers while a Wayland workspace is
        // hidden, starving compositor pings. Idle repaints use our one-second timer.
        glow_options: eframe::egui_glow::GlowConfiguration {
            vsync: false,
            ..Default::default()
        },
        ..Default::default()
    };
    match eframe::run_native(
        "Market Monitor",
        options,
        Box::new(|cc| {
            theme::apply_style(&cc.egui_ctx);
            Ok(Box::new(Desktop::new(cc.storage, &cc.egui_ctx)))
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
    fn new(storage: Option<&dyn eframe::Storage>, ctx: &egui::Context) -> Self {
        let mut prefs: Preferences = storage
            .and_then(|s| eframe::get_value(s, "workspace"))
            .unwrap_or_default();
        prefs.scale = prefs.scale.clamp(0.85, 1.35);
        prefs.refresh_minutes = prefs.refresh_minutes.clamp(1, 60);
        ctx.set_zoom_factor(prefs.scale);
        let (tx, rx) = mpsc::channel();
        let (cmd_tx, cmd_rx) = mpsc::channel::<Snapshot>();
        let (tick_tx, tick_rx) = mpsc::channel();
        let (pulse_tx, pulse_rx) = mpsc::channel();
        thread::spawn(move || {
            while let Ok(prev) = cmd_rx.recv() {
                if tx.send(guarded_refresh(&prev, refresh)).is_err() {
                    break;
                }
            }
        });
        let focused = Arc::new(AtomicBool::new(true));
        let poll_focus = focused.clone();
        thread::spawn(move || {
            // One client for the life of the app keeps the poll on a warm connection.
            let client = reqwest::blocking::Client::builder()
                .timeout(Duration::from_secs(6))
                .user_agent("dollar-dashboard/0.1")
                .build();
            let mut failures = 0u32;
            loop {
                let retry_after = match client.as_ref().map_err(|_| None).and_then(fetch_tick) {
                    Ok(rows) => {
                        failures = 0;
                        if tick_tx.send(rows).is_err() {
                            break;
                        }
                        None
                    }
                    Err(retry_after) => {
                        failures = failures.saturating_add(1);
                        retry_after
                    }
                };
                thread::sleep(tick_delay(
                    failures,
                    poll_focus.load(Ordering::Relaxed),
                    retry_after,
                ));
            }
        });
        let mut app = Self {
            codex: crate::codex::Chat::restored(storage.and_then(|s| eframe::get_value(s, "codex_chat")).unwrap_or_default()),
            desk: Desk::all()
                .get(prefs.last_desk)
                .copied()
                .unwrap_or(Desk::Home),
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
            pulse: dollar_dashboard::pulse::load(),
            pulse_busy: false,
            pulse_error: String::new(),
            pulse_sector: None,
            pulse_import: String::new(),
            pulse_prompt_at: None,
            pulse_rx,
            pulse_tx,
            pulse_attempt: None,
            visit_baseline: prefs.last_visit.clone(),
            focused,
            away: None,
            rule_notified: HashMap::new(),
            prefs,
            search: String::new(),
            search_index: 0,
            show_search: false,
            show_settings: false,
            show_sources: false,
            show_alerts: false,
            watch_only: false,
            sort_column: 0,
            sort_desc: false,
            notice: String::new(),
            alert_symbol: "BTC".into(),
            alert_value: String::new(),
            alert_direction: Direction::Above,
        };
        app.kick();
        app
    }

    pub(crate) fn kick(&mut self) {
        if self.refreshing {
            return;
        }
        self.refreshing = true;
        self.refresh_at = Instant::now();
        let _ = self.cmd_tx.send(self.snap.clone());
    }

    pub(crate) fn refresh_pulse(&mut self) {
        if self.pulse_busy {
            return;
        }
        self.pulse_busy = true;
        self.pulse_error.clear();
        self.pulse_attempt = Some(Instant::now());
        let tx = self.pulse_tx.clone();
        thread::spawn(move || {
            let _ = tx.send(dollar_dashboard::pulse::fetch());
        });
    }

    pub(crate) fn import_pulse(&mut self) {
        let now = now_unix();
        let result = dollar_dashboard::pulse::parse_output(
            &self.pulse_import,
            self.pulse_prompt_at.unwrap_or(now),
            now,
        );
        if result.is_ok() {
            self.pulse_import.clear();
            self.pulse_prompt_at = None;
        }
        self.accept_pulse(result);
    }

    fn accept_pulse(&mut self, result: Result<dollar_dashboard::pulse::Pulse, String>) {
        match result {
            Ok(pulse) => {
                self.pulse_error = dollar_dashboard::pulse::save(&pulse)
                    .err()
                    .map(|e| format!("X report received, but cache could not be saved: {e}"))
                    .unwrap_or_default();
                if self.pulse_sector.is_some_and(|s| !pulse.report.topics.iter().any(|t| t.sector == s)) {
                    self.pulse_sector = None;
                }
                self.pulse = Some(pulse);
            }
            Err(error) => self.pulse_error = error,
        }
    }

    /// Seconds until the next scheduled source refresh.
    pub(crate) fn next_refresh_secs(&self) -> u64 {
        (self.prefs.refresh_minutes as u64 * 60).saturating_sub(self.refresh_at.elapsed().as_secs())
    }

    fn accept_refresh(&mut self, mut snapshot: Snapshot) -> bool {
        let updated = snapshot.fetched_unix > self.snap.fetched_unix;
        fetch::preserve_newer_spot(&mut snapshot, &self.snap);
        if updated && self.prefs.rule_alerts {
            self.announce_rule_changes(&snapshot, now_unix());
        }
        self.snap = snapshot;
        self.refreshing = false;
        updated
    }

    /// Notify once per rule and six hours when a new dated reading moves its status.
    fn announce_rule_changes(&mut self, incoming: &Snapshot, now: i64) -> usize {
        let mut sent = 0;
        for change in dollar_dashboard::changes::rule_changes(&self.snap, incoming, now) {
            if self
                .rule_notified
                .get(change.id)
                .is_some_and(|t| now - t < RULE_ALERT_COOLDOWN)
            {
                continue;
            }
            self.rule_notified.insert(change.id, now);
            self.notice = format!(
                "{}: {} → {} at {}",
                change.label,
                change.from.label(),
                change.to.label(),
                change.value
            );
            notify(
                &format!("{} {}", change.label, change.to.label()),
                &format!(
                    "{} → {} at {} · {}",
                    change.from.label(),
                    change.to.label(),
                    change.value,
                    change.rule
                ),
            );
            sent += 1;
        }
        sent
    }

    /// "Since your last visit" follows attention, not the process lifetime:
    /// after a long spell without focus, the readings from the moment focus
    /// was lost become the comparison.
    fn track_visit(&mut self, focused: bool, at: Instant) {
        self.focused.store(focused, Ordering::Relaxed);
        if !focused {
            if self.away.is_none() {
                self.away = Some((at, VisitBaseline::from_snapshot(&self.snap, now_unix())));
            }
        } else if let Some((left, baseline)) = self.away.take() {
            if at.saturating_duration_since(left) >= VISIT_GAP {
                self.visit_baseline = Some(baseline);
            }
        }
    }

    fn pump(&mut self, ctx: &egui::Context) {
        self.track_visit(ctx.input(|i| i.focused), Instant::now());
        while let Ok(result) = self.pulse_rx.try_recv() {
            self.pulse_busy = false;
            self.accept_pulse(result);
        }
        if self.prefs.x_pulse_hourly
            && !self.pulse_busy
            && dollar_dashboard::pulse::api_available()
            && self.pulse.as_ref().is_none_or(|p| p.stale(now_unix()))
            && self.pulse_attempt.is_none_or(|t| {
                t.elapsed() >= Duration::from_secs(dollar_dashboard::pulse::REFRESH_SECS as u64)
            })
        {
            self.refresh_pulse();
        }
        while let Ok(snapshot) = self.rx.try_recv() {
            let updated = self.accept_refresh(snapshot);
            if let Err(err) = cache::save(&self.snap) {
                self.cache_error = err;
            } else {
                self.cache_error.clear();
            }
            // A failed source can retain an old quote; alert only on crypto tick updates.
            if updated {
                let path = dollar_dashboard::journal::journal_path();
                if let Err(err) =
                    dollar_dashboard::journal::append_daily(&path, &self.snap, now_unix())
                {
                    self.cache_error = format!("readings log: {err}");
                }
                ctx.request_repaint();
            }
        }
        while let Ok(batch) = self.tick_rx.try_recv() {
            let applied = fetch::apply_ticker_batch(&mut self.snap, &batch);
            if !applied.is_empty() {
                self.tick_unix = self.tick_unix.max(batch.acquired_unix);
                self.snap
                    .feeds
                    .insert("binance-spot".into(), self.tick_unix);
            }
            let now = now_unix();
            let fresh: Vec<_> = applied
                .into_iter()
                .filter(|row| {
                    let symbol = row.0.trim_end_matches("USDT");
                    self.snap
                        .price_stamp(symbol)
                        .is_some_and(|s| s.live_spot(now))
                })
                .collect();
            self.check_alerts(&fresh);
        }
        if ctx.input(|i| i.modifiers.command && i.key_pressed(Key::K)) {
            self.show_search = !self.show_search;
            self.search_index = 0;
        }
        if ctx.input(|i| i.key_pressed(Key::Escape)) {
            self.show_search = false;
            self.show_settings = false;
            self.show_alerts = false;
            self.show_sources = false;
            self.selected = None;
        }
        let typing = ctx.egui_wants_keyboard_input();
        let dialog =
            self.show_search || self.show_settings || self.show_alerts || self.show_sources;
        if !typing && !dialog {
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
                    (Key::Num6, Desk::Markets),
                    (Key::Num7, Desk::Zcash),
                    (Key::Num8, Desk::SpaceX),
                    (Key::Num9, Desk::Pulse),
                ] {
                    if input.key_pressed(key) && !input.modifiers.any() {
                        next = Some(desk);
                    }
                }
                refresh = input.key_pressed(Key::R) && !input.modifiers.any();
                clear = input.key_pressed(Key::Escape);
            });
            if let Some(desk) = next {
                self.navigate(desk);
            }
            if clear {
                self.selected = None;
                self.filter.clear();
            }
            if refresh && self.refresh_at.elapsed() > Duration::from_secs(2) {
                self.kick();
            }
        }
        if !self.refreshing
            && self.refresh_at.elapsed()
                > Duration::from_secs(self.prefs.refresh_minutes as u64 * 60)
        {
            self.kick();
        }
        ctx.request_repaint_after(Duration::from_secs(1));
    }

    pub(crate) fn navigate(&mut self, desk: Desk) {
        self.desk = desk;
        self.selected = None;
        self.filter.clear();
    }

    fn check_alerts(&mut self, ticks: &[dollar_dashboard::parse::TickerPrint]) {
        // Price alerts use the successful live Binance tick, not delayed equity or cached data.
        for alert in &mut self.prefs.alerts {
            if !ticks
                .iter()
                .any(|row| row.0 == format!("{}USDT", alert.symbol))
            {
                continue;
            }
            let price = match alert.symbol.as_str() {
                "BTC" => self.snap.btc.price,
                "ETH" => self.snap.btc.eth_price,
                "ZEC" => self.snap.btc.zec_price,
                _ => None,
            };
            if let Some(price) = price {
                if alert.evaluate(price, self.tick_unix) {
                    let target = format!(
                        "{} {}",
                        if alert.direction == Direction::Above {
                            "at or above"
                        } else {
                            "at or below"
                        },
                        dollar_dashboard::model::fmt_usd(alert.threshold)
                    );
                    self.notice = format!(
                        "Price alert: {} is {} ({target})",
                        alert.symbol,
                        dollar_dashboard::model::fmt_usd(price),
                    );
                    self.show_alerts = true;
                    notify(
                        &format!(
                            "{} {}",
                            alert.symbol,
                            dollar_dashboard::model::fmt_usd(price)
                        ),
                        &format!("Price alert: {target}"),
                    );
                }
            }
        }
    }
}

/// Desktop notification through the session's notification daemon, if any.
fn notify(title: &str, body: &str) {
    #[cfg(not(test))]
    {
        let (title, body) = (title.to_string(), body.to_string());
        // Wait on a side thread so the child is reaped without blocking the UI.
        thread::spawn(move || {
            let _ = std::process::Command::new("notify-send")
                .args([
                    "--app-name=Market Monitor",
                    "--icon=io.github.hank-dev.MarketMonitor",
                    &title,
                    &body,
                ])
                .status();
        });
    }
    #[cfg(test)]
    let _ = (title, body);
}

impl eframe::App for Desktop {
    fn ui(&mut self, root: &mut egui::Ui, _frame: &mut eframe::Frame) {
        theme::watch(root.ctx());
        self.render(root);
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, "codex_chat", &self.codex.history);
        self.prefs.last_desk = self.desk.key() - 1;
        self.prefs.last_visit = Some(VisitBaseline::from_snapshot(&self.snap, now_unix()));
        eframe::set_value(storage, "workspace", &self.prefs);
    }

    fn auto_save_interval(&self) -> Duration {
        Duration::from_secs(30)
    }
}

impl Desktop {
    pub(crate) fn render(&mut self, root: &mut egui::Ui) {
        let ctx = root.ctx().clone();
        self.pump(&ctx);
        chrome::title_bar(self, root);
        chrome::ticker(self, root);
        chrome::status_bar(self, root);
        chrome::sidebar(self, root);
        if self.selected.is_some() && desks::has_detail(self.desk) {
            if ctx.content_rect().width() < 1150.0 {
                let mut open = true;
                egui::Window::new("Detail")
                    .open(&mut open)
                    .default_width(360.0)
                    .collapsible(false)
                    .show(&ctx, |ui| desks::detail(self, ui));
                if !open {
                    self.selected = None;
                }
            } else {
                egui::Panel::right("detail")
                    .exact_size(340.0)
                    .frame(
                        Frame::new()
                            .fill(theme::SURFACE())
                            .stroke(Stroke::new(1.0, theme::BORDER()))
                            .inner_margin(Margin::symmetric(16, 14)),
                    )
                    .show(root, |ui| {
                        ui.horizontal(|ui| {
                            widgets::caps(ui, "Detail", theme::FAINT());
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui
                                        .add(
                                            egui::Button::new(RichText::new("Close").size(11.5))
                                                .small(),
                                        )
                                        .on_hover_text("Esc")
                                        .clicked()
                                    {
                                        self.selected = None;
                                    }
                                },
                            );
                        });
                        ui.add_space(8.0);
                        desks::detail(self, ui);
                    });
            }
        }
        egui::CentralPanel::default()
            .frame(
                Frame::new()
                    .fill(BG())
                    .inner_margin(Margin::symmetric(20, 16)),
            )
            .show(root, |ui| {
                if !self.notice.is_empty() {
                    chrome::notice(self, ui);
                    ui.add_space(theme::GAP);
                }
                egui::ScrollArea::vertical()
                    .id_salt(("desk", self.desk.key()))
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        desks::show(self, ui);
                        ui.add_space(24.0);
                        #[cfg(test)]
                        assert!(
                            ui.min_rect().right() <= ui.max_rect().right() + 1.0,
                            "{} content overflowed: {} > {}",
                            self.desk.title(),
                            ui.min_rect().right(),
                            ui.max_rect().right()
                        );
                    });
            });
        dialogs::show(self, &ctx);
        crate::codex::show(self, &ctx);
    }
}

/// Time away from the window after which the next look counts as a new visit.
const VISIT_GAP: Duration = Duration::from_secs(4 * 3600);
const RULE_ALERT_COOLDOWN: i64 = 6 * 3600;

/// A panic inside a refresh must not end the worker: the UI would wait on it
/// for the rest of the session. Keep the previous data and say what happened.
fn guarded_refresh(prev: &Snapshot, run: impl FnOnce(&Snapshot) -> Snapshot) -> Snapshot {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run(prev))) {
        Ok(snapshot) => snapshot,
        Err(cause) => {
            let why = cause
                .downcast_ref::<&str>()
                .map(|s| s.to_string())
                .or_else(|| cause.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "unknown cause".into());
            let mut kept = prev.clone();
            kept.notes = vec![format!(
                "Refresh stopped on an unexpected response ({why}); previous data kept."
            )];
            kept
        }
    }
}

/// Spot poll cadence: 2 s focused, 10 s in the background (alerts still fire),
/// doubling on consecutive failures up to two minutes. A rate-limit response
/// names its own wait, which always wins.
fn tick_delay(failures: u32, focused: bool, retry_after: Option<u64>) -> Duration {
    let base: u64 = if focused { 2 } else { 10 };
    let backoff = base.saturating_mul(1u64 << failures.min(6)).min(120);
    Duration::from_secs(match retry_after {
        Some(wait) => wait.clamp(backoff, 900),
        None if failures == 0 => base,
        None => backoff,
    })
}

/// Err carries the server's Retry-After seconds when it sent one.
fn fetch_tick(
    client: &reqwest::blocking::Client,
) -> Result<dollar_dashboard::parse::TickerBatch, Option<u64>> {
    let response = client.get(fetch::SPOT_TICKERS).send().map_err(|_| None)?;
    if !response.status().is_success() {
        let wait = response
            .headers()
            .get(reqwest::header::RETRY_AFTER)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.trim().parse::<u64>().ok());
        // 429 and 418 without a header still mean "stop": wait at least a minute.
        let limited = matches!(response.status().as_u16(), 418 | 429);
        return Err(wait.or(limited.then_some(60)));
    }
    let body = response.text().map_err(|_| None)?;
    let batch = dollar_dashboard::parse::binance_batch(&body, now_unix());
    if batch.rows.is_empty() {
        return Err(None);
    }
    Ok(batch)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn fixture() -> Desktop {
        let (_, rx) = mpsc::channel();
        let (_, tick_rx) = mpsc::channel();
        let (cmd_tx, _) = mpsc::channel();
        let (pulse_tx, pulse_rx) = mpsc::channel();
        let mut snap = fallback_snapshot();
        snap.btc.price = Some(100_000.0);
        snap.btc.eth_price = Some(3_000.0);
        snap.btc.zec_price = Some(1_217.0);
        snap.btc.zec_change_24h_pct = Some(6.4);
        snap.btc.daily = (0..400)
            .map(|i| dollar_dashboard::model::Point {
                t: now_unix() - (400 - i) * 86400,
                v: 60_000.0 + i as f64 * 100.0,
            })
            .collect();
        Desktop {
            codex: crate::codex::Chat::default(),
            desk: Desk::Home,
            snap,
            catalog: dollar_dashboard::catalog::load(),
            selected: None,
            filter: String::new(),
            refreshing: false,
            cache_error: String::new(),
            tick_unix: 0,
            rx,
            tick_rx,
            cmd_tx,
            refresh_at: Instant::now(),
            pulse: None,
            pulse_busy: false,
            pulse_error: String::new(),
            pulse_sector: None,
            pulse_import: String::new(),
            pulse_prompt_at: None,
            pulse_rx,
            pulse_tx,
            pulse_attempt: None,
            prefs: Preferences::default(),
            visit_baseline: None,
            focused: Arc::new(AtomicBool::new(true)),
            away: None,
            rule_notified: HashMap::new(),
            search: String::new(),
            search_index: 0,
            show_search: false,
            show_settings: false,
            show_sources: false,
            show_alerts: false,
            watch_only: false,
            sort_column: 0,
            sort_desc: false,
            notice: String::new(),
            alert_symbol: "BTC".into(),
            alert_value: String::new(),
            alert_direction: Direction::Above,
        }
    }

    fn run(ctx: &egui::Context, app: &mut Desktop, size: [f32; 2]) -> egui::FullOutput {
        ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size.into())),
                ..Default::default()
            },
            |ui| app.render(ui),
        )
    }

    #[test]
    fn codex_chat_renders_at_laptop_and_monitor_sizes() {
        let ctx = egui::Context::default();
        theme::apply_style(&ctx);
        let mut app = fixture();
        app.codex.open = true;
        for size in [[900.0, 600.0], [1440.0, 900.0]] {
            for _ in 0..3 {
                let mut output = run(&ctx, &mut app, size);
                assert!(app.codex.open);
                assert!(output.shapes.iter().all(|s| s.clip_rect.is_finite()));
                output.textures_delta.clear();
            }
        }
    }

    #[test]
    fn all_desks_render_at_laptop_and_monitor_sizes() {
        for size in [
            [900.0, 600.0],
            [1440.0, 900.0],
            [1920.0, 1080.0],
            [2560.0, 1400.0],
        ] {
            let ctx = egui::Context::default();
            theme::apply_style(&ctx);
            let mut app = fixture();
            for desk in Desk::all() {
                app.navigate(desk);
                for _ in 0..2 {
                    let mut output = run(&ctx, &mut app, size);
                    assert!(
                        output.shapes.len() > 20,
                        "{} had no useful UI",
                        desk.title()
                    );
                    assert!(output
                        .shapes
                        .iter()
                        .all(|shape| shape.clip_rect.is_finite()));
                    output.textures_delta.clear();
                }
            }
        }
    }

    #[test]
    fn x_pulse_renders_sourced_report_and_keeps_it_after_failure() {
        use dollar_dashboard::pulse::{Confidence, Pulse, Report, Sector, Source, Tone, Topic};
        let mut app = fixture();
        let now = now_unix();
        app.pulse = Some(Pulse {
            origin: dollar_dashboard::pulse::Origin::GrokChat,
            requested_unix: now,
            completed_unix: now,
            report: Report {
                summary: "Split views on the next liquidity catalyst".into(),
                tone: Tone::Mixed,
                confidence: Confidence::Low,
                limitations: "Search sample; nuclear and Zcash not covered".into(),
                topics: vec![Topic {
                    sector: Sector::Bitcoin,
                    headline: "ETF demand versus tight liquidity".into(),
                    summary: "The retrieved posts disagree on whether flows can offset tighter funding.".into(),
                    tone: Tone::Mixed,
                    confidence: Confidence::Moderate,
                    bullish_case: "Sustained accumulation could absorb available supply.".into(),
                    bearish_case: "Funding stress could dominate spot inflows.".into(),
                    watch_next: "The next reported ETF flow and funding readings.".into(),
                    sources: vec![Source { url: "https://x.com/example/status/2118079587116614010".into() }],
                }],
            },
        });
        let ctx = egui::Context::default();
        theme::apply_style(&ctx);
        for desk in [Desk::Home, Desk::Pulse] {
            app.navigate(desk);
            for size in [[900.0, 600.0], [1440.0, 900.0]] {
                run(&ctx, &mut app, size).textures_delta.clear();
            }
        }
        app.pulse_busy = true;
        app.pulse_tx.send(Err("Search unavailable".into())).unwrap();
        run(&ctx, &mut app, [900.0, 600.0]).textures_delta.clear();
        assert!(!app.pulse_busy);
        assert_eq!(app.pulse_error, "Search unavailable");
        assert_eq!(app.pulse.as_ref().unwrap().requested_unix, now);
        assert_eq!(app.pulse.as_ref().unwrap().report.topics.len(), 1);
        app.pulse_sector = Some(Sector::Nuclear);
        run(&ctx, &mut app, [900.0, 600.0]).textures_delta.clear();
    }

    #[test]
    fn detail_panel_and_dialogs_render() {
        let ctx = egui::Context::default();
        theme::apply_style(&ctx);
        let mut app = fixture();
        for (desk, id) in [
            (Desk::Dollar, "ust10"),
            (Desk::Ai, "ai:nvidia"),
            (Desk::Ai, "aim:capex-race"),
            (Desk::Nuclear, "nucs:restart-uprate"),
            (Desk::Nuclear, "nuc:cameco"),
        ] {
            app.navigate(desk);
            app.selected = Some(id.into());
            for size in [[1440.0, 900.0], [1000.0, 700.0]] {
                run(&ctx, &mut app, size).textures_delta.clear();
            }
        }
        for flag in 0..4 {
            app.show_search = flag == 0;
            app.show_sources = flag == 1;
            app.show_settings = flag == 2;
            app.show_alerts = flag == 3;
            run(&ctx, &mut app, [1440.0, 900.0]).textures_delta.clear();
        }
    }

    #[test]
    fn partial_tick_cannot_trigger_an_alert_on_a_cached_instrument() {
        let mut app = fixture();
        app.prefs
            .alerts
            .push(dollar_dashboard::workspace::PriceAlert {
                symbol: "BTC".into(),
                direction: Direction::Above,
                threshold: 90_000.0,
                triggered_at: None,
            });
        app.tick_unix = 123;
        app.check_alerts(&[("ETHUSDT".into(), 3000.0, 1.0, 3100.0, 2900.0)]);
        assert_eq!(app.prefs.alerts[0].triggered_at, None);
        app.check_alerts(&[("BTCUSDT".into(), 100_000.0, 1.0, 110_000.0, 90_000.0)]);
        assert_eq!(app.prefs.alerts[0].triggered_at, Some(123));
        assert!(app.show_alerts);
        assert!(app.notice.contains("at or above $90,000"));
    }

    #[test]
    fn palette_enter_opens_the_first_match() {
        let ctx = egui::Context::default();
        theme::apply_style(&ctx);
        let mut app = fixture();
        app.show_search = true;
        app.search = "cameco".into();
        run(&ctx, &mut app, [1440.0, 900.0]).textures_delta.clear();
        let enter = egui::Event::Key {
            key: Key::Enter,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        };
        ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    [1440.0, 900.0].into(),
                )),
                events: vec![enter],
                ..Default::default()
            },
            |ui| app.render(ui),
        )
        .textures_delta
        .clear();
        assert!(!app.show_search);
        assert_eq!(app.desk, Desk::Nuclear);
        assert_eq!(app.selected.as_deref(), Some("nuc:cameco"));
    }

    #[test]
    fn switching_desks_clears_previous_filter_and_selection() {
        let mut app = fixture();
        app.filter = "NVDA".into();
        app.selected = Some("ai:nvidia".into());
        app.navigate(Desk::Nuclear);
        assert!(app.filter.is_empty());
        assert!(app.selected.is_none());
        assert_eq!(Desk::Zcash.key(), 7);
        assert_eq!(Desk::SpaceX.key(), 8);
        assert_eq!(Desk::Pulse.key(), 9);
    }

    #[test]
    fn background_refresh_does_not_undo_a_newer_tick_or_change_visit_baseline() {
        let mut app = fixture();
        app.visit_baseline = Some(VisitBaseline::from_snapshot(&app.snap, 1_700_000_000));
        let background = app.snap.clone();
        let batch = dollar_dashboard::parse::TickerBatch {
            rows: vec![("BTCUSDT".into(), 100_100.0, 3.2, 101_000.0, 99_000.0)],
            acquired_unix: 1_700_000_101,
            observed_unix: [("BTC".into(), 1_700_000_100)].into(),
        };
        fetch::apply_ticker_batch(&mut app.snap, &batch);
        app.accept_refresh(background);
        assert_eq!(app.snap.btc.price, Some(100_100.0));
        assert_eq!(app.snap.btc.change_24h_pct, Some(3.2));
        assert_eq!(app.snap.btc.high_24h, Some(101_000.0));
        assert_eq!(app.snap.btc.low_24h, Some(99_000.0));
        assert_eq!(app.snap.price_observed_unix("BTC"), Some(1_700_000_100));
        assert_eq!(
            app.visit_baseline.as_ref().unwrap().captured_unix,
            1_700_000_000
        );
    }

    #[test]
    fn a_panicking_refresh_keeps_the_data_and_the_worker() {
        let prev = fixture().snap;
        let kept = guarded_refresh(&prev, |_| {
            let token = "-";
            let _ = &token[..3];
            unreachable!()
        });
        assert_eq!(kept.btc.price, prev.btc.price);
        assert_eq!(kept.fetched_unix, prev.fetched_unix);
        assert!(kept.notes[0].contains("previous data kept"));
        let ok = guarded_refresh(&prev, |p| {
            let mut next = p.clone();
            next.fetched_unix += 1;
            next
        });
        assert_eq!(ok.fetched_unix, prev.fetched_unix + 1);
    }

    #[test]
    fn spot_poll_slows_in_the_background_and_backs_off_on_failure() {
        assert_eq!(tick_delay(0, true, None), Duration::from_secs(2));
        assert_eq!(tick_delay(0, false, None), Duration::from_secs(10));
        assert_eq!(tick_delay(1, true, None), Duration::from_secs(4));
        assert_eq!(tick_delay(3, true, None), Duration::from_secs(16));
        assert_eq!(tick_delay(40, true, None), Duration::from_secs(120));
        // A rate limit names its own wait; never poll sooner than that.
        assert_eq!(tick_delay(1, true, Some(60)), Duration::from_secs(60));
        assert_eq!(tick_delay(1, true, Some(1)), Duration::from_secs(4));
        assert_eq!(tick_delay(1, true, Some(86_400)), Duration::from_secs(900));
    }

    #[test]
    fn a_long_absence_starts_a_new_visit_and_a_short_one_does_not() {
        let mut app = fixture();
        app.visit_baseline = Some(VisitBaseline::from_snapshot(&app.snap, 1_700_000_000));
        let start = Instant::now();
        app.track_visit(true, start);
        app.track_visit(false, start + Duration::from_secs(60));
        assert!(!app.focused.load(Ordering::Relaxed));
        app.track_visit(false, start + Duration::from_secs(600));
        app.track_visit(true, start + Duration::from_secs(1800));
        assert_eq!(
            app.visit_baseline.as_ref().unwrap().captured_unix,
            1_700_000_000
        );
        app.track_visit(false, start + Duration::from_secs(2000));
        app.track_visit(true, start + Duration::from_secs(2000) + VISIT_GAP);
        assert!(app.visit_baseline.as_ref().unwrap().captured_unix > 1_700_000_000);
        assert!(app.away.is_none());
    }

    #[test]
    fn a_rule_crossing_notifies_once_until_the_cooldown_passes() {
        let obs = |date: &str, value: f64| {
            Some(dollar_dashboard::model::Obs {
                date: date.into(),
                value,
                source: "U.S. Treasury".into(),
            })
        };
        let now = chrono::NaiveDate::from_ymd_opt(2026, 11, 5)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap()
            .and_utc()
            .timestamp();
        let mut app = fixture();
        app.snap.dollar.ust30 = obs("2026-11-04", 4.9);
        let mut incoming = app.snap.clone();
        incoming.dollar.ust30 = obs("2026-11-05", 5.1);
        assert_eq!(app.announce_rule_changes(&incoming, now), 1);
        assert!(app.notice.contains("Elevated → Stressed"));
        assert_eq!(app.announce_rule_changes(&incoming, now + 600), 0);
        assert_eq!(
            app.announce_rule_changes(&incoming, now + RULE_ALERT_COOLDOWN),
            1
        );
    }
}

#[cfg(test)]
mod previews {
    use super::*;
    use egui::epaint::{ImageData, Primitive, TextureId};
    use egui::Rect;
    use serde_json::json;
    use std::collections::HashMap;

    /// Export actual egui meshes for visual QA without waking or unlocking the desktop.
    #[test]
    #[ignore = "writes development-only rendering artifacts to /tmp"]
    fn export_ui_previews() {
        let dir_buf =
            std::env::var("MM_QA_DIR").unwrap_or_else(|_| "/tmp/market-monitor-qa".into());
        let dir = std::path::Path::new(&dir_buf);
        std::fs::create_dir_all(dir).unwrap();
        let mut app = super::tests::fixture();
        app.codex.open = std::env::var_os("MM_QA_CODEX").is_some();
        if app.codex.open {
            app.codex.history = serde_json::from_value(json!({"thread":null,"messages":[
                {"role":"You","text":"What does this view tell me?"},
                {"role":"Codex","text":"The overview compares market moves across assets. Check each reading’s source and observation time before comparing them. A falling price shows a move; it does not by itself establish the cause.\n\nWhich reading would you like to explore?"}
            ]})).unwrap();
        }
        if let Some(snapshot) = cache::load() {
            app.snap = snapshot;
        }
        app.pulse = dollar_dashboard::pulse::load();
        for size in [[1440.0, 1000.0], [900.0, 600.0], [1440.0, 4000.0]] {
            for (index, desk) in Desk::all().into_iter().enumerate() {
                if std::env::var("MM_QA_DESK").ok().is_some_and(|name| name != desk.title()) {
                    continue;
                }
                if size[1] > 1000.0
                    && !matches!(desk, Desk::Btc | Desk::Dollar | Desk::Zcash | Desk::SpaceX)
                {
                    continue;
                }
                let ctx = egui::Context::default();
                theme::apply_style(&ctx);
                app.navigate(desk);
                let mut atlas: HashMap<TextureId, egui::ColorImage> = HashMap::new();
                let passes = if app.codex.open { 20 } else { 3 };
                for pass in 0..passes {
                    let mut output = ctx.run_ui(
                        egui::RawInput {
                            screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, size.into())),
                            ..Default::default()
                        },
                        |ui| app.render(ui),
                    );
                    for (id, delta) in output
                        .textures_delta
                        .set
                        .iter()
                        .flat_map(|(id, deltas)| deltas.iter().map(move |delta| (id, delta)))
                    {
                        let ImageData::Color(image) = &delta.image;
                        if let Some([x, y]) = delta.pos {
                            let dest = atlas.get_mut(id).unwrap();
                            for row in 0..image.size[1] {
                                let start = (y + row) * dest.size[0] + x;
                                dest.pixels[start..start + image.size[0]].copy_from_slice(
                                    &image.pixels[row * image.size[0]..(row + 1) * image.size[0]],
                                );
                            }
                        } else {
                            atlas.insert(*id, (**image).clone());
                        }
                    }
                    output.textures_delta.clear();
                    if pass != passes - 1 {
                        continue;
                    }
                    let name = if size[1] > 1000.0 {
                        format!("desk-{index}-full")
                    } else {
                        format!("desk-{index}-{}", size[0] as u32)
                    };
                    let mut textures = Vec::new();
                    for (id, image) in &atlas {
                        let file = format!("{name}-{id:?}.rgba");
                        let bytes: Vec<_> =
                            image.pixels.iter().flat_map(|p| p.to_array()).collect();
                        std::fs::write(dir.join(&file), bytes).unwrap();
                        textures
                            .push(json!({"id":format!("{id:?}"),"file":file,"size":image.size}));
                    }
                    let meshes: Vec<_> = ctx.tessellate(output.shapes,output.pixels_per_point).into_iter().filter_map(|p| {
                        if let Primitive::Mesh(mesh) = p.primitive { Some(json!({
                            "clip":[p.clip_rect.min.x,p.clip_rect.min.y,p.clip_rect.max.x,p.clip_rect.max.y],
                            "texture":format!("{:?}",mesh.texture_id),"indices":mesh.indices,
                            "vertices":mesh.vertices.iter().map(|v|[v.pos.x,v.pos.y,v.uv.x,v.uv.y,
                                v.color.r() as f32,v.color.g() as f32,v.color.b() as f32,v.color.a() as f32]).collect::<Vec<_>>()
                        })) } else { None }
                    }).collect();
                    std::fs::write(
                        dir.join(format!("{name}.json")),
                        serde_json::to_vec(
                            &json!({"size":size,"textures":textures,"meshes":meshes}),
                        )
                        .unwrap(),
                    )
                    .unwrap();
                }
            }
        }
    }
}
