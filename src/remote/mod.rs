//! Remote copies. Use cases depend on [`RemotePort`], not on acli or HTTP.

mod fake;
mod jira;

use crate::document::ClassifiedLink;

pub use fake::FakeRemote;
pub use jira::JiraAcli;

/// A remote body and the revision that identifies that version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteCopy {
    /// Provider revision. The fake store uses a decimal counter.
    pub revision: String,
    /// Body text, not yet normalized.
    pub body: String,
}

/// A remote copy could not be read or written.
#[derive(Debug, thiserror::Error)]
pub enum RemoteError {
    #[error("no remote copy for {0}")]
    NotFound(String),
    #[error("{0}")]
    Message(String),
}

/// The port use cases call. Adapters and the file-backed fake implement it.
pub trait RemotePort: Send + Sync {
    /// Read the current remote copy.
    fn fetch(&self, link: &ClassifiedLink) -> Result<RemoteCopy, RemoteError>;
    /// Write `body` and return the copy as it is after the write.
    fn push(&self, link: &ClassifiedLink, body: &str) -> Result<RemoteCopy, RemoteError>;
}

/// The fake store when `SCRIPTORIUM_FAKE_REMOTE` is set, otherwise live adapters.
pub fn from_env() -> Result<Box<dyn RemotePort>, RemoteError> {
    if let Ok(path) = std::env::var("SCRIPTORIUM_FAKE_REMOTE") {
        if !path.is_empty() {
            return Ok(Box::new(FakeRemote::open(path)));
        }
    }
    Ok(Box::new(RoutingRemote {
        jira: JiraAcli::from_env(),
    }))
}

struct RoutingRemote {
    jira: JiraAcli<jira::SystemCommand>,
}

impl RemotePort for RoutingRemote {
    fn fetch(&self, link: &ClassifiedLink) -> Result<RemoteCopy, RemoteError> {
        match &link.kind {
            crate::document::LinkKind::Jira { .. } => self.jira.fetch(link),
            other => Err(RemoteError::Message(format!(
                "{} cannot be copied yet",
                other.summary()
            ))),
        }
    }

    fn push(&self, link: &ClassifiedLink, body: &str) -> Result<RemoteCopy, RemoteError> {
        match &link.kind {
            crate::document::LinkKind::Jira { .. } => self.jira.push(link, body),
            other => Err(RemoteError::Message(format!(
                "{} cannot be copied yet",
                other.summary()
            ))),
        }
    }
}
