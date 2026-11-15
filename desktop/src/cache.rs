use crate::model::{Snapshot, SNAPSHOT_VERSION};
use std::fs;
use std::path::PathBuf;

pub fn cache_path() -> PathBuf {
    let base = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")))
        .unwrap_or_else(|| PathBuf::from(".cache"));
    base.join("dollar-dashboard").join("snapshot.json")
}

pub fn load() -> Option<Snapshot> {
    let text = fs::read_to_string(cache_path()).ok()?;
    let mut snap: Snapshot = serde_json::from_str(&text).ok()?;
    if crate::readouts::invalidate_legacy_liquidity(&mut snap.terminal) {
        snap.feeds.remove("fred-liquidity");
    }
    (snap.version == SNAPSHOT_VERSION).then_some(snap)
}

pub fn save(snapshot: &Snapshot) -> Result<(), String> {
    let path = cache_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    let text = serde_json::to_string(snapshot).map_err(|err| err.to_string())?;
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, text).map_err(|err| err.to_string())?;
    fs::rename(&tmp, &path).map_err(|err| err.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A cache written before dated histories and feed stamps must still load,
    /// or an upgrade would throw away the user's last good snapshot.
    #[test]
    fn caches_from_before_dated_histories_still_load() {
        let snap: Snapshot =
            serde_json::from_str(include_str!("testdata/snapshot-2026-10-09.json")).unwrap();
        assert_eq!(snap.version, SNAPSHOT_VERSION);
        assert!(snap.histories.is_empty() && snap.feeds.is_empty());
        assert!(snap.price_stamps.is_empty());
        assert_eq!(snap.price_as_of("BTC", crate::model::now_unix()), "cached");
        assert!(snap.terminal.net_liq_hist.is_empty());
        assert!(snap.terminal.macro_strip[0].hist.is_empty());
        assert!(snap.zec.btc_ratio_daily.is_empty());
        assert!(snap.btc.price.is_some());
    }
}
