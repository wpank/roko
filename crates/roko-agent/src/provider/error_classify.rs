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

    // Billing/credit errors must be checked before generic rate-limit detection
    // so that messages containing "quota" + billing indicators are not
    // misclassified as transient rate limits.
    if is_billing_message(&lower) {
        return ProviderError::InsufficientCredits;
    }

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
    // Extract the error message once so billing checks can inspect it for
    // every status code that may carry billing-specific payloads.
    let error_msg = body
        .pointer("/error/message")
        .and_then(Value::as_str)
        .unwrap_or("");
    let error_msg_lower = error_msg.to_ascii_lowercase();

    match status {
        // HTTP 402 is always a billing/payment error.
        402 => ProviderError::InsufficientCredits,

        // 429: distinguish billing-quota exhaustion from transient rate limits.
        429 | 529 => {
            if is_billing_message(&error_msg_lower) {
                ProviderError::InsufficientCredits
            } else {
                ProviderError::RateLimit {
                    retry_after_ms: extract_retry_after(body, source),
                }
            }
        }

        // 403: billing issues vs auth/permission failures.
        401 => ProviderError::AuthFailure,
        403 => {
            if is_billing_message(&error_msg_lower) {
                ProviderError::InsufficientCredits
            } else {
                ProviderError::AuthFailure
            }
        }

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

/// Returns `true` when the lowercased error message indicates a
/// billing/credit/quota exhaustion problem rather than a transient rate limit.
///
/// The patterns cover the major providers:
/// - OpenAI: `"insufficient_quota"`, `"billing_not_active"`
/// - Anthropic: `"credit balance"`, `"account_deactivated"`
/// - Generic: HTTP 402, `"payment required"`, `"quota exceeded"`, etc.
pub fn is_billing_message(lower: &str) -> bool {
    // Exact well-known error codes / phrases.
    if lower.contains("insufficient_quota")
        || lower.contains("insufficient quota")
        || lower.contains("insufficient funds")
        || lower.contains("insufficient credits")
        || lower.contains("billing_not_active")
        || lower.contains("billing not active")
        || lower.contains("account_deactivated")
        || lower.contains("account deactivated")
        || lower.contains("payment required")
    {
        return true;
    }

    // Composite checks: "quota" near an exhaustion verb, or "billing" near an
    // error indicator, or "credit" near "balance".
    if lower.contains("quota")
        && (lower.contains("exceeded") || lower.contains("exhausted") || lower.contains("limit"))
        && !lower.contains("rate limit")
    {
        return true;
    }
    if lower.contains("billing")
        && (lower.contains("error") || lower.contains("issue") || lower.contains("disabled"))
    {
        return true;
    }
    if lower.contains("credit") && lower.contains("balance") {
        return true;
    }

    false
}

/// Classify HTTP 400 (Bad Request) — usually a context overflow signal.
fn classify_bad_request(body: &Value) -> ProviderError {
    let msg = body
        .pointer("/error/message")
        .and_then(Value::as_str)
        .unwrap_or("");
    let lower = msg.to_ascii_lowercase();
    if lower.contains("context_length_exceeded")
        || lower.contains("maximum context length")
        || lower.contains("token limit")
        || (lower.contains("context") && (lower.contains("token") || lower.contains("length")))
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
    fn cli_quota_exceeded_from_stderr_is_billing() {
        // "quota exceeded" is a billing/credit issue, not a transient rate
        // limit. The classification was updated to distinguish these.
        let err = classify_cli_error(1, &json!("quota exceeded"), "CLI");
        assert!(
            matches!(err, ProviderError::InsufficientCredits),
            "expected InsufficientCredits for 'quota exceeded', got {err:?}"
        );
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

    // ── Billing/credit error classification ────────────────────────────

    #[test]
    fn http_402_is_insufficient_credits() {
        let err = classify_http_status(402, &json!(null), RetryAfterSource::None);
        assert!(
            matches!(err, ProviderError::InsufficientCredits),
            "expected InsufficientCredits for 402, got {err:?}"
        );
    }

    #[test]
    fn http_429_with_billing_message_is_insufficient_credits() {
        let body = json!({ "error": { "message": "insufficient_quota: you have exceeded your billing quota" } });
        let err = classify_http_status(429, &body, RetryAfterSource::None);
        assert!(
            matches!(err, ProviderError::InsufficientCredits),
            "expected InsufficientCredits for 429+billing, got {err:?}"
        );
    }

    #[test]
    fn http_429_plain_rate_limit_still_works() {
        let body = json!({ "error": { "message": "too many requests" } });
        let err = classify_http_status(429, &body, RetryAfterSource::None);
        assert!(
            matches!(err, ProviderError::RateLimit { .. }),
            "expected RateLimit for plain 429, got {err:?}"
        );
    }

    #[test]
    fn http_403_with_billing_disabled_is_insufficient_credits() {
        let body = json!({ "error": { "message": "billing not active on this account" } });
        let err = classify_http_status(403, &body, RetryAfterSource::None);
        assert!(
            matches!(err, ProviderError::InsufficientCredits),
            "expected InsufficientCredits for 403+billing, got {err:?}"
        );
    }

    #[test]
    fn http_403_plain_auth_still_works() {
        let body = json!({ "error": { "message": "forbidden" } });
        let err = classify_http_status(403, &body, RetryAfterSource::None);
        assert!(
            matches!(err, ProviderError::AuthFailure),
            "expected AuthFailure for plain 403, got {err:?}"
        );
    }

    #[test]
    fn cli_insufficient_quota_from_stderr() {
        let err = classify_cli_error(1, &json!("insufficient_quota"), "CLI");
        assert!(
            matches!(err, ProviderError::InsufficientCredits),
            "expected InsufficientCredits, got {err:?}"
        );
    }

    #[test]
    fn cli_payment_required_from_stderr() {
        let err = classify_cli_error(1, &json!("402 payment required"), "CLI");
        assert!(
            matches!(err, ProviderError::InsufficientCredits),
            "expected InsufficientCredits, got {err:?}"
        );
    }

    #[test]
    fn cli_account_deactivated_from_stderr() {
        let err = classify_cli_error(1, &json!("account_deactivated"), "CLI");
        assert!(
            matches!(err, ProviderError::InsufficientCredits),
            "expected InsufficientCredits, got {err:?}"
        );
    }

    #[test]
    fn is_billing_message_detects_known_patterns() {
        assert!(is_billing_message("insufficient_quota"));
        assert!(is_billing_message("you have insufficient credits"));
        assert!(is_billing_message("billing_not_active"));
        assert!(is_billing_message("account_deactivated"));
        assert!(is_billing_message("account deactivated"));
        assert!(is_billing_message("402 payment required"));
        assert!(is_billing_message("your quota exceeded the plan limit"));
        assert!(is_billing_message("credit balance is zero"));
        assert!(is_billing_message("billing error on your account"));
    }

    #[test]
    fn is_billing_message_does_not_trigger_on_rate_limit() {
        assert!(!is_billing_message("rate limit exceeded"));
        assert!(!is_billing_message("too many requests"));
        assert!(!is_billing_message("try again later"));
    }
}
