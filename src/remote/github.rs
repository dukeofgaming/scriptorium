//! GitHub issues and pull requests via the official GitHub CLI (`gh`).
//!
//! The copied text is the issue body or the pull request body. `updatedAt`
//! is the revision, the same role Jira's `updated` field plays.

use std::process::{Command, Stdio};

use serde_json::Value;

use super::{RemoteCopy, RemoteError, RemotePort};
use crate::document::{ClassifiedLink, LinkKind};
use crate::sync_state::normalize_body;

/// Runs a process and returns stdout when the status is success.
pub trait CommandRunner: Send + Sync {
    /// Run `program` with `args`.
    fn run(&self, program: &str, args: &[String]) -> Result<String, RemoteError>;
}

/// The real process runner. Prompts are disabled so `gh` cannot wait on a scribe.
#[derive(Debug, Default)]
pub struct SystemCommand;

impl CommandRunner for SystemCommand {
    fn run(&self, program: &str, args: &[String]) -> Result<String, RemoteError> {
        let output = Command::new(program)
            .args(args)
            .env("GH_PROMPT_DISABLED", "1")
            .stdin(Stdio::null())
            .output()
            .map_err(|err| {
                if err.kind() == std::io::ErrorKind::NotFound {
                    RemoteError::Message(format!(
                        "`{program}` was not found. Install the GitHub CLI and run `gh auth login`."
                    ))
                } else {
                    RemoteError::Message(format!("could not run {program}: {err}"))
                }
            })?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stderr = stderr.trim();
            return Err(RemoteError::Message(format!(
                "{program} exited with {}: {stderr}",
                output.status
            )));
        }
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }
}

/// GitHub issues and pull requests via `gh issue` and `gh pr`.
#[derive(Debug)]
pub struct GitHubCli<R> {
    program: String,
    runner: R,
}

impl GitHubCli<SystemCommand> {
    /// `gh` on `PATH`, or `SCRIPTORIUM_GH` when set.
    pub fn from_env() -> Self {
        let program = std::env::var("SCRIPTORIUM_GH").unwrap_or_else(|_| "gh".to_string());
        Self {
            program,
            runner: SystemCommand,
        }
    }
}

impl<R> GitHubCli<R> {
    /// Build an adapter around a runner. Tests pass a stub.
    pub fn new(program: impl Into<String>, runner: R) -> Self {
        Self {
            program: program.into(),
            runner,
        }
    }
}

impl<R: CommandRunner> RemotePort for GitHubCli<R> {
    fn fetch(&self, link: &ClassifiedLink) -> Result<RemoteCopy, RemoteError> {
        let noun = noun(link)?;
        let stdout = self.runner.run(
            &self.program,
            &[
                noun.into(),
                "view".into(),
                link.url.clone(),
                "--json".into(),
                "body,updatedAt".into(),
            ],
        )?;
        parse_view(&stdout)
    }

    fn push(&self, link: &ClassifiedLink, body: &str) -> Result<RemoteCopy, RemoteError> {
        let noun = noun(link)?;
        let file =
            std::env::temp_dir().join(format!("scriptorium-gh-{}-{noun}.md", target_label(link)));
        std::fs::write(&file, normalize_body(body))
            .map_err(|err| RemoteError::Message(format!("could not write a body file: {err}")))?;
        let edited = self.runner.run(
            &self.program,
            &[
                noun.into(),
                "edit".into(),
                link.url.clone(),
                "--body-file".into(),
                file.display().to_string(),
            ],
        );
        let _ = std::fs::remove_file(&file);
        edited?;
        self.fetch(link)
    }
}

fn noun(link: &ClassifiedLink) -> Result<&'static str, RemoteError> {
    match &link.kind {
        LinkKind::GitHubIssue { .. } => Ok("issue"),
        LinkKind::GitHubPullRequest { .. } => Ok("pr"),
        _ => Err(RemoteError::Message(format!(
            "{} is not a GitHub issue or pull request",
            link.url
        ))),
    }
}

fn target_label(link: &ClassifiedLink) -> String {
    match &link.kind {
        LinkKind::GitHubIssue {
            owner,
            repo,
            number,
        }
        | LinkKind::GitHubPullRequest {
            owner,
            repo,
            number,
        } => format!("{owner}-{repo}-{number}"),
        _ => "copy".to_string(),
    }
}

/// Parse `gh issue view --json` or `gh pr view --json` output into a copy.
pub fn parse_view(stdout: &str) -> Result<RemoteCopy, RemoteError> {
    let value = parse_json(stdout)?;
    if !value.is_object() {
        return Err(RemoteError::Message(
            "gh did not return a GitHub issue or pull request".to_string(),
        ));
    }
    let body = match value.get("body") {
        Some(Value::String(text)) => text.clone(),
        Some(Value::Null) | None => String::new(),
        Some(_) => {
            return Err(RemoteError::Message(
                "gh returned a body that is not text".to_string(),
            ))
        }
    };
    let revision = value
        .get("updatedAt")
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_string();
    Ok(RemoteCopy {
        revision,
        body: normalize_body(&body),
    })
}

fn parse_json(stdout: &str) -> Result<Value, RemoteError> {
    let trimmed = stdout.trim();
    if let Ok(value) = serde_json::from_str(trimmed) {
        return Ok(value);
    }
    if let Some(start) = trimmed.find('{') {
        if let Ok(value) = serde_json::from_str(&trimmed[start..]) {
            return Ok(value);
        }
    }
    Err(RemoteError::Message(
        "gh did not return JSON for the issue or pull request".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    struct Script {
        calls: Mutex<Vec<(String, Vec<String>)>>,
        stdout: Mutex<Vec<String>>,
        written: Mutex<Vec<String>>,
    }

    impl CommandRunner for Script {
        fn run(&self, program: &str, args: &[String]) -> Result<String, RemoteError> {
            self.calls
                .lock()
                .expect("calls")
                .push((program.to_string(), args.to_vec()));
            if let Some(index) = args.iter().position(|arg| arg == "--body-file") {
                let path = args.get(index + 1).expect("body file path");
                let text = std::fs::read_to_string(path).expect("body file");
                self.written.lock().expect("written").push(text);
            }
            self.stdout
                .lock()
                .expect("stdout")
                .pop()
                .ok_or_else(|| RemoteError::Message("no scripted stdout".into()))
        }
    }

    fn issue() -> ClassifiedLink {
        ClassifiedLink {
            url: "https://github.com/acme/library/issues/12".into(),
            kind: LinkKind::GitHubIssue {
                owner: "acme".into(),
                repo: "library".into(),
                number: "12".into(),
            },
        }
    }

    fn pull_request() -> ClassifiedLink {
        ClassifiedLink {
            url: "https://github.com/acme/library/pull/4".into(),
            kind: LinkKind::GitHubPullRequest {
                owner: "acme".into(),
                repo: "library".into(),
                number: "4".into(),
            },
        }
    }

    #[test]
    fn view_reads_body_and_updated_at() {
        let script = Script {
            calls: Mutex::new(Vec::new()),
            stdout: Mutex::new(vec![r#"{
                "body": "The exemplar.",
                "updatedAt": "2026-10-01T12:00:00Z"
            }"#
            .into()]),
            written: Mutex::new(Vec::new()),
        };
        let github = GitHubCli::new("gh", script);
        let copy = github.fetch(&issue()).unwrap();
        assert_eq!(copy.revision, "2026-10-01T12:00:00Z");
        assert_eq!(copy.body, "The exemplar.\n");
        let calls = github.runner.calls.lock().unwrap().clone();
        assert_eq!(
            calls,
            vec![(
                "gh".into(),
                vec![
                    "issue".into(),
                    "view".into(),
                    "https://github.com/acme/library/issues/12".into(),
                    "--json".into(),
                    "body,updatedAt".into(),
                ]
            )]
        );
    }

    #[test]
    fn a_null_body_is_empty() {
        let copy = parse_view(r#"{"body": null, "updatedAt": "rev-1"}"#).unwrap();
        assert_eq!(copy.body, "\n");
        assert_eq!(copy.revision, "rev-1");
    }

    #[test]
    fn edit_writes_a_body_file_then_views_the_new_revision() {
        let script = Script {
            calls: Mutex::new(Vec::new()),
            stdout: Mutex::new(vec![
                r#"{"body": "Edited.", "updatedAt": "rev-2"}"#.into(),
                String::new(),
            ]),
            written: Mutex::new(Vec::new()),
        };
        let github = GitHubCli::new("gh", script);
        let copy = github.push(&issue(), "Edited.").unwrap();
        assert_eq!(copy.revision, "rev-2");
        assert_eq!(copy.body, "Edited.\n");
        let calls = github.runner.calls.lock().unwrap().clone();
        assert_eq!(calls[0].1[0], "issue");
        assert_eq!(calls[0].1[1], "edit");
        assert!(calls[0].1.iter().any(|arg| arg == "--body-file"));
        assert_eq!(calls[1].1[1], "view");
        let written = github.runner.written.lock().unwrap().clone();
        assert_eq!(written, vec!["Edited.\n".to_string()]);
    }

    #[test]
    fn a_pull_request_uses_pr_view_and_pr_edit() {
        let script = Script {
            calls: Mutex::new(Vec::new()),
            stdout: Mutex::new(vec![
                r#"{"body": "From the manuscript.", "updatedAt": "rev-9"}"#.into(),
                String::new(),
            ]),
            written: Mutex::new(Vec::new()),
        };
        let github = GitHubCli::new("gh", script);
        let copy = github
            .push(&pull_request(), "From the manuscript.")
            .unwrap();
        assert_eq!(copy.revision, "rev-9");
        let calls = github.runner.calls.lock().unwrap().clone();
        assert_eq!(calls[0].1[0], "pr");
        assert_eq!(calls[0].1[1], "edit");
        assert_eq!(calls[0].1[2], "https://github.com/acme/library/pull/4");
        assert_eq!(calls[1].1[0], "pr");
        assert_eq!(calls[1].1[1], "view");
        assert!(calls[1].1.iter().any(|arg| arg == "body,updatedAt"));
    }
}
