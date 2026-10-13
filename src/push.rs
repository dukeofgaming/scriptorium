//! Copy the manuscript onto each remote exemplar.

use std::fs;
use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::document::{self, ClassifiedLink, DocumentError};
use crate::drift::{self, Gate};
use crate::remote::{RemoteError, RemotePort};
use crate::sync_state::{self, CopyRecord, Ledger, SyncStateError};

/// What push did with one link.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PushReport {
    /// The remote was updated.
    Pushed { url: String },
    /// Ledger, manuscript, and remote already agreed.
    InSync { url: String },
}

/// Push failed before or while writing a remote.
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
    #[error("refusing to overwrite {url} (pass --force)")]
    Conflict { url: String },
}

/// Write the manuscript body to every sync link whose gate allows it.
///
/// The whole push is refused, and nothing is written, when any link would
/// overwrite a remote that moved. Links that already match are left untouched.
pub fn push(
    path: &Path,
    remote: &dyn RemotePort,
    force: bool,
) -> Result<Vec<PushReport>, PushError> {
    let text = fs::read_to_string(path).map_err(|source| PushError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    let manuscript = document::parse(&text)?;
    let ledger_path = sync_state::path_for(path);
    let mut ledger = Ledger::load(&ledger_path)?;
    let local_hash = sync_state::content_sha256(&manuscript.body);

    let mut planned = Vec::with_capacity(manuscript.links.len());
    for link in &manuscript.links {
        let current = match remote.fetch(link) {
            Ok(copy) => Some(copy),
            Err(RemoteError::NotFound(_)) => None,
            Err(err) => return Err(err.into()),
        };
        let (remote_revision, remote_body) = match current.as_ref() {
            Some(copy) => (copy.revision.clone(), copy.body.clone()),
            None => (String::new(), String::new()),
        };
        let state = drift::drift(&local_hash, &remote_revision, ledger.copies.get(&link.url));
        let gate = drift::gate_push(
            state,
            force,
            sync_state::body_is_blank(&remote_body),
            sync_state::content_sha256(&remote_body) == local_hash,
        );
        planned.push(Planned { link, gate });
    }

    if let Some(refused) = planned.iter().find(|item| item.gate == Gate::Refuse) {
        return Err(PushError::Conflict {
            url: refused.link.url.clone(),
        });
    }

    let mut reports = Vec::with_capacity(planned.len());
    for item in &planned {
        if item.gate == Gate::Already {
            reports.push(PushReport::InSync {
                url: item.link.url.clone(),
            });
            continue;
        }
        let updated = remote.push(item.link, &manuscript.body)?;
        ledger.copies.insert(
            item.link.url.clone(),
            CopyRecord {
                revision: updated.revision,
                content_sha256: local_hash.clone(),
            },
        );
        reports.push(PushReport::Pushed {
            url: item.link.url.clone(),
        });
    }
    if reports
        .iter()
        .any(|report| matches!(report, PushReport::Pushed { .. }))
    {
        ledger.save(&ledger_path)?;
    }
    Ok(reports)
}

struct Planned<'a> {
    link: &'a ClassifiedLink,
    gate: Gate,
}
