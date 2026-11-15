//! One line of readings per UTC day, so the rules and scores can later be
//! checked against what happened. Append-only JSONL; nothing reads it back
//! except the duplicate-day check.

use crate::model::{self, Snapshot};
use serde_json::{json, Value};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

pub fn journal_path() -> PathBuf {
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")))
        .unwrap_or_else(|| PathBuf::from(".local/share"));
    base.join("dollar-dashboard").join("readings.jsonl")
}

fn day(unix: i64) -> String {
    chrono::DateTime::from_timestamp(unix, 0)
        .map(|t| t.format("%Y-%m-%d").to_string())
        .unwrap_or_default()
}

/// The day's row. Unavailable readings stay null rather than becoming zero.
pub fn row(snapshot: &Snapshot, now: i64) -> Value {
    let obs = |o: &Option<model::Obs>| o.as_ref().map(|o| json!({ "v": o.value, "date": o.date }));
    let t = &snapshot.terminal;
    let d = &snapshot.dollar;
    let rules: serde_json::Map<String, Value> = model::dollar_metrics(snapshot)
        .into_iter()
        .map(|m| {
            (
                m.id.to_string(),
                json!({ "status": m.status.label(), "value": m.value, "live": m.live }),
            )
        })
        .collect();
    json!({
        "date": day(now),
        "unix": now,
        "refreshed_unix": snapshot.fetched_unix,
        "btc": snapshot.btc.price,
        "eth": snapshot.btc.eth_price,
        "zec": snapshot.btc.zec_price,
        "fng": snapshot.btc.fng,
        "regime_score": t.regime_score,
        "regime_alloc": t.regime_alloc,
        "net_liq_b": t.net_liq_b,
        "mvrv_z": t.mvrv_z,
        "nupl": t.nupl,
        "mayer": t.mayer,
        "funding_ann": t.funding_ann,
        "etf_7d_m": t.etf_7d_m,
        "mei": t.mei_state,
        "ust2": obs(&d.ust2),
        "ust10": obs(&d.ust10),
        "ust30": obs(&d.ust30),
        "vix": obs(&d.vix),
        "dxy": obs(&d.dxy),
        "usdjpy": obs(&d.usdjpy),
        "spx": obs(&d.spx),
        "gold": obs(&d.gold),
        "brent": obs(&d.brent),
        "rules": rules,
        "issues": snapshot.notes.len(),
    })
}

/// Append today's row unless the file already ends with one for today, or the
/// snapshot was not refreshed today. Returns whether a row was written.
pub fn append_daily(path: &Path, snapshot: &Snapshot, now: i64) -> Result<bool, String> {
    let today = day(now);
    if today.is_empty() || day(snapshot.fetched_unix) != today {
        return Ok(false);
    }
    let last_day = fs::read_to_string(path)
        .ok()
        .and_then(|text| {
            text.lines()
                .rev()
                .find(|line| !line.trim().is_empty())
                .and_then(|line| serde_json::from_str::<Value>(line).ok())
        })
        .and_then(|v| v["date"].as_str().map(str::to_owned));
    if last_day.as_deref() == Some(today.as_str()) {
        return Ok(false);
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| e.to_string())?;
    writeln!(file, "{}", row(snapshot, now)).map_err(|e| e.to_string())?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_row_per_day_and_only_after_a_refresh_that_day() {
        let dir = std::env::temp_dir().join(format!("mm-journal-{}", std::process::id()));
        let path = dir.join("readings.jsonl");
        let _ = fs::remove_file(&path);
        let now = 1_794_400_000;
        let mut snap = model::fallback_snapshot();
        snap.fetched_unix = now - 3 * 86_400;
        assert_eq!(append_daily(&path, &snap, now), Ok(false));
        snap.fetched_unix = now - 60;
        snap.btc.price = Some(100_000.0);
        assert_eq!(append_daily(&path, &snap, now), Ok(true));
        assert_eq!(append_daily(&path, &snap, now + 600), Ok(false));
        snap.fetched_unix = now + 86_400;
        assert_eq!(append_daily(&path, &snap, now + 86_400), Ok(true));
        let text = fs::read_to_string(&path).unwrap();
        let rows: Vec<Value> = text
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0]["btc"], 100_000.0);
        assert!(rows[0]["regime_score"].is_null());
        assert!(rows[0]["rules"].is_object());
        assert_ne!(rows[0]["date"], rows[1]["date"]);
        let _ = fs::remove_file(&path);
        let _ = fs::remove_dir(&dir);
    }
}
