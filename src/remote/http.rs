//! HTTP used by the Confluence and Google Docs adapters.

use super::RemoteError;

/// One HTTP call. Adapters build these; tests answer them from a script.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpRequest {
    /// `GET`, `PUT`, or `POST`.
    pub method: &'static str,
    /// Absolute URL.
    pub url: String,
    /// Header pairs. The client does not add its own.
    pub headers: Vec<(String, String)>,
    /// Request body. `None` for a call with no body.
    pub body: Option<String>,
}

/// A response, including error statuses. The adapter decides what they mean.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpResponse {
    /// HTTP status code.
    pub status: u16,
    /// Response body.
    pub body: String,
}

/// The HTTP port. `ureq` implements it for live calls.
pub trait HttpClient: Send + Sync {
    /// Send `request` and return the status and body.
    fn send(&self, request: &HttpRequest) -> Result<HttpResponse, RemoteError>;
}

/// Live client. TLS uses rustls and the webpki root set.
#[derive(Debug, Default)]
pub struct UreqHttp;

impl HttpClient for UreqHttp {
    fn send(&self, request: &HttpRequest) -> Result<HttpResponse, RemoteError> {
        let mut call = ureq::request(request.method, &request.url);
        for (name, value) in &request.headers {
            call = call.set(name, value);
        }
        let result = match &request.body {
            Some(body) => call.send_string(body),
            None => call.call(),
        };
        match result {
            Ok(response) => Ok(HttpResponse {
                status: response.status(),
                body: response.into_string().map_err(|err| {
                    RemoteError::Message(format!("could not read the response body: {err}"))
                })?,
            }),
            Err(ureq::Error::Status(status, response)) => Ok(HttpResponse {
                status,
                body: response.into_string().unwrap_or_default(),
            }),
            Err(err) => Err(RemoteError::Message(format!(
                "request to {} failed: {err}",
                request.url
            ))),
        }
    }
}

pub(crate) fn require_success(response: HttpResponse, what: &str) -> Result<String, RemoteError> {
    if (200..300).contains(&response.status) {
        Ok(response.body)
    } else {
        Err(RemoteError::Message(format!(
            "{what} returned HTTP {}: {}",
            response.status,
            response.body.trim()
        )))
    }
}
