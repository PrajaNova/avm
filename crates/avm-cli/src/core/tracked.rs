//! Project directories whose config pins tools, so `avm prune` keeps their versions.
use super::{shims, store};
use anyhow::Result;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// Location of the tracked-configs store (`~/.avm/tracked-configs.json`).
pub fn path() -> Result<PathBuf> {
    Ok(shims::avm_home()?.join("tracked-configs.json"))
}

/// The tracked project directories; errors if the store is unreadable so pruning never proceeds on a guess.
pub fn dirs() -> Result<BTreeSet<String>> {
    store::read_or_default(&path()?, "tracked-configs.json; cannot safely prune")
}

/// Forget everything but `live`. The caller holds the store lock.
pub fn replace(live: &BTreeSet<String>) -> Result<()> {
    store::write_pretty(&path()?, live)
}

/// Remember `dir`. Called on every resolution, so it only writes when the dir is new.
pub fn track(dir: &Path) {
    let key = dir.to_string_lossy().into_owned();
    let result = (|| -> Result<()> {
        // Hot path (every prompt and shim): skip the exclusive lock once the dir is known.
        if dirs().is_ok_and(|known| known.contains(&key)) {
            return Ok(());
        }
        let _guard = avm_plugin_api::storage::lock(&path()?)?;
        let mut known = dirs()?;
        if known.insert(key) {
            replace(&known)?;
        }
        Ok(())
    })();
    if let Err(err) = result {
        warn!("could not track project for prune: {err}");
    }
}
