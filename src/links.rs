//! List the copies a manuscript names.

use std::fs;
use std::path::Path;

use thiserror::Error;

use crate::document::{self, DocumentError};

/// Read `path` and render one classified link per line.
pub fn list(path: &Path) -> Result<String, ListError> {
    let text = fs::read_to_string(path).map_err(|source| ListError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    let manuscript = document::parse(&text)?;
    let mut out = String::new();
    for link in &manuscript.links {
        out.push_str(&link.kind.summary());
        out.push(' ');
        out.push_str(&link.url);
        out.push('\n');
    }
    Ok(out)
}

/// A failure while listing sync links.
#[derive(Debug, Error)]
pub enum ListError {
    #[error("could not read {path}: {source}")]
    Read {
        path: std::path::PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error(transparent)]
    Document(#[from] DocumentError),
}
