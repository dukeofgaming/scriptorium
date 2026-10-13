//! Jira adapter. Shells out to the official Atlassian CLI (`acli`).

use std::process::Command;

use serde_json::Value;

use super::{RemoteCopy, RemoteError, RemotePort};
use crate::document::{ClassifiedLink, LinkKind};
use crate::sync_state::normalize_body;

/// Runs a process and returns stdout when the status is success.
pub trait CommandRunner: Send + Sync {
    /// Run `program` with `args`.
    fn run(&self, program: &str, args: &[String]) -> Result<String, RemoteError>;
}

/// The real process runner.
#[derive(Debug, Default)]
pub struct SystemCommand;

impl CommandRunner for SystemCommand {
    fn run(&self, program: &str, args: &[String]) -> Result<String, RemoteError> {
        let output = Command::new(program).args(args).output().map_err(|err| {
            if err.kind() == std::io::ErrorKind::NotFound {
                RemoteError::Message(format!(
                    "`{program}` was not found. Install the official Atlassian CLI and run `acli jira auth login`."
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

/// Jira work items via `acli jira workitem view` and `edit`.
#[derive(Debug)]
pub struct JiraAcli<R> {
    program: String,
    runner: R,
}

impl JiraAcli<SystemCommand> {
    /// `acli` on `PATH`, or `SCRIPTORIUM_ACLI` when set.
    pub fn from_env() -> Self {
        let program = std::env::var("SCRIPTORIUM_ACLI").unwrap_or_else(|_| "acli".to_string());
        Self {
            program,
            runner: SystemCommand,
        }
    }
}

impl<R> JiraAcli<R> {
    /// Build an adapter around a runner. Tests pass a stub.
    pub fn new(program: impl Into<String>, runner: R) -> Self {
        Self {
            program: program.into(),
            runner,
        }
    }
}

impl<R: CommandRunner> RemotePort for JiraAcli<R> {
    fn fetch(&self, link: &ClassifiedLink) -> Result<RemoteCopy, RemoteError> {
        let key = jira_key(link)?;
        let stdout = self.runner.run(
            &self.program,
            &[
                "jira".into(),
                "workitem".into(),
                "view".into(),
                key,
                "--json".into(),
                "--fields".into(),
                "description,updated".into(),
            ],
        )?;
        parse_workitem(&stdout)
    }

    fn push(&self, link: &ClassifiedLink, body: &str) -> Result<RemoteCopy, RemoteError> {
        let key = jira_key(link)?;
        let file = std::env::temp_dir().join(format!("scriptorium-{key}.txt"));
        std::fs::write(&file, normalize_body(body)).map_err(|err| {
            RemoteError::Message(format!("could not write a description file: {err}"))
        })?;
        let edited = self.runner.run(
            &self.program,
            &[
                "jira".into(),
                "workitem".into(),
                "edit".into(),
                "--key".into(),
                key,
                "--description-file".into(),
                file.display().to_string(),
                "--yes".into(),
            ],
        );
        let _ = std::fs::remove_file(&file);
        edited?;
        self.fetch(link)
    }
}

fn jira_key(link: &ClassifiedLink) -> Result<String, RemoteError> {
    match &link.kind {
        LinkKind::Jira { key } => Ok(key.clone()),
        _ => Err(RemoteError::Message(format!(
            "{} is not a Jira work item",
            link.url
        ))),
    }
}

/// Parse `acli jira workitem view --json` output into a copy.
pub fn parse_workitem(stdout: &str) -> Result<RemoteCopy, RemoteError> {
    let value = parse_json(stdout)?;
    let issue = first_issue(&value)
        .ok_or_else(|| RemoteError::Message("acli did not return a Jira work item".to_string()))?;
    let fields = issue.get("fields").unwrap_or(issue);
    let body = fields
        .get("description")
        .map(description_to_text)
        .unwrap_or_default();
    let revision = fields
        .get("updated")
        .and_then(Value::as_str)
        .or_else(|| issue.get("updated").and_then(Value::as_str))
        .unwrap_or("unknown")
        .to_string();
    Ok(RemoteCopy { revision, body })
}

fn parse_json(stdout: &str) -> Result<Value, RemoteError> {
    let trimmed = stdout.trim();
    if let Ok(value) = serde_json::from_str(trimmed) {
        return Ok(value);
    }
    if let Some(start) = trimmed.find(['{', '[']) {
        if let Ok(value) = serde_json::from_str(&trimmed[start..]) {
            return Ok(value);
        }
    }
    Err(RemoteError::Message(
        "acli did not return JSON for the work item".to_string(),
    ))
}

fn first_issue(value: &Value) -> Option<&Value> {
    if let Some(issues) = value.get("issues").and_then(Value::as_array) {
        return issues.first();
    }
    if let Some(array) = value.as_array() {
        return array.first();
    }
    Some(value)
}

fn description_to_text(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Null => String::new(),
        Value::Array(items) => items.iter().map(description_to_text).collect(),
        Value::Object(map) => {
            let kind = map.get("type").and_then(Value::as_str);
            if kind == Some("text") {
                return map
                    .get("text")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
            }
            if kind == Some("hardBreak") {
                return "\n".to_string();
            }
            let inner = map
                .get("content")
                .map(description_to_text)
                .unwrap_or_default();
            if matches!(
                kind,
                Some("paragraph" | "heading" | "blockquote" | "codeBlock" | "listItem")
            ) {
                let mut text = inner;
                if !text.ends_with('\n') {
                    text.push('\n');
                }
                text
            } else {
                inner
            }
        }
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    struct Script {
        calls: Mutex<Vec<(String, Vec<String>)>>,
        stdout: Mutex<Vec<String>>,
    }

    impl CommandRunner for Script {
        fn run(&self, program: &str, args: &[String]) -> Result<String, RemoteError> {
            self.calls
                .lock()
                .expect("calls")
                .push((program.to_string(), args.to_vec()));
            self.stdout
                .lock()
                .expect("stdout")
                .pop()
                .ok_or_else(|| RemoteError::Message("no scripted stdout".into()))
        }
    }

    fn link() -> ClassifiedLink {
        ClassifiedLink {
            url: "https://acme.atlassian.net/browse/BOOK-12".into(),
            kind: LinkKind::Jira {
                key: "BOOK-12".into(),
            },
        }
    }

    #[test]
    fn view_reads_adf_description_and_updated() {
        let script = Script {
            calls: Mutex::new(Vec::new()),
            stdout: Mutex::new(vec![r#"{
                "key": "BOOK-12",
                "fields": {
                    "updated": "2026-10-01T12:00:00.000+0000",
                    "description": {
                        "type": "doc",
                        "version": 1,
                        "content": [
                            {"type": "paragraph", "content": [
                                {"type": "text", "text": "The exemplar."}
                            ]}
                        ]
                    }
                }
            }"#
            .into()]),
        };
        // pop() takes the last item, so the view response is the only one.
        let jira = JiraAcli::new("acli", script);
        let copy = jira.fetch(&link()).unwrap();
        assert_eq!(copy.revision, "2026-10-01T12:00:00.000+0000");
        assert_eq!(copy.body, "The exemplar.\n");
        let calls = jira.runner.calls.lock().unwrap().clone();
        assert_eq!(
            calls,
            vec![(
                "acli".into(),
                vec![
                    "jira".into(),
                    "workitem".into(),
                    "view".into(),
                    "BOOK-12".into(),
                    "--json".into(),
                    "--fields".into(),
                    "description,updated".into(),
                ]
            )]
        );
    }

    #[test]
    fn a_plain_description_and_a_json_array_are_accepted() {
        let copy =
            parse_workitem(r#"[{"fields": {"description": "Plain text", "updated": "rev-1"}}]"#)
                .unwrap();
        assert_eq!(copy.body, "Plain text");
        assert_eq!(copy.revision, "rev-1");
    }

    #[test]
    fn edit_writes_a_description_file_then_views_the_new_revision() {
        let script = Script {
            calls: Mutex::new(Vec::new()),
            stdout: Mutex::new(vec![
                r#"{"fields": {"description": "Edited", "updated": "rev-2"}}"#.into(),
                String::new(),
            ]),
        };
        let jira = JiraAcli::new("acli", script);
        let copy = jira.push(&link(), "Edited").unwrap();
        assert_eq!(copy.revision, "rev-2");
        let calls = jira.runner.calls.lock().unwrap().clone();
        assert_eq!(calls[0].1[2], "edit");
        assert!(calls[0].1.iter().any(|arg| arg == "--yes"));
        assert!(calls[0].1.iter().any(|arg| arg == "--description-file"));
        assert_eq!(calls[1].1[2], "view");
    }
}
