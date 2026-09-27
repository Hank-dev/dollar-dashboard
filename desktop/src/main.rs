mod app;
mod draw;
mod pages;

use dollar_dashboard::cache;
use dollar_dashboard::fetch::{self, refresh};
use dollar_dashboard::model::fallback_snapshot;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        None => app::launch(),
        Some("--fetch") => fetch_once(),
        Some("--help") | Some("-h") => {
            println!(
                "Market Monitor\n\n\
                 Usage:\n  \
                 dollar-dashboard\n      \
                 Open the local desktop app\n  \
                 dollar-dashboard --fetch\n      \
                 Refresh public sources once, write the cache, and print a summary\n\n\
                 Keys: 1-5 switch desks, R refreshes, Esc clears the selection."
            );
            ExitCode::SUCCESS
        }
        Some(other) => {
            eprintln!("Unknown option: {other}. Try --help.");
            ExitCode::from(2)
        }
    }
}

fn fetch_once() -> ExitCode {
    let prev = cache::load().unwrap_or_else(fallback_snapshot);
    let snapshot = refresh(&prev);
    if let Err(err) = cache::save(&snapshot) {
        eprintln!("cache: {err}");
    }
    println!("{}", fetch::summary(&snapshot));
    if snapshot.fetched_unix == 0 {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
