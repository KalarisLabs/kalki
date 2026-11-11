//! Snapshot checkpoint index (redb-style single file, pre-WAL anchor).
//! v0.2 ships the anchor format only; the redb file lands with the segment-rotation milestone.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::Result;

/// A checkpoint anchor: the hash of the snapshot state at a given event sequence.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Anchor {
    /// Event sequence covered by this anchor.
    pub seq: u64,
    /// Hex SHA-256 of the snapshot's canonical bytes.
    pub anchor_hash: String,
    /// Wall-clock millis when the anchor was written.
    pub ts_ms: u64,
}

/// Sidecar JSON file next to the redb store until the redb integration lands.
///
/// # Errors
/// Serialization or I/O failures.
pub fn write_anchor(dir: &Path, anchor: &Anchor) -> Result<PathBuf> {
    std::fs::create_dir_all(dir)?;
    let path = dir.join(format!("anchor-{:020}.json", anchor.seq));
    let tmp = dir.join(format!(".anchor-{:020}.tmp", anchor.seq));
    std::fs::write(&tmp, serde_json::to_vec_pretty(anchor)?)?;
    std::fs::rename(&tmp, &path)?;
    Ok(path)
}

/// Load the newest anchor present in `dir`, if any.
///
/// # Errors
/// I/O or JSON failures on a present file.
pub fn read_latest_anchor(dir: &Path) -> Result<Option<Anchor>> {
    let mut best: Option<Anchor> = None;
    if !dir.exists() {
        return Ok(None);
    }
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.starts_with("anchor-"))
        {
            let data = std::fs::read(&path)?;
            let anchor: Anchor = serde_json::from_slice(&data)?;
            if best.as_ref().is_none_or(|b| b.seq < anchor.seq) {
                best = Some(anchor);
            }
        }
    }
    Ok(best)
}
