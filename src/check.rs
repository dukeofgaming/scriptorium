//! Compare every sync link with the manuscript and the ledger.

use std::fs;
use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::document::{self, DocumentError};
use crate::drift::{self, Drift};
use crate::remote::{RemoteError, RemotePort};
use crate::sync_state::{self, Ledger, SyncStateError};

/// One line of `check` output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// How this link has drifted.
    pub drift: Drift,
    /// The sync URL.
    pub url: String,
}

/// Check failed before it could report drift.
#[derive(Debug, Error)]
pub enum CheckError {
    #[error("could not read {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error(transparent)]
    Document(#[from] DocumentError),
    #[error(transparent)]
    Remote(#[from] RemoteError),
    #[error(transparent)]
    Ledger(#[from] SyncStateError),
}

/// Fetch each sync link and compare it with the manuscript and the ledger.
pub fn check(path: &Path, remote: &dyn RemotePort) -> Result<Vec<Finding>, CheckError> {
    let text = fs::read_to_string(path).map_err(|source| CheckError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    let manuscript = document::parse(&text)?;
    let ledger = Ledger::load(&sync_state::path_for(path))?;
    let local_hash = sync_state::content_sha256(&manuscript.body);
    let mut findings = Vec::with_capacity(manuscript.links.len());
    for link in &manuscript.links {
        let copy = remote.fetch(link)?;
        let state = drift::drift(&local_hash, &copy.revision, ledger.copies.get(&link.url));
        findings.push(Finding {
            drift: state,
            url: link.url.clone(),
        });
    }
    Ok(findings)
}
