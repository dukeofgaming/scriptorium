//! File-backed remote used by end-to-end features.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::{RemoteCopy, RemoteError, RemotePort};
use crate::document::ClassifiedLink;
use crate::sync_state::normalize_body;

/// JSON map of URL to copy. Missing file means an empty scriptorium.
#[derive(Debug, Clone)]
pub struct FakeRemote {
    path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct Store {
    #[serde(default)]
    copies: BTreeMap<String, StoredCopy>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct StoredCopy {
    revision: String,
    body: String,
}

impl FakeRemote {
    /// Remember copies in `path`. The file is created on the first write.
    pub fn open(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Seed or replace one copy. Used by tests and the cucumber steps.
    pub fn insert(&self, url: &str, revision: &str, body: &str) -> Result<(), RemoteError> {
        let mut store = self.load()?;
        store.copies.insert(
            url.to_string(),
            StoredCopy {
                revision: revision.to_string(),
                body: normalize_body(body),
            },
        );
        self.save(&store)
    }

    /// Read one copy back, for assertions.
    pub fn get(&self, url: &str) -> Result<Option<RemoteCopy>, RemoteError> {
        Ok(self.load()?.copies.get(url).map(|copy| RemoteCopy {
            revision: copy.revision.clone(),
            body: copy.body.clone(),
        }))
    }

    fn load(&self) -> Result<Store, RemoteError> {
        if !self.path.exists() {
            return Ok(Store::default());
        }
        let text = fs::read_to_string(&self.path).map_err(|err| {
            RemoteError::Message(format!(
                "could not read fake remote {}: {err}",
                self.path.display()
            ))
        })?;
        serde_json::from_str(&text).map_err(|err| {
            RemoteError::Message(format!(
                "fake remote {} is not valid JSON: {err}",
                self.path.display()
            ))
        })
    }

    fn save(&self, store: &Store) -> Result<(), RemoteError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|err| {
                RemoteError::Message(format!("could not create {}: {err}", parent.display()))
            })?;
        }
        let text = serde_json::to_string_pretty(store)
            .map_err(|err| RemoteError::Message(format!("could not encode fake remote: {err}")))?;
        fs::write(&self.path, format!("{text}\n")).map_err(|err| {
            RemoteError::Message(format!(
                "could not write fake remote {}: {err}",
                self.path.display()
            ))
        })
    }
}

impl RemotePort for FakeRemote {
    fn fetch(&self, link: &ClassifiedLink) -> Result<RemoteCopy, RemoteError> {
        self.get(&link.url)?
            .ok_or_else(|| RemoteError::NotFound(link.url.clone()))
    }

    fn push(&self, link: &ClassifiedLink, body: &str) -> Result<RemoteCopy, RemoteError> {
        let mut store = self.load()?;
        let revision = match store.copies.get(&link.url) {
            Some(existing) => next_revision(&existing.revision),
            None => "1".to_string(),
        };
        let copy = StoredCopy {
            revision,
            body: normalize_body(body),
        };
        store.copies.insert(link.url.clone(), copy.clone());
        self.save(&store)?;
        Ok(RemoteCopy {
            revision: copy.revision,
            body: copy.body,
        })
    }
}

fn next_revision(current: &str) -> String {
    match current.parse::<u64>() {
        Ok(number) => (number + 1).to_string(),
        Err(_) => format!("{current}+1"),
    }
}

impl FakeRemote {
    /// Path passed through `SCRIPTORIUM_FAKE_REMOTE`.
    pub fn path(&self) -> &Path {
        &self.path
    }
}
