//! Small JSON state files: read with a default when missing, write atomically.
use anyhow::{Context, Result};
use fs_err as fs;
use serde::{de::DeserializeOwned, Serialize};
use std::io::ErrorKind;
use std::path::Path;

/// A missing file is `T::default()`; a malformed one is an error naming `what`.
pub fn read_or_default<T: DeserializeOwned + Default>(path: &Path, what: &str) -> Result<T> {
    match fs::read(path) {
        Ok(raw) => serde_json::from_slice(&raw).with_context(|| format!("invalid {what}")),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(T::default()),
        Err(e) => Err(e.into()),
    }
}

/// Serialize `value` as pretty JSON to `path`, atomically.
pub fn write_pretty<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    avm_plugin_sdk::storage::atomic_write(path, &serde_json::to_vec_pretty(value)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn missing_is_default_malformed_is_error_and_round_trips() {
        let dir = std::env::temp_dir().join(format!("avm-store-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("s.json");
        assert!(read_or_default::<BTreeSet<String>>(&path, "s")
            .unwrap()
            .is_empty());
        write_pretty(&path, &BTreeSet::from(["a".to_string()])).unwrap();
        assert_eq!(
            read_or_default::<BTreeSet<String>>(&path, "s")
                .unwrap()
                .len(),
            1
        );
        fs::write(&path, "nope").unwrap();
        assert!(read_or_default::<BTreeSet<String>>(&path, "s").is_err());
        fs::remove_dir_all(&dir).ok();
    }
}
