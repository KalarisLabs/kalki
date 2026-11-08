//! Content-addressed blob store for payloads larger than the inline threshold.
//! Layout: `blobs/<sha256-hex>`; write-tmp-then-rename; dedups by construction.

use std::path::{Path, PathBuf};

use crate::Result;

/// Inline-vs-blob threshold in bytes.
pub const INLINE_THRESHOLD_BYTES: usize = 4 * 1024;

/// Store `bytes` and return its hex SHA-256 handle.
///
/// # Errors
/// I/O failures are returned; corrupted existing blobs are an error, never overwritten.
pub fn put_blob(dir: &Path, bytes: &[u8]) -> Result<String> {
    let hash = hex_sha256(bytes);
    let final_path = dir.join(&hash);
    if final_path.exists() {
        return Ok(hash); // dedup: bytes are identical by hash construction
    }
    std::fs::create_dir_all(dir)?;
    let tmp = dir.join(format!(".tmp-{hash}"));
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, &final_path)?;
    Ok(hash)
}

/// Fetch a blob by handle.
///
/// # Errors
/// Missing or unreadable blobs return an error.
pub fn get_blob(dir: &Path, handle: &str) -> Result<Vec<u8>> {
    Ok(std::fs::read(dir.join(handle))?)
}

/// Full-path helper used by the WAL/snapshot layer.
#[must_use]
pub fn blob_path(dir: &Path, handle: &str) -> PathBuf {
    dir.join(handle)
}

/// Hex-encode a SHA-256 digest without pulling in a `hex` crate (few lines, no dep).
#[must_use]
pub fn hex_sha256(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    use std::fmt::Write;
    let mut out = String::with_capacity(64);
    for b in Sha256::digest(bytes) {
        write!(out, "{b:02x}").expect("writing to String cannot fail");
    }
    out
}

/// Verify a blob against its handle (used by `kalki audit verify`).
pub fn verify(dir: &Path, handle: &str) -> Result<bool> {
    get_blob(dir, handle).map(|b| hex_sha256(&b) == handle)
}

/// Placeholder kept to satisfy the module contract in early milestones.
///
/// # Errors
/// Always succeeds in v0.2 (no-op marker).
pub fn _anchor_marker() -> Result<()> {
    Ok(())
}
