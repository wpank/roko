//! Centralized error classification for provider adapters.
//!
//! Before this module, each of the 8+ provider adapters hand-rolled its own
//! string matching for rate limits, auth failures, timeouts, etc. The two
//! public entry points here cover the two transport families:
//!
//! - [`classify_cli_error`] — CLI subprocess adapters (body carries stderr text)
//! - [`classify_http_status`] — HTTP API adapters (status code + JSON body)
//!
//! Each adapter can still layer provider-specific checks (e.g. Anthropic
//! `content_policy_violation`, OpenAI Z.AI error codes) *before* calling these
//! helpers.

use serde_json::Value;

use super::ProviderError;

/// Hints for where to find `retry_after` in the HTTP response body.
///
/// Different providers place the retry-after value in different JSON paths.
/// Pass the appropriate variant so that [`classify_http_status`] can extract it
/// without each adapter duplicating the pointer logic.
#[derive(Debug, Clone, Copy, Default)]
pub enum RetryAfterSource {
    /// Anthropic: `/retry_after` (seconds as integer).
    #[default]
    BodyRetryAfter,
    /// OpenAI-compat / Cursor ACP: `/retry_after` (seconds as integer).
    /// Same JSON path as Anthropic, kept as a separate variant for clarity.
    BodyRetryAfterCompat,
    /// Cerebras: `/error/retry_after` (seconds as float).
    ErrorRetryAfter,
    /// Gemini: `/error/details[*]/retryDelay` (string like "30s").
    ErrorDetailsRetryDelay,
    /// No body-based retry-after; only HTTP headers would carry it.
    None,
}

/// Classify a CLI subprocess error from its exit code (passed as `status`) and
/// stderr text (passed as the JSON `body`, typically a string value or an
/// object with `/error` or `/message` fields).
///
/// The `cli_label` is used in the fallback `Other` message (e.g. "CLI",
/// "gemini CLI", "OpenClaw").
pub fn classify_cli_error(status: u16, body: &Value, cli_label: &str) -> ProviderError {
    let stderr = body
        .as_str()
        .or_else(|| body.pointer("/error").and_then(Value::as_str))
        .or_else(|| body.pointer("/message").and_then(Value::as_str))
        .unwrap_or("");
    let lower = stderr.to_ascii_lowercase();

    // --- Text-based classification (most specific first) ---

    if lower.contains("rate limit") || lower.contains("quota") {
        return ProviderError::RateLimit {
            retry_after_ms: None,
        };
    }
    if lower.contains("unauthorized")
        || lower.contains("permission denied")
        || lower.contains("unauthenticated")
        || lower.contains("sign in")
        || lower.contains("not logged in")
    {
        return ProviderError::AuthFailure;
    }
    if lower.contains("timed out") || lower.contains("timeout") {
        return ProviderError::Timeout;
    }
    if lower.contains("content policy")
        || lower.contains("content_policy")
        || lower.contains("content filter")
    {
        return ProviderError::ContentPolicy;
    }
    if lower.contains("context window")
        || lower.contains("context length")
        || lower.contains("token limit")
    {
        return ProviderError::ContextOverflow;
    }
    if lower.contains("model not found") || lower.contains("unknown model") {
        return ProviderError::ModelNotFound;
    }

    // --- Status-code fallback ---

    match status {
        429 => ProviderError::RateLimit {
            retry_after_ms: None,
        },
        401 | 403 => ProviderError::AuthFailure,
        404 => ProviderError::ModelNotFound,
        408 => ProviderError::Timeout,
        500..=599 => ProviderError::ServerError(status),
        _ => {
            if stderr.is_empty() {
                ProviderError::Other(format!("{cli_label} exit status {status}"))
            } else {
                ProviderError::Other(stderr.to_string())
            }
        }
    }
}

/// Classify an HTTP API error from its status code and JSON response body.
///
/// `source` tells the function where to look for a `retry_after` value inside
/// the JSON body. Each provider places it in a different path.
pub fn classify_http_status(status: u16, body: &Value, source: RetryAfterSource) -> ProviderError {
    match status {
        429 | 529 => ProviderError::RateLimit {
            retry_after_ms: extract_retry_after(body, source),
        },
        401 | 403 => ProviderError::AuthFailure,
        404 => ProviderError::ModelNotFound,
        408 | 504 => ProviderError::Timeout,
        400 => classify_bad_request(body),
        500..=599 => ProviderError::ServerError(status),
        _ => ProviderError::Other(format!("HTTP {status}")),
    }
}

/// Extract `retry_after` in milliseconds from the JSON body using the
/// provider-specific path indicated by `source`.
fn extract_retry_after(body: &Value, source: RetryAfterSource) -> Option<u64> {
    match source {
        RetryAfterSource::BodyRetryAfter | RetryAfterSource::BodyRetryAfterCompat => body
            .pointer("/retry_after")
            .and_then(|v| v.as_u64())
            .map(|seconds| seconds * 1000),
        RetryAfterSource::ErrorRetryAfter => body
            .pointer("/error/retry_after")
            .and_then(|v| v.as_f64())
            .map(|secs| (secs * 1000.0) as u64),
        RetryAfterSource::ErrorDetailsRetryDelay => body
            .pointer("/error/details")
            .and_then(Value::as_array)
            .and_then(|details| {
                details.iter().find_map(|d| {
                    d.get("retryDelay")
                        .and_then(Value::as_str)
                        .and_then(parse_duration_str)
                })
            }),
        RetryAfterSource::None => None,
    }
}

/// Parse a duration string like `"30s"` or `"1.5s"` into milliseconds.
fn parse_duration_str(s: &str) -> Option<u64> {
    let s = s.trim();
    if let Some(secs_str) = s.strip_suffix('s') {
        secs_str
            .trim()
            .parse::<f64>()
            .ok()
            .map(|s| (s * 1000.0) as u64)
    } else {
        // Try bare number as seconds.
        s.parse::<f64>().ok().map(|s| (s * 1000.0) as u64)
    }
}

/// Classify HTTP 400 (Bad Request) — usually a context overflow signal.
fn classify_bad_request(body: &Value) -> ProviderError {
    let msg = body
        .pointer("/error/message")
        .and_then(Value::as_str)
        .unwrap_or("");
    if msg.contains("context_length_exceeded")
        || msg.contains("maximum context length")
        || (msg.contains("context") && (msg.contains("token") || msg.contains("length")))
    {
        ProviderError::ContextOverflow
    } else {
        ProviderError::Other(format!("HTTP 400: {msg}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ── CLI classification ──────────────────────────────────────────

    #[test]
    fn cli_rate_limit_from_stderr() {
        let err = classify_cli_error(1, &json!("rate limit exceeded"), "CLI");
        assert!(matches!(
            err,
            ProviderError::RateLimit {
                retry_after_ms: None
            }
        ));
    }

    #[test]
    fn cli_quota_from_stderr() {
        let err = classify_cli_error(1, &json!("quota exceeded"), "CLI");
        assert!(matches!(
            err,
            ProviderError::RateLimit {
                retry_after_ms: None
            }
        ));
    }

    #[test]
    fn cli_auth_failure_from_stderr() {
        let err = classify_cli_error(1, &json!("unauthorized access"), "CLI");
        assert!(matches!(err, ProviderError::AuthFailure));
    }

    #[test]
    fn cli_auth_failure_unauthenticated() {
        let err = classify_cli_error(1, &json!("unauthenticated request"), "CLI");
        assert!(matches!(err, ProviderError::AuthFailure));
    }

    #[test]
    fn cli_auth_failure_sign_in() {
        let err = classify_cli_error(1, &json!("please sign in first"), "CLI");
        assert!(matches!(err, ProviderError::AuthFailure));
    }

    #[test]
    fn cli_timeout_from_stderr() {
        let err = classify_cli_error(1, &json!("request timed out"), "CLI");
        assert!(matches!(err, ProviderError::Timeout));
    }

    #[test]
    fn cli_context_overflow_from_stderr() {
        let err = classify_cli_error(1, &json!("context window exceeded"), "CLI");
        assert!(matches!(err, ProviderError::ContextOverflow));
    }

    #[test]
    fn cli_token_limit_from_stderr() {
        let err = classify_cli_error(1, &json!("token limit reached"), "CLI");
        assert!(matches!(err, ProviderError::ContextOverflow));
    }

    #[test]
    fn cli_model_not_found_from_stderr() {
        let err = classify_cli_error(1, &json!("model not found"), "CLI");
        assert!(matches!(err, ProviderError::ModelNotFound));
    }

    #[test]
    fn cli_content_policy_from_stderr() {
        let err = classify_cli_error(1, &json!("content policy violation"), "CLI");
        assert!(matches!(err, ProviderError::ContentPolicy));
    }

    #[test]
    fn cli_rate_limit_from_status_429() {
        let err = classify_cli_error(429, &json!(null), "CLI");
        assert!(matches!(
            err,
            ProviderError::RateLimit {
                retry_after_ms: None
            }
        ));
    }

    #[test]
    fn cli_auth_from_status_401() {
        let err = classify_cli_error(401, &json!(null), "CLI");
        assert!(matches!(err, ProviderError::AuthFailure));
    }

    #[test]
    fn cli_server_error_from_status() {
        let err = classify_cli_error(502, &json!(null), "CLI");
        assert!(matches!(err, ProviderError::ServerError(502)));
    }

    #[test]
    fn cli_fallback_label() {
        let err = classify_cli_error(999, &json!(null), "gemini CLI");
        match err {
            ProviderError::Other(msg) => assert!(msg.contains("gemini CLI"), "got: {msg}"),
            other => panic!("expected Other, got {other:?}"),
        }
    }

    #[test]
    fn cli_reads_nested_error_field() {
        let body = json!({ "error": "rate limit hit" });
        let err = classify_cli_error(1, &body, "CLI");
        assert!(matches!(err, ProviderError::RateLimit { .. }));
    }

    #[test]
    fn cli_reads_nested_message_field() {
        let body = json!({ "message": "unauthorized" });
        let err = classify_cli_error(1, &body, "CLI");
        assert!(matches!(err, ProviderError::AuthFailure));
    }

    // ── HTTP classification ─────────────────────────────────────────

    #[test]
    fn http_429_with_body_retry_after() {
        let body = json!({ "retry_after": 30 });
        let err = classify_http_status(429, &body, RetryAfterSource::BodyRetryAfter);
        match err {
            ProviderError::RateLimit {
                retry_after_ms: Some(ms),
            } => assert_eq!(ms, 30_000),
            other => panic!("expected RateLimit(30_000), got {other:?}"),
        }
    }

    #[test]
    fn http_429_no_retry_after() {
        let err = classify_http_status(429, &json!(null), RetryAfterSource::None);
        assert!(matches!(
            err,
            ProviderError::RateLimit {
                retry_after_ms: None
            }
        ));
    }

    #[test]
    fn http_529_overload_treated_as_rate_limit() {
        let body = json!({ "retry_after": 10 });
        let err = classify_http_status(529, &body, RetryAfterSource::BodyRetryAfter);
        match err {
            ProviderError::RateLimit {
                retry_after_ms: Some(ms),
            } => assert_eq!(ms, 10_000),
            other => panic!("expected RateLimit(10_000), got {other:?}"),
        }
    }

    #[test]
    fn http_401_auth_failure() {
        let err = classify_http_status(401, &json!(null), RetryAfterSource::None);
        assert!(matches!(err, ProviderError::AuthFailure));
    }

    #[test]
    fn http_404_model_not_found() {
        let err = classify_http_status(404, &json!(null), RetryAfterSource::None);
        assert!(matches!(err, ProviderError::ModelNotFound));
    }

    #[test]
    fn http_408_timeout() {
        let err = classify_http_status(408, &json!(null), RetryAfterSource::None);
        assert!(matches!(err, ProviderError::Timeout));
    }

    #[test]
    fn http_504_timeout() {
        let err = classify_http_status(504, &json!(null), RetryAfterSource::None);
        assert!(matches!(err, ProviderError::Timeout));
    }

    #[test]
    fn http_400_context_overflow() {
        let body = json!({ "error": { "message": "context_length_exceeded" } });
        let err = classify_http_status(400, &body, RetryAfterSource::None);
        assert!(matches!(err, ProviderError::ContextOverflow));
    }

    #[test]
    fn http_400_generic() {
        let body = json!({ "error": { "message": "bad input" } });
        let err = classify_http_status(400, &body, RetryAfterSource::None);
        match err {
            ProviderError::Other(msg) => assert!(msg.contains("bad input"), "got: {msg}"),
            other => panic!("expected Other, got {other:?}"),
        }
    }

    #[test]
    fn http_500_server_error() {
        let err = classify_http_status(503, &json!(null), RetryAfterSource::None);
        assert!(matches!(err, ProviderError::ServerError(503)));
    }

    // ── Retry-after extraction ──────────────────────────────────────

    #[test]
    fn extract_cerebras_error_retry_after() {
        let body = json!({ "error": { "retry_after": 2.5 } });
        let err = classify_http_status(429, &body, RetryAfterSource::ErrorRetryAfter);
        match err {
            ProviderError::RateLimit {
                retry_after_ms: Some(ms),
            } => assert_eq!(ms, 2500),
            other => panic!("expected RateLimit(2500), got {other:?}"),
        }
    }

    #[test]
    fn extract_gemini_retry_delay() {
        let body = json!({ "error": { "details": [{ "retryDelay": "30s" }] } });
        let err = classify_http_status(429, &body, RetryAfterSource::ErrorDetailsRetryDelay);
        match err {
            ProviderError::RateLimit {
                retry_after_ms: Some(ms),
            } => assert_eq!(ms, 30_000),
            other => panic!("expected RateLimit(30_000), got {other:?}"),
        }
    }

    #[test]
    fn parse_duration_str_seconds() {
        assert_eq!(parse_duration_str("30s"), Some(30_000));
        assert_eq!(parse_duration_str("1.5s"), Some(1500));
        assert_eq!(parse_duration_str("0.5"), Some(500));
    }
}
