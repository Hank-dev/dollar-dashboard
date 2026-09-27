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
    let snap: Snapshot = serde_json::from_str(&text).ok()?;
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
