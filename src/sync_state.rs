//! The sidecar that remembers the last copy known to match.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

/// Ledger written beside a manuscript (`colophon.md` → `colophon.scriptorium.json`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Ledger {
    /// One record per sync URL.
    #[serde(default)]
    pub copies: BTreeMap<String, CopyRecord>,
}

/// The remote revision and body hash at the last successful copy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CopyRecord {
    /// Provider revision (`updated` timestamp, page version, or document revision).
    pub revision: String,
    /// SHA-256 of the normalized manuscript body.
    pub content_sha256: String,
}

/// A ledger could not be read or written.
#[derive(Debug, Error)]
pub enum SyncStateError {
    #[error("could not read ledger {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("could not write ledger {path}: {source}")]
    Write {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("ledger {path} is not valid JSON: {source}")]
    Format {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
}

impl Ledger {
    /// Load the ledger, or an empty one when the file is not there yet.
    pub fn load(path: &Path) -> Result<Self, SyncStateError> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = fs::read_to_string(path).map_err(|source| SyncStateError::Read {
            path: path.to_path_buf(),
            source,
        })?;
        serde_json::from_str(&text).map_err(|source| SyncStateError::Format {
            path: path.to_path_buf(),
            source,
        })
    }

    /// Write the ledger as pretty JSON.
    pub fn save(&self, path: &Path) -> Result<(), SyncStateError> {
        let text = serde_json::to_string_pretty(self).map_err(|source| SyncStateError::Format {
            path: path.to_path_buf(),
            source,
        })?;
        fs::write(path, format!("{text}\n")).map_err(|source| SyncStateError::Write {
            path: path.to_path_buf(),
            source,
        })
    }
}

/// Path of the sidecar for `manuscript`.
pub fn path_for(manuscript: &Path) -> PathBuf {
    let stem = manuscript.file_stem().unwrap_or_default();
    let mut name = stem.to_os_string();
    name.push(".scriptorium.json");
    manuscript.with_file_name(name)
}

/// Normalize a body so hashes and writes do not flap on a missing final newline.
pub fn normalize_body(body: &str) -> String {
    let mut text = body.replace("\r\n", "\n");
    if !text.ends_with('\n') {
        text.push('\n');
    }
    text
}

/// True when the body has no visible text.
pub fn body_is_blank(body: &str) -> bool {
    normalize_body(body).trim().is_empty()
}

/// SHA-256 hex of the normalized body.
pub fn content_sha256(body: &str) -> String {
    let digest = Sha256::digest(normalize_body(body).as_bytes());
    hex_encode(&digest)
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

/// Join the original front matter to a normalized body.
pub fn render(preamble: &str, body: &str) -> String {
    let body = normalize_body(body);
    if preamble.ends_with('\n') {
        format!("{preamble}{body}")
    } else {
        format!("{preamble}\n{body}")
    }
}
