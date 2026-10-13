//! Google Docs REST API.
//!
//! The token comes from `GOOGLE_ACCESS_TOKEN` or from
//! `gcloud auth application-default print-access-token`. The Docs scope is
//! not part of a default ADC login; see the README.

use serde_json::{json, Value};

use super::http::{require_success, HttpClient, HttpRequest};
use super::{RemoteCopy, RemoteError, RemotePort};
use crate::document::{ClassifiedLink, LinkKind};
use crate::sync_state::normalize_body;

/// Google Docs addressed by the document id in the sync URL.
pub struct GoogleDocsRest<H> {
    http: H,
    /// `Some` in tests. `None` resolves a token on each call.
    token: Option<String>,
}

impl GoogleDocsRest<super::http::UreqHttp> {
    /// Resolve a bearer token from the environment or gcloud when a call is made.
    pub fn from_env() -> Self {
        Self {
            http: super::http::UreqHttp,
            token: None,
        }
    }
}

impl<H> GoogleDocsRest<H> {
    /// Build an adapter with a fixed token. Tests use this.
    pub fn new(http: H, token: impl Into<String>) -> Self {
        Self {
            http,
            token: Some(token.into()),
        }
    }
}

impl<H: HttpClient> RemotePort for GoogleDocsRest<H> {
    fn fetch(&self, link: &ClassifiedLink) -> Result<RemoteCopy, RemoteError> {
        let document_id = document_id(link)?;
        let response = self.http.send(&HttpRequest {
            method: "GET",
            url: format!("https://docs.googleapis.com/v1/documents/{document_id}"),
            headers: self.headers()?,
            body: None,
        })?;
        let body = require_success(response, "Google Docs")?;
        parse_document(&body)
    }

    fn push(&self, link: &ClassifiedLink, body: &str) -> Result<RemoteCopy, RemoteError> {
        let document_id = document_id(link)?;
        let raw = self.raw_document(link)?;
        let value: Value = serde_json::from_str(&raw).map_err(|err| {
            RemoteError::Message(format!("Google Docs did not return JSON: {err}"))
        })?;
        let parsed = parse_document_value(&value)?;
        let end_index = parsed.end_index;
        let mut requests = Vec::new();
        if end_index > 2 {
            requests.push(json!({
                "deleteContentRange": {
                    "range": { "startIndex": 1, "endIndex": end_index - 1 }
                }
            }));
        }
        let insert = normalize_body(body);
        let insert = insert.trim_end_matches('\n');
        if !insert.is_empty() {
            requests.push(json!({
                "insertText": {
                    "location": { "index": 1 },
                    "text": insert
                }
            }));
        }
        let response = self.http.send(&HttpRequest {
            method: "POST",
            url: format!("https://docs.googleapis.com/v1/documents/{document_id}:batchUpdate"),
            headers: self.headers()?,
            body: Some(json!({ "requests": requests }).to_string()),
        })?;
        require_success(response, "Google Docs")?;
        self.fetch(link)
    }
}

impl<H: HttpClient> GoogleDocsRest<H> {
    fn headers(&self) -> Result<Vec<(String, String)>, RemoteError> {
        let mut headers = vec![
            ("Authorization".into(), format!("Bearer {}", self.bearer()?)),
            ("Accept".into(), "application/json".into()),
            ("Content-Type".into(), "application/json".into()),
        ];
        if let Ok(project) = std::env::var("GOOGLE_CLOUD_QUOTA_PROJECT") {
            if !project.is_empty() {
                headers.push(("x-goog-user-project".into(), project));
            }
        }
        Ok(headers)
    }

    fn bearer(&self) -> Result<String, RemoteError> {
        if let Some(token) = &self.token {
            if !token.is_empty() {
                return Ok(token.clone());
            }
        }
        access_token()
    }

    fn raw_document(&self, link: &ClassifiedLink) -> Result<String, RemoteError> {
        let document_id = document_id(link)?;
        let response = self.http.send(&HttpRequest {
            method: "GET",
            url: format!("https://docs.googleapis.com/v1/documents/{document_id}"),
            headers: self.headers()?,
            body: None,
        })?;
        require_success(response, "Google Docs")
    }
}

fn document_id(link: &ClassifiedLink) -> Result<String, RemoteError> {
    match &link.kind {
        LinkKind::GoogleDoc { document_id } => Ok(document_id.clone()),
        _ => Err(RemoteError::Message(format!(
            "{} is not a Google Doc",
            link.url
        ))),
    }
}

/// `GOOGLE_ACCESS_TOKEN`, otherwise `gcloud auth application-default print-access-token`.
pub fn access_token() -> Result<String, RemoteError> {
    if let Ok(token) = std::env::var("GOOGLE_ACCESS_TOKEN") {
        let token = token.trim().to_string();
        if !token.is_empty() {
            return Ok(token);
        }
    }
    let output = std::process::Command::new("gcloud")
        .args(["auth", "application-default", "print-access-token"])
        .output()
        .map_err(|err| {
            if err.kind() == std::io::ErrorKind::NotFound {
                RemoteError::Message(
                    "`gcloud` was not found. Install the Google Cloud CLI, or export GOOGLE_ACCESS_TOKEN. The Docs scope has to be requested explicitly; see the README.".into(),
                )
            } else {
                RemoteError::Message(format!("could not run gcloud: {err}"))
            }
        })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(RemoteError::Message(format!(
            "gcloud auth application-default print-access-token failed: {}",
            stderr.trim()
        )));
    }
    let token = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if token.is_empty() {
        return Err(RemoteError::Message(
            "gcloud returned an empty access token".into(),
        ));
    }
    Ok(token)
}

/// Parse a `documents.get` body into text and a revision.
pub fn parse_document(json: &str) -> Result<RemoteCopy, RemoteError> {
    let value: Value = serde_json::from_str(json)
        .map_err(|err| RemoteError::Message(format!("Google Docs did not return JSON: {err}")))?;
    let parsed = parse_document_value(&value)?;
    Ok(RemoteCopy {
        revision: parsed.revision,
        body: parsed.text,
    })
}

struct ParsedDoc {
    revision: String,
    text: String,
    end_index: usize,
}

fn parse_document_value(value: &Value) -> Result<ParsedDoc, RemoteError> {
    let revision = value
        .get("revisionId")
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_string();
    let content = value.pointer("/body/content");
    let text = content.map(collect_text).unwrap_or_default();
    let end_index = content.map(max_end_index).unwrap_or(2).max(2);
    Ok(ParsedDoc {
        revision,
        text: normalize_body(&text),
        end_index,
    })
}

fn collect_text(value: &Value) -> String {
    match value {
        Value::Array(items) => items.iter().map(collect_text).collect(),
        Value::Object(map) => {
            if let Some(text) = map
                .get("textRun")
                .and_then(|run| run.get("content"))
                .and_then(Value::as_str)
            {
                return text.to_string();
            }
            map.values().map(collect_text).collect()
        }
        _ => String::new(),
    }
}

fn max_end_index(value: &Value) -> usize {
    match value {
        Value::Array(items) => items.iter().map(max_end_index).max().unwrap_or(0),
        Value::Object(map) => {
            let own = map
                .get("endIndex")
                .and_then(Value::as_u64)
                .map(|n| n as usize)
                .unwrap_or(0);
            map.values().map(max_end_index).fold(own, usize::max)
        }
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::remote::http::{HttpClient, HttpRequest, HttpResponse};
    use std::sync::Mutex;

    #[test]
    fn a_document_yields_its_revision_and_text() {
        let copy = parse_document(
            r#"{
                "revisionId": "rev-9",
                "body": { "content": [
                    { "endIndex": 14, "paragraph": { "elements": [
                        { "endIndex": 14, "textRun": { "content": "From the doc.\n" } }
                    ]}}
                ]}
            }"#,
        )
        .unwrap();
        assert_eq!(copy.revision, "rev-9");
        assert_eq!(copy.body, "From the doc.\n");
    }

    struct Script {
        responses: Mutex<Vec<HttpResponse>>,
        requests: Mutex<Vec<HttpRequest>>,
    }

    impl HttpClient for Script {
        fn send(&self, request: &HttpRequest) -> Result<HttpResponse, RemoteError> {
            self.requests
                .lock()
                .expect("requests")
                .push(request.clone());
            self.responses
                .lock()
                .expect("responses")
                .pop()
                .ok_or_else(|| RemoteError::Message("no scripted response".into()))
        }
    }

    fn link() -> ClassifiedLink {
        ClassifiedLink {
            url: "https://docs.google.com/document/d/abc_XYZ-123/edit".into(),
            kind: LinkKind::GoogleDoc {
                document_id: "abc_XYZ-123".into(),
            },
        }
    }

    fn document_json(revision: &str, text: &str, end_index: u64) -> String {
        json!({
            "revisionId": revision,
            "body": { "content": [{
                "endIndex": end_index,
                "paragraph": { "elements": [{
                    "endIndex": end_index,
                    "textRun": { "content": text }
                }]}
            }]}
        })
        .to_string()
    }

    #[test]
    fn push_deletes_the_old_range_and_inserts_the_new_text() {
        let script = Script {
            responses: Mutex::new(vec![
                HttpResponse {
                    status: 200,
                    body: document_json("rev-2", "Edited.\n", 8),
                },
                HttpResponse {
                    status: 200,
                    body: "{}".into(),
                },
                HttpResponse {
                    status: 200,
                    body: document_json("rev-1", "Old.\n", 6),
                },
            ]),
            requests: Mutex::new(Vec::new()),
        };
        let api = GoogleDocsRest::new(script, "token");
        let copy = api.push(&link(), "Edited.").unwrap();
        assert_eq!(copy.revision, "rev-2");
        assert_eq!(copy.body, "Edited.\n");
        let requests = api.http.requests.lock().unwrap().clone();
        assert_eq!(requests[1].method, "POST");
        assert!(requests[1].url.contains(":batchUpdate"));
        let payload: Value = serde_json::from_str(requests[1].body.as_deref().unwrap()).unwrap();
        let delete_end = payload.pointer("/requests/0/deleteContentRange/range/endIndex");
        assert_eq!(delete_end.and_then(Value::as_u64), Some(5));
        let inserted = payload
            .pointer("/requests/1/insertText/text")
            .and_then(Value::as_str);
        assert_eq!(inserted, Some("Edited."));
    }
}
