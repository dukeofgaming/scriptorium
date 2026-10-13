//! Confluence Cloud REST API v2.
//!
//! `acli` can view some pages and cannot update them, so writes go to the
//! official REST API with the same API token a scribe uses for `acli`.

use serde_json::{json, Value};

use super::http::{require_success, HttpClient, HttpRequest};
use super::{RemoteCopy, RemoteError, RemotePort};
use crate::document::{ClassifiedLink, LinkKind};
use crate::sync_state::normalize_body;

/// Confluence pages addressed by the numeric id in the sync URL.
pub struct ConfluenceRest<H> {
    http: H,
    email: String,
    token: String,
}

impl ConfluenceRest<super::http::UreqHttp> {
    /// Read `ATLASSIAN_EMAIL` and `ATLASSIAN_API_TOKEN`. The site host comes from the link.
    pub fn from_env() -> Self {
        Self {
            http: super::http::UreqHttp,
            email: std::env::var("ATLASSIAN_EMAIL").unwrap_or_default(),
            token: std::env::var("ATLASSIAN_API_TOKEN").unwrap_or_default(),
        }
    }
}

impl<H> ConfluenceRest<H> {
    /// Build an adapter around an HTTP client. Tests pass a script.
    pub fn new(http: H, email: impl Into<String>, token: impl Into<String>) -> Self {
        Self {
            http,
            email: email.into(),
            token: token.into(),
        }
    }
}

impl<H: HttpClient> RemotePort for ConfluenceRest<H> {
    fn fetch(&self, link: &ClassifiedLink) -> Result<RemoteCopy, RemoteError> {
        let page_id = page_id(link)?;
        let host = https_host(&link.url)?;
        self.credentials()?;
        let response = self.http.send(&HttpRequest {
            method: "GET",
            url: format!("https://{host}/wiki/api/v2/pages/{page_id}?body-format=storage"),
            headers: self.headers(),
            body: None,
        })?;
        let body = require_success(response, "Confluence")?;
        parse_page(&body)
    }

    fn push(&self, link: &ClassifiedLink, body: &str) -> Result<RemoteCopy, RemoteError> {
        let page_id = page_id(link)?;
        let host = https_host(&link.url)?;
        self.credentials()?;
        let page = fetch_raw_page(&self.http, &host, &page_id, &self.headers())?;
        let title = page
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or("Untitled")
            .to_string();
        let current = parse_page(&page.to_string())?;
        let version = current
            .revision
            .parse::<u64>()
            .unwrap_or(1)
            .saturating_add(1);
        let payload = json!({
            "id": page_id,
            "status": "current",
            "title": title,
            "body": {
                "representation": "storage",
                "value": text_to_storage(body),
            },
            "version": {
                "number": version,
                "message": "Scriptorium push",
            }
        });
        let response = self.http.send(&HttpRequest {
            method: "PUT",
            url: format!("https://{host}/wiki/api/v2/pages/{page_id}"),
            headers: self.headers(),
            body: Some(payload.to_string()),
        })?;
        let updated = require_success(response, "Confluence")?;
        let parsed = parse_page(&updated).unwrap_or(RemoteCopy {
            revision: version.to_string(),
            body: normalize_body(body),
        });
        Ok(RemoteCopy {
            revision: parsed.revision,
            body: normalize_body(body),
        })
    }
}

impl<H> ConfluenceRest<H> {
    fn credentials(&self) -> Result<(), RemoteError> {
        if self.email.is_empty() || self.token.is_empty() {
            return Err(RemoteError::Message(
                "Confluence needs ATLASSIAN_EMAIL and ATLASSIAN_API_TOKEN. Create an API token at https://id.atlassian.com/manage-profile/security/api-tokens and export both variables.".into(),
            ));
        }
        Ok(())
    }

    fn headers(&self) -> Vec<(String, String)> {
        vec![
            ("Authorization".into(), basic_auth(&self.email, &self.token)),
            ("Accept".into(), "application/json".into()),
            ("Content-Type".into(), "application/json".into()),
        ]
    }
}

fn fetch_raw_page<H: HttpClient>(
    http: &H,
    host: &str,
    page_id: &str,
    headers: &[(String, String)],
) -> Result<Value, RemoteError> {
    let response = http.send(&HttpRequest {
        method: "GET",
        url: format!("https://{host}/wiki/api/v2/pages/{page_id}?body-format=storage"),
        headers: headers.to_vec(),
        body: None,
    })?;
    let body = require_success(response, "Confluence")?;
    serde_json::from_str(&body)
        .map_err(|err| RemoteError::Message(format!("Confluence did not return JSON: {err}")))
}

fn page_id(link: &ClassifiedLink) -> Result<String, RemoteError> {
    match &link.kind {
        LinkKind::Confluence { page_id } => Ok(page_id.clone()),
        _ => Err(RemoteError::Message(format!(
            "{} is not a Confluence page",
            link.url
        ))),
    }
}

pub(crate) fn https_host(url: &str) -> Result<String, RemoteError> {
    let rest = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .ok_or_else(|| RemoteError::Message(format!("{url} has no host")))?;
    let host = rest.split(['/', '?', '#']).next().unwrap_or("");
    if host.is_empty() {
        return Err(RemoteError::Message(format!("{url} has no host")));
    }
    Ok(host.to_string())
}

/// Parse a Confluence v2 page into a revision and plain text.
pub fn parse_page(json: &str) -> Result<RemoteCopy, RemoteError> {
    let value: Value = serde_json::from_str(json)
        .map_err(|err| RemoteError::Message(format!("Confluence did not return JSON: {err}")))?;
    let revision = value
        .pointer("/version/number")
        .and_then(Value::as_u64)
        .or_else(|| {
            value
                .pointer("/version/number")
                .and_then(Value::as_i64)
                .map(|n| n as u64)
        })
        .map(|n| n.to_string())
        .unwrap_or_else(|| "unknown".into());
    let storage = value
        .pointer("/body/storage/value")
        .and_then(Value::as_str)
        .unwrap_or("");
    Ok(RemoteCopy {
        revision,
        body: storage_to_text(storage),
    })
}

/// Turn a Confluence storage fragment into plain text.
pub fn storage_to_text(html: &str) -> String {
    let mut tagged = html
        .replace("<br />", "\n")
        .replace("<br/>", "\n")
        .replace("<br>", "\n");
    for end in ["</p>", "</h1>", "</h2>", "</h3>", "</li>"] {
        tagged = tagged.replace(end, "\n\n");
    }
    let stripped = strip_tags(&tagged);
    let decoded = decode_entities(&stripped);
    normalize_body(decoded.trim())
}

/// Wrap plain text in storage paragraphs. Blank lines separate paragraphs.
pub fn text_to_storage(text: &str) -> String {
    let text = normalize_body(text);
    let text = text.trim_end_matches('\n');
    if text.is_empty() {
        return "<p></p>".to_string();
    }
    text.split("\n\n")
        .map(|paragraph| {
            let inner = paragraph
                .split('\n')
                .map(escape_html)
                .collect::<Vec<_>>()
                .join("<br />");
            format!("<p>{inner}</p>")
        })
        .collect()
}

fn strip_tags(html: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    out
}

fn decode_entities(text: &str) -> String {
    text.replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
}

fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn basic_auth(email: &str, token: &str) -> String {
    format!("Basic {}", base64(format!("{email}:{token}").as_bytes()))
}

fn base64(data: &[u8]) -> String {
    const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    let mut index = 0;
    while index + 3 <= data.len() {
        let chunk = u32::from_be_bytes([0, data[index], data[index + 1], data[index + 2]]);
        for shift in [18, 12, 6, 0] {
            out.push(TABLE[((chunk >> shift) & 0x3f) as usize] as char);
        }
        index += 3;
    }
    let rest = data.len() - index;
    if rest == 1 {
        let chunk = u32::from(data[index]) << 16;
        out.push(TABLE[((chunk >> 18) & 0x3f) as usize] as char);
        out.push(TABLE[((chunk >> 12) & 0x3f) as usize] as char);
        out.push('=');
        out.push('=');
    } else if rest == 2 {
        let chunk = (u32::from(data[index]) << 16) | (u32::from(data[index + 1]) << 8);
        out.push(TABLE[((chunk >> 18) & 0x3f) as usize] as char);
        out.push(TABLE[((chunk >> 12) & 0x3f) as usize] as char);
        out.push(TABLE[((chunk >> 6) & 0x3f) as usize] as char);
        out.push('=');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::remote::http::{HttpClient, HttpRequest, HttpResponse};
    use std::sync::Mutex;

    #[test]
    fn storage_round_trip_keeps_paragraphs_and_line_breaks() {
        let storage = "<p>Hello<br />there</p><p>World &amp; co</p>";
        let text = storage_to_text(storage);
        assert_eq!(text, "Hello\nthere\n\nWorld & co\n");
        assert_eq!(
            text_to_storage(&text),
            "<p>Hello<br />there</p><p>World &amp; co</p>"
        );
    }

    #[test]
    fn basic_auth_encodes_the_token() {
        assert_eq!(base64(b"user:token"), "dXNlcjp0b2tlbg==");
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

    #[test]
    fn fetch_reads_version_and_storage() {
        let script = Script {
            responses: Mutex::new(vec![HttpResponse {
                status: 200,
                body: r#"{"title":"Colophon","version":{"number":7},"body":{"storage":{"value":"<p>From the page.</p>"}}}"#.into(),
            }]),
            requests: Mutex::new(Vec::new()),
        };
        let api = ConfluenceRest::new(script, "you@acme.com", "token");
        let link = ClassifiedLink {
            url: "https://acme.atlassian.net/wiki/spaces/LIB/pages/98765/Colophon".into(),
            kind: LinkKind::Confluence {
                page_id: "98765".into(),
            },
        };
        let copy = api.fetch(&link).unwrap();
        assert_eq!(copy.revision, "7");
        assert_eq!(copy.body, "From the page.\n");
        let url = api.http.requests.lock().unwrap()[0].url.clone();
        assert!(url.contains("/wiki/api/v2/pages/98765"));
    }
}
