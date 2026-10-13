//! Copy a remote exemplar onto the manuscript.

use std::fs;
use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::document::{self, DocumentError, LinkKind};
use crate::remote::{RemoteError, RemotePort};
use crate::sync_state::{self, Ledger, SyncStateError};

/// The link that was copied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PullReport {
    /// Sync URL that was pulled.
    pub url: String,
}

/// Pull failed before or while writing the manuscript.
#[derive(Debug, Error)]
pub enum PullError {
    #[error("could not read {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("could not write {path}: {source}")]
    Write {
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
    #[error("pull copies one link at a time; {path} names {count}")]
    TooMany { path: PathBuf, count: usize },
    #[error("{kind} cannot be pulled yet")]
    Unsupported { kind: String },
}

/// Fetch the single sync link and write its body into `path`.
///
/// The front matter is kept byte for byte. The sidecar ledger records the
/// remote revision and a hash of the body. Conflict refusal arrives with
/// `check`; this copy overwrites the body.
pub fn pull(path: &Path, remote: &dyn RemotePort) -> Result<PullReport, PullError> {
    let text = fs::read_to_string(path).map_err(|source| PullError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    let manuscript = document::parse(&text)?;
    let link = match manuscript.links.as_slice() {
        [link] => link,
        other => {
            return Err(PullError::TooMany {
                path: path.to_path_buf(),
                count: other.len(),
            });
        }
    };
    if !matches!(link.kind, LinkKind::Jira { .. }) {
        return Err(PullError::Unsupported {
            kind: link.kind.summary(),
        });
    }
    let copy = remote.fetch(link)?;
    let rendered = sync_state::render(&manuscript.preamble, &copy.body);
    fs::write(path, &rendered).map_err(|source| PullError::Write {
        path: path.to_path_buf(),
        source,
    })?;
    let ledger_path = sync_state::path_for(path);
    let mut ledger = Ledger::load(&ledger_path)?;
    ledger.copies.insert(
        link.url.clone(),
        crate::sync_state::CopyRecord {
            revision: copy.revision,
            content_sha256: sync_state::content_sha256(&copy.body),
        },
    );
    ledger.save(&ledger_path)?;
    Ok(PullReport {
        url: link.url.clone(),
    })
}
