//! Copy remote exemplars onto the manuscript.

use std::fs;
use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::document::{self, ClassifiedLink, DocumentError};
use crate::drift::{self, Gate};
use crate::remote::{RemoteCopy, RemoteError, RemotePort};
use crate::sync_state::{self, Ledger, SyncStateError};

/// What pull did with one link.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PullReport {
    /// The manuscript body was replaced from this remote.
    Pulled { url: String },
    /// This link already agreed with the manuscript.
    InSync { url: String },
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
    #[error("refusing to overwrite {url} (pass --force)")]
    Conflict { url: String },
    #[error("remote copies disagree; pull would not know which text to keep")]
    Disagree,
}

/// Fetch every sync link and, when they agree, write that body into `path`.
///
/// The front matter is kept byte for byte. A link that has local edits or has
/// diverged is refused unless `force` is set. Remotes that do not contain the
/// same text are refused even with `--force`.
pub fn pull(
    path: &Path,
    remote: &dyn RemotePort,
    force: bool,
) -> Result<Vec<PullReport>, PullError> {
    let text = fs::read_to_string(path).map_err(|source| PullError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    let manuscript = document::parse(&text)?;
    let ledger_path = sync_state::path_for(path);
    let mut ledger = Ledger::load(&ledger_path)?;
    let local_hash = sync_state::content_sha256(&manuscript.body);
    let local_blank = sync_state::body_is_blank(&manuscript.body);

    let mut fetched = Vec::with_capacity(manuscript.links.len());
    for link in &manuscript.links {
        let copy = remote.fetch(link)?;
        let remote_hash = sync_state::content_sha256(&copy.body);
        let state = drift::drift(&local_hash, &copy.revision, ledger.copies.get(&link.url));
        let gate = drift::gate_pull(state, force, local_blank, local_hash == remote_hash);
        fetched.push(Fetched { link, copy, gate });
    }

    if let Some(refused) = fetched.iter().find(|item| item.gate == Gate::Refuse) {
        return Err(PullError::Conflict {
            url: refused.link.url.clone(),
        });
    }
    if bodies_disagree(fetched.iter().map(|item| item.copy.body.as_str())) {
        return Err(PullError::Disagree);
    }
    if fetched.iter().all(|item| item.gate == Gate::Already) {
        return Ok(fetched
            .iter()
            .map(|item| PullReport::InSync {
                url: item.link.url.clone(),
            })
            .collect());
    }

    let body = &fetched[0].copy.body;
    let rendered = sync_state::render(&manuscript.preamble, body);
    fs::write(path, rendered).map_err(|source| PullError::Write {
        path: path.to_path_buf(),
        source,
    })?;
    let new_hash = sync_state::content_sha256(body);
    let mut reports = Vec::with_capacity(fetched.len());
    for item in &fetched {
        ledger.copies.insert(
            item.link.url.clone(),
            crate::sync_state::CopyRecord {
                revision: item.copy.revision.clone(),
                content_sha256: new_hash.clone(),
            },
        );
        reports.push(if item.gate == Gate::Already {
            PullReport::InSync {
                url: item.link.url.clone(),
            }
        } else {
            PullReport::Pulled {
                url: item.link.url.clone(),
            }
        });
    }
    ledger.save(&ledger_path)?;
    Ok(reports)
}

struct Fetched<'a> {
    link: &'a ClassifiedLink,
    copy: RemoteCopy,
    gate: Gate,
}

fn bodies_disagree<'a>(bodies: impl IntoIterator<Item = &'a str>) -> bool {
    let mut hashes = bodies.into_iter().map(sync_state::content_sha256);
    let Some(first) = hashes.next() else {
        return false;
    };
    hashes.any(|hash| hash != first)
}
