//! Copy the manuscript onto the remote exemplar.

use std::fs;
use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::document::{self, DocumentError, LinkKind};
use crate::remote::{RemoteCopy, RemoteError, RemotePort};
use crate::sync_state::{self, CopyRecord, Ledger, SyncStateError};

/// What push did with the single link.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PushReport {
    /// The remote was updated.
    Pushed { url: String },
    /// Ledger, manuscript, and remote already agreed.
    InSync { url: String },
}

/// Push failed before or while writing the remote.
#[derive(Debug, Error)]
pub enum PushError {
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
    #[error("push copies one link at a time; {path} names {count}")]
    TooMany { path: PathBuf, count: usize },
    #[error("{kind} cannot be pushed yet")]
    Unsupported { kind: String },
}

/// Write the manuscript body to its Jira work item and update the ledger.
///
/// When the ledger, the body, and the remote revision already agree, the
/// remote is left untouched. Refusing a remote that moved is the conflict slice.
pub fn push(path: &Path, remote: &dyn RemotePort) -> Result<PushReport, PushError> {
    let text = fs::read_to_string(path).map_err(|source| PushError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    let manuscript = document::parse(&text)?;
    let link = match manuscript.links.as_slice() {
        [link] => link,
        other => {
            return Err(PushError::TooMany {
                path: path.to_path_buf(),
                count: other.len(),
            });
        }
    };
    if !matches!(link.kind, LinkKind::Jira { .. }) {
        return Err(PushError::Unsupported {
            kind: link.kind.summary(),
        });
    }

    let ledger_path = sync_state::path_for(path);
    let mut ledger = Ledger::load(&ledger_path)?;
    let local_hash = sync_state::content_sha256(&manuscript.body);
    let current = match remote.fetch(link) {
        Ok(copy) => Some(copy),
        Err(RemoteError::NotFound(_)) => None,
        Err(err) => return Err(err.into()),
    };
    if already_in_sync(ledger.copies.get(&link.url), current.as_ref(), &local_hash) {
        return Ok(PushReport::InSync {
            url: link.url.clone(),
        });
    }

    let updated = remote.push(link, &manuscript.body)?;
    ledger.copies.insert(
        link.url.clone(),
        CopyRecord {
            revision: updated.revision,
            content_sha256: local_hash,
        },
    );
    ledger.save(&ledger_path)?;
    Ok(PushReport::Pushed {
        url: link.url.clone(),
    })
}

fn already_in_sync(
    record: Option<&CopyRecord>,
    remote: Option<&RemoteCopy>,
    local_hash: &str,
) -> bool {
    match (record, remote) {
        (Some(record), Some(remote)) => {
            record.revision == remote.revision && record.content_sha256 == local_hash
        }
        _ => false,
    }
}
