//! Shared HTTP transport seam used by the LLM-backed agents.
//!
//! Each agent backend (`Claude`, `Ollama`, `OpenAI`, …) needs to POST a JSON body
//! to a remote endpoint and read back a string response. Rather than each
//! backend inventing its own `HttpPoster` trait and `ReqwestPoster` pair,
//! this module provides a single canonical trait and a production-grade
//! reqwest-backed implementation.
//!
//! # Why a trait (and not just `reqwest::Client`)?
//!
//! Tests inject a `MockPoster` that returns canned responses, so unit tests
//! never open sockets. The trait is the test seam.
//!
//! # Why `&[u8]` for the body?
//!
//! Most callers serialize JSON to `String` and call `.as_bytes()`; a few
//! serialize via `serde_json::to_vec`. Using `&[u8]` accepts both without
//! forcing an extra allocation.

use async_trait::async_trait;
use std::sync::Arc;
use std::time::Duration;

/// An error returned by the low-level HTTP transport.
#[derive(Clone, Debug)]
pub struct HttpPostError {
    /// HTTP status code, if any (e.g. 429, 500). `None` if the request
    /// failed before a response was received (DNS, connect, TLS…).
    pub status: Option<u16>,
    /// Human-readable message describing the failure.
    pub message: String,
    /// Parsed `Retry-After` header value in seconds, if present in the
    /// response. Only populated for 429 and 529 responses.
    pub retry_after_secs: Option<u64>,
}

impl HttpPostError {
    /// Build a pre-response transport error (no HTTP status).
    #[must_use]
    pub fn transport(message: impl Into<String>) -> Self {
        Self {
            status: None,
            message: message.into(),
            retry_after_secs: None,
        }
    }

    /// Build an error from an HTTP status + response body.
    #[must_use]
    pub fn http(status: u16, message: impl Into<String>) -> Self {
        Self {
            status: Some(status),
            message: message.into(),
            retry_after_secs: None,
        }
    }

    /// Build an error from an HTTP status, response body, and optional
    /// `Retry-After` duration in seconds.
    #[must_use]
    pub fn http_with_retry_after(
        status: u16,
        message: impl Into<String>,
        retry_after_secs: Option<u64>,
    ) -> Self {
        Self {
            status: Some(status),
            message: message.into(),
            retry_after_secs,
        }
    }
}

/// Parse a `Retry-After` header value into a duration in seconds.
///
/// Supports two formats per RFC 7231 section 7.1.3:
/// - Integer seconds (e.g. `"30"`)
/// - HTTP-date (e.g. `"Sun, 06 Nov 1994 08:49:37 GMT"`)
///
/// Returns `None` if the header is absent, unparseable, or represents a
/// date in the past.
#[must_use]
pub fn parse_retry_after(header_value: &str) -> Option<u64> {
    let trimmed = header_value.trim();
    if trimmed.is_empty() {
        return None;
    }

    // Try integer seconds first (most common).
    if let Ok(seconds) = trimmed.parse::<u64>() {
        return Some(seconds);
    }

    // Try HTTP-date (RFC 7231): e.g. "Sun, 06 Nov 1994 08:49:37 GMT"
    if let Ok(date) = httpdate::parse_http_date(trimmed) {
        let now = std::time::SystemTime::now();
        return date
            .duration_since(now)
            .ok()
            .map(|dur| dur.as_secs().max(1));
    }

    tracing::warn!(
        retry_after = trimmed,
        "failed to parse Retry-After header value; falling back to default backoff"
    );
    None
}

/// Extract a `Retry-After` value from a reqwest response (public for use
/// by streaming backends that handle the HTTP response directly).
#[must_use]
pub fn extract_retry_after_from_response(response: &reqwest::Response) -> Option<u64> {
    let status = response.status().as_u16();
    if status != 429 && status != 529 && status != 503 {
        return None;
    }
    let header_value = response
        .headers()
        .get("retry-after")
        .and_then(|v| v.to_str().ok())?;
    parse_retry_after(header_value)
}

impl std::fmt::Display for HttpPostError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.status {
            Some(code) => write!(f, "http {code}: {}", self.message),
            None => write!(f, "transport error: {}", self.message),
        }
    }
}

/// Transport abstraction so tests can inject a mock without hitting the network.
///
/// Implementors POST `body` as JSON to `url`, attaching the provided `headers`.
/// On 2xx, return the response body as a `String`. On any non-2xx status or
/// transport failure, return [`HttpPostError`].
///
/// Implementations MUST honour `timeout_ms`: if the remote does not respond
/// within the deadline the call should return an [`HttpPostError::transport`].
#[async_trait]
pub trait HttpPoster: Send + Sync {
    /// Post `body` to `url` with `headers`. See trait docs for semantics.
    async fn post_json(
        &self,
        url: &str,
        headers: &[(String, String)],
        body: &[u8],
        timeout_ms: u64,
    ) -> Result<String, HttpPostError>;

    /// Perform a GET request to `url` with `headers`.
    ///
    /// The default implementation returns an error. Override in implementations
    /// that need GET support (e.g. polling endpoints).
    async fn get_json(
        &self,
        url: &str,
        headers: &[(String, String)],
        timeout_ms: u64,
    ) -> Result<String, HttpPostError> {
        let _ = (url, headers, timeout_ms);
        Err(HttpPostError::transport(
            "get_json not supported by this poster",
        ))
    }

    /// Perform a DELETE request to `url` with `headers`.
    ///
    /// The default implementation returns an error. Override in implementations
    /// that need DELETE support.
    async fn delete_json(
        &self,
        url: &str,
        headers: &[(String, String)],
        timeout_ms: u64,
    ) -> Result<String, HttpPostError> {
        let _ = (url, headers, timeout_ms);
        Err(HttpPostError::transport(
            "delete_json not supported by this poster",
        ))
    }
}

/// Production [`HttpPoster`] backed by `reqwest`.
#[derive(Debug, Default)]
pub struct ReqwestPoster {
    client: reqwest::Client,
}

impl ReqwestPoster {
    /// Build a new poster backed by the shared `reqwest::Client`.
    ///
    /// This reuses the process-wide connection pool so agent instances do not
    /// pay a fresh TCP+TLS handshake for every new backend.
    #[must_use]
    pub fn new() -> Self {
        Self {
            client: crate::provider::shared_http_client(),
        }
    }

    /// Wrap an existing `reqwest::Client` (useful when pooling is desired).
    #[must_use]
    pub const fn with_client(client: reqwest::Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl HttpPoster for ReqwestPoster {
    async fn post_json(
        &self,
        url: &str,
        headers: &[(String, String)],
        body: &[u8],
        timeout_ms: u64,
    ) -> Result<String, HttpPostError> {
        let mut req = self
            .client
            .post(url)
            .timeout(Duration::from_millis(timeout_ms));
        for (k, v) in headers {
            req = req.header(k.as_str(), v.as_str());
        }
        let resp = req
            .body(body.to_vec())
            .send()
            .await
            .map_err(|e| HttpPostError::transport(format!("request failed: {e}")))?;
        let status = resp.status();
        let retry_after = extract_retry_after_from_response(&resp);
        let text = resp
            .text()
            .await
            .map_err(|e| HttpPostError::transport(format!("read body failed: {e}")))?;
        if status.is_success() {
            Ok(text)
        } else {
            Err(HttpPostError::http_with_retry_after(
                status.as_u16(),
                text,
                retry_after,
            ))
        }
    }

    async fn get_json(
        &self,
        url: &str,
        headers: &[(String, String)],
        timeout_ms: u64,
    ) -> Result<String, HttpPostError> {
        let mut req = self
            .client
            .get(url)
            .timeout(Duration::from_millis(timeout_ms));
        for (k, v) in headers {
            req = req.header(k.as_str(), v.as_str());
        }
        let resp = req
            .send()
            .await
            .map_err(|e| HttpPostError::transport(format!("request failed: {e}")))?;
        let status = resp.status();
        let retry_after = extract_retry_after_from_response(&resp);
        let text = resp
            .text()
            .await
            .map_err(|e| HttpPostError::transport(format!("read body failed: {e}")))?;
        if status.is_success() {
            Ok(text)
        } else {
            Err(HttpPostError::http_with_retry_after(
                status.as_u16(),
                text,
                retry_after,
            ))
        }
    }

    async fn delete_json(
        &self,
        url: &str,
        headers: &[(String, String)],
        timeout_ms: u64,
    ) -> Result<String, HttpPostError> {
        let mut req = self
            .client
            .delete(url)
            .timeout(Duration::from_millis(timeout_ms));
        for (k, v) in headers {
            req = req.header(k.as_str(), v.as_str());
        }
        let resp = req
            .send()
            .await
            .map_err(|e| HttpPostError::transport(format!("request failed: {e}")))?;
        let status = resp.status();
        let retry_after = extract_retry_after_from_response(&resp);
        let text = resp
            .text()
            .await
            .map_err(|e| HttpPostError::transport(format!("read body failed: {e}")))?;
        if status.is_success() {
            Ok(text)
        } else {
            Err(HttpPostError::http_with_retry_after(
                status.as_u16(),
                text,
                retry_after,
            ))
        }
    }
}

/// Create a shared HTTP poster for the process.
///
/// Call this once at startup and clone the returned [`Arc`] wherever HTTP
/// dispatch is needed.
#[must_use]
pub fn shared_http_client() -> Arc<ReqwestPoster> {
    Arc::new(ReqwestPoster::new())
}

/// Create a shared HTTP poster from an existing `reqwest::Client`.
#[must_use]
pub fn shared_http_client_from(client: reqwest::Client) -> Arc<ReqwestPoster> {
    Arc::new(ReqwestPoster::with_client(client))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transport_error_has_no_status() {
        let e = HttpPostError::transport("dns");
        assert!(e.status.is_none());
        assert_eq!(e.message, "dns");
        assert!(e.retry_after_secs.is_none());
        assert_eq!(e.to_string(), "transport error: dns");
    }

    #[test]
    fn http_error_carries_status() {
        let e = HttpPostError::http(429, "rate limited");
        assert_eq!(e.status, Some(429));
        assert!(e.retry_after_secs.is_none());
        assert_eq!(e.to_string(), "http 429: rate limited");
    }

    #[test]
    fn http_error_with_retry_after() {
        let e = HttpPostError::http_with_retry_after(429, "rate limited", Some(30));
        assert_eq!(e.status, Some(429));
        assert_eq!(e.retry_after_secs, Some(30));
    }

    #[test]
    fn retry_after_parse_integer_seconds() {
        assert_eq!(parse_retry_after("30"), Some(30));
        assert_eq!(parse_retry_after("0"), Some(0));
        assert_eq!(parse_retry_after("120"), Some(120));
    }

    #[test]
    fn retry_after_parse_empty_and_invalid() {
        assert_eq!(parse_retry_after(""), None);
        assert_eq!(parse_retry_after("   "), None);
        assert_eq!(parse_retry_after("not-a-number"), None);
    }

    #[test]
    fn retry_after_parse_http_date() {
        let future_date = "Sat, 06 Nov 2094 08:49:37 GMT";
        let result = parse_retry_after(future_date);
        assert!(result.is_some(), "should parse valid future HTTP date");
        assert!(
            result.unwrap() > 0,
            "future date should produce positive seconds"
        );
    }

    #[test]
    fn reqwest_poster_is_constructible() {
        let _p = ReqwestPoster::new();
        let _d = ReqwestPoster::default();
    }
}
