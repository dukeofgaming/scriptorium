//! Opt-in calls to real Jira, Confluence, Google Docs, and GitHub.
//!
//! `cargo test` skips these. Set `SCRIPTORIUM_LIVE=1` and at least one URL:
//! `SCRIPTORIUM_LIVE_JIRA_URL`, `SCRIPTORIUM_LIVE_CONFLUENCE_URL`,
//! `SCRIPTORIUM_LIVE_GOOGLE_DOC_URL`, `SCRIPTORIUM_LIVE_GITHUB_ISSUE_URL`,
//! `SCRIPTORIUM_LIVE_GITHUB_PR_URL`. Push is a second opt-in,
//! `SCRIPTORIUM_LIVE_PUSH=1`, and it writes the same body back.

use scriptorium::document::{self, LinkKind};
use scriptorium::remote::{ConfluenceRest, GitHubCli, GoogleDocsRest, JiraAcli, RemotePort};

#[test]
fn live_services_fetch_when_requested() {
    if std::env::var("SCRIPTORIUM_LIVE").ok().as_deref() != Some("1") {
        return;
    }

    let mut ran = false;
    if let Ok(url) = std::env::var("SCRIPTORIUM_LIVE_JIRA_URL") {
        ran = true;
        let link = document::classify(&url).expect("jira url");
        assert!(matches!(link.kind, LinkKind::Jira { .. }));
        let copy = JiraAcli::from_env().fetch(&link).expect("fetch jira");
        assert!(!copy.revision.is_empty(), "jira revision");
        if live_push() {
            JiraAcli::from_env()
                .push(&link, &copy.body)
                .expect("push jira");
        }
    }
    if let Ok(url) = std::env::var("SCRIPTORIUM_LIVE_CONFLUENCE_URL") {
        ran = true;
        let link = document::classify(&url).expect("confluence url");
        assert!(matches!(link.kind, LinkKind::Confluence { .. }));
        let copy = ConfluenceRest::from_env()
            .fetch(&link)
            .expect("fetch confluence");
        assert!(!copy.revision.is_empty(), "confluence revision");
        if live_push() {
            ConfluenceRest::from_env()
                .push(&link, &copy.body)
                .expect("push confluence");
        }
    }
    if let Ok(url) = std::env::var("SCRIPTORIUM_LIVE_GOOGLE_DOC_URL") {
        ran = true;
        let link = document::classify(&url).expect("google doc url");
        assert!(matches!(link.kind, LinkKind::GoogleDoc { .. }));
        let copy = GoogleDocsRest::from_env()
            .fetch(&link)
            .expect("fetch google doc");
        assert!(!copy.revision.is_empty(), "google docs revision");
        if live_push() {
            GoogleDocsRest::from_env()
                .push(&link, &copy.body)
                .expect("push google doc");
        }
    }
    if let Ok(url) = std::env::var("SCRIPTORIUM_LIVE_GITHUB_ISSUE_URL") {
        ran = true;
        let link = document::classify(&url).expect("github issue url");
        assert!(matches!(link.kind, LinkKind::GitHubIssue { .. }));
        let copy = GitHubCli::from_env()
            .fetch(&link)
            .expect("fetch github issue");
        assert!(!copy.revision.is_empty(), "github issue revision");
        if live_push() {
            GitHubCli::from_env()
                .push(&link, &copy.body)
                .expect("push github issue");
        }
    }
    if let Ok(url) = std::env::var("SCRIPTORIUM_LIVE_GITHUB_PR_URL") {
        ran = true;
        let link = document::classify(&url).expect("github pull request url");
        assert!(matches!(link.kind, LinkKind::GitHubPullRequest { .. }));
        let copy = GitHubCli::from_env()
            .fetch(&link)
            .expect("fetch github pull request");
        assert!(!copy.revision.is_empty(), "github pull request revision");
        if live_push() {
            GitHubCli::from_env()
                .push(&link, &copy.body)
                .expect("push github pull request");
        }
    }
    assert!(
        ran,
        "SCRIPTORIUM_LIVE=1 needs SCRIPTORIUM_LIVE_JIRA_URL, SCRIPTORIUM_LIVE_CONFLUENCE_URL, SCRIPTORIUM_LIVE_GOOGLE_DOC_URL, SCRIPTORIUM_LIVE_GITHUB_ISSUE_URL, or SCRIPTORIUM_LIVE_GITHUB_PR_URL"
    );
}

fn live_push() -> bool {
    std::env::var("SCRIPTORIUM_LIVE_PUSH").ok().as_deref() == Some("1")
}
