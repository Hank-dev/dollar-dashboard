mod app;
mod charts;
mod chrome;
mod codex;
mod desks;
mod dialogs;
mod theme;
mod widgets;

use dollar_dashboard::cache;
use dollar_dashboard::fetch::{self, refresh};
use dollar_dashboard::model::fallback_snapshot;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        None => app::launch(),
        Some("--fetch") => fetch_once(),
        Some("--x-pulse") => fetch_pulse(),
        Some("--help") | Some("-h") => {
            println!(
                "Market Monitor\n\n\
                 Usage:\n  \
                 dollar-dashboard\n      \
                 Open the local desktop app\n  \
                 dollar-dashboard --fetch\n      \
                 Refresh public sources once, write the cache, and print a summary\n\n\
                 dollar-dashboard --x-pulse\n      \
                 Search X using XAI_API_KEY (separately billed) and cache the report\n\n\
                 Keys: 1-9 switch desks, Ctrl+K searches, R refreshes prices, Esc closes panels."
            );
            ExitCode::SUCCESS
        }
        Some(other) => {
            eprintln!("Unknown option: {other}. Try --help.");
            ExitCode::from(2)
        }
    }
}

fn fetch_pulse() -> ExitCode {
    match dollar_dashboard::pulse::fetch() {
        Ok(pulse) => {
            if let Err(err) = dollar_dashboard::pulse::save(&pulse) {
                eprintln!("X cache: {err}");
                return ExitCode::FAILURE;
            }
            println!("{}", serde_json::to_string_pretty(&pulse).unwrap());
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("X Pulse: {err}");
            ExitCode::FAILURE
        }
    }
}

fn fetch_once() -> ExitCode {
    let prev = cache::load().unwrap_or_else(fallback_snapshot);
    let snapshot = refresh(&prev);
    if let Err(err) = cache::save(&snapshot) {
        eprintln!("cache: {err}");
    }
    if let Err(err) = dollar_dashboard::journal::append_daily(
        &dollar_dashboard::journal::journal_path(),
        &snapshot,
        dollar_dashboard::model::now_unix(),
    ) {
        eprintln!("readings log: {err}");
    }
    println!("{}", fetch::summary(&snapshot));
    // Success means at least one feed answered in this run, not an older cached refresh.
    if snapshot.fetched_unix > prev.fetched_unix {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
