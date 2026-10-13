//! A manuscript: YAML front matter, a `sync` list, and the body that is copied.

use std::sync::LazyLock;

use gray_matter::engine::YAML;
use gray_matter::Matter;
use regex::Regex;
use serde::Deserialize;
use thiserror::Error;

/// A Markdown file split into the header the scribe wrote and the copied body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manuscript {
    /// Original front matter, including the opening and closing `---` lines.
    pub preamble: String,
    /// Text after the front matter, unchanged.
    pub body: String,
    /// Classified `sync` links, in the order they were written.
    pub links: Vec<ClassifiedLink>,
}

/// One remote copy named by `sync`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassifiedLink {
    /// The URL as it was written in the manuscript.
    pub url: String,
    /// What kind of remote the URL points at.
    pub kind: LinkKind,
}

/// A remote Scriptorium knows how to copy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkKind {
    /// A Jira work item, addressed by its key.
    Jira { key: String },
    /// A Confluence page, addressed by its numeric id.
    Confluence { page_id: String },
    /// A Google Doc, addressed by its document id.
    GoogleDoc { document_id: String },
}

impl LinkKind {
    /// Stable single-line label used by `scriptorium links`.
    pub fn summary(&self) -> String {
        match self {
            Self::Jira { key } => format!("jira {key}"),
            Self::Confluence { page_id } => format!("confluence {page_id}"),
            Self::GoogleDoc { document_id } => format!("google-doc {document_id}"),
        }
    }
}

/// Why a manuscript could not be read as a sync list.
#[derive(Debug, Error)]
pub enum DocumentError {
    #[error("manuscript has no YAML front matter")]
    MissingFrontMatter,
    #[error("front matter is not valid YAML: {0}")]
    InvalidYaml(String),
    #[error("front matter is missing `sync`")]
    MissingSync,
    #[error("`sync` must be a link or a list of links")]
    SyncShape,
    #[error("`sync` has no links")]
    EmptySync,
    #[error("unrecognized sync link: {0}")]
    UnrecognizedLink(String),
}

#[derive(Debug, Deserialize)]
struct SyncFront {
    sync: SyncField,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum SyncField {
    One(String),
    Many(Vec<String>),
}

impl SyncField {
    fn links(self) -> Result<Vec<String>, DocumentError> {
        let links = match self {
            Self::One(link) => vec![link],
            Self::Many(links) => links,
        };
        if links.is_empty() || links.iter().all(|link| link.trim().is_empty()) {
            return Err(DocumentError::EmptySync);
        }
        Ok(links)
    }
}

/// Parse a manuscript and classify every `sync` link.
pub fn parse(text: &str) -> Result<Manuscript, DocumentError> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let (preamble, body) = split_preamble(text).ok_or(DocumentError::MissingFrontMatter)?;
    let front = parse_sync(text)?;
    let mut links = Vec::with_capacity(front.len());
    for link in front {
        links.push(classify(&link)?);
    }
    Ok(Manuscript {
        preamble,
        body,
        links,
    })
}

/// Classify a single sync URL.
pub fn classify(url: &str) -> Result<ClassifiedLink, DocumentError> {
    let url = url.trim();
    if url.is_empty() {
        return Err(DocumentError::EmptySync);
    }
    if let Some(document_id) = google_doc_id(url) {
        return Ok(ClassifiedLink {
            url: url.to_string(),
            kind: LinkKind::GoogleDoc { document_id },
        });
    }
    if url.contains("/wiki/") {
        let Some(page_id) = confluence_page_id(url) else {
            return Err(DocumentError::UnrecognizedLink(url.to_string()));
        };
        return Ok(ClassifiedLink {
            url: url.to_string(),
            kind: LinkKind::Confluence { page_id },
        });
    }
    if let Some(key) = jira_key(url) {
        return Ok(ClassifiedLink {
            url: url.to_string(),
            kind: LinkKind::Jira { key },
        });
    }
    Err(DocumentError::UnrecognizedLink(url.to_string()))
}

fn parse_sync(text: &str) -> Result<Vec<String>, DocumentError> {
    // A `---` rule inside the body must not be read as an excerpt cut.
    let mut matter: Matter<YAML> = Matter::new();
    matter.excerpt_delimiter = Some("\u{0}".to_string());
    let parsed = matter
        .parse::<SyncFront>(text)
        .map_err(|err| DocumentError::InvalidYaml(err.to_string()))?;
    let Some(front) = parsed.data else {
        return Err(DocumentError::MissingSync);
    };
    front.sync.links()
}

fn split_preamble(text: &str) -> Option<(String, String)> {
    let (first, rest) = text.split_once('\n')?;
    if first.trim_end() != "---" {
        return None;
    }
    let mut offset = first.len() + 1;
    for line in rest.split_inclusive('\n') {
        offset += line.len();
        if line.trim_end() == "---" {
            return Some((text[..offset].to_string(), text[offset..].to_string()));
        }
    }
    None
}

fn google_doc_id(url: &str) -> Option<String> {
    static RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^https?://docs\.google\.com/document/d/([A-Za-z0-9_-]+)").expect("regex")
    });
    RE.captures(url)
        .and_then(|caps| caps.get(1))
        .map(|id| id.as_str().to_string())
}

fn confluence_page_id(url: &str) -> Option<String> {
    static PAGES: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"/pages/(\d+)").expect("regex"));
    static PAGE_ID: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"[?&]pageId=(\d+)").expect("regex"));
    PAGES
        .captures(url)
        .or_else(|| PAGE_ID.captures(url))
        .and_then(|caps| caps.get(1))
        .map(|id| id.as_str().to_string())
}

fn jira_key(url: &str) -> Option<String> {
    static BROWSE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^https?://[^/\s]+/browse/([A-Z][A-Z0-9]+-\d+)(?:[/?#].*)?$").expect("regex")
    });
    static SELECTED: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^https?://[^/\s]+/.*[?&]selectedIssue=([A-Z][A-Z0-9]+-\d+)(?:&|#|$)")
            .expect("regex")
    });
    BROWSE
        .captures(url)
        .or_else(|| SELECTED.captures(url))
        .and_then(|caps| caps.get(1))
        .map(|key| key.as_str().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const JIRA: &str = "https://acme.atlassian.net/browse/BOOK-12";

    #[test]
    fn a_horizontal_rule_stays_in_the_body() {
        let text = format!("---\nsync: {JIRA}\n---\nBefore\n\n---\n\nAfter\n");
        let manuscript = parse(&text).unwrap();
        assert!(manuscript.body.contains("After"));
        assert!(manuscript.preamble.contains("sync:"));
        assert_eq!(
            manuscript.links[0].kind,
            LinkKind::Jira {
                key: "BOOK-12".into()
            }
        );
    }

    #[test]
    fn a_trailing_slash_is_still_the_work_item() {
        let link = classify("https://acme.atlassian.net/browse/BOOK-12/").unwrap();
        assert_eq!(
            link.kind,
            LinkKind::Jira {
                key: "BOOK-12".into()
            }
        );
    }

    #[test]
    fn a_blank_link_in_a_list_is_empty() {
        let text = format!("---\nsync:\n  - \"\"\n  - {JIRA}\n---\n");
        let err = parse(&text).unwrap_err();
        assert!(matches!(err, DocumentError::EmptySync));
    }

    #[test]
    fn a_number_is_not_a_sync_link() {
        let err = parse("---\nsync: 12\n---\n").unwrap_err();
        assert!(matches!(
            err,
            DocumentError::SyncShape | DocumentError::InvalidYaml(_)
        ));
    }
}
