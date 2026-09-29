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
//!
//! [`detect_provider_exhaustion`] recognises subscription/usage-window
//! refusals (Claude CLI "You've hit your session limit · resets 4pm", Codex
//! "You've hit your usage limit", 429s with an hour-long `Retry-After`) in any
//! provider's text, so callers can quarantine the provider until its window
//! resets instead of retrying it.

use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, NaiveTime, TimeZone};
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

    // A usage-window refusal mentions "limit" and sometimes "quota"; it must
    // win over both billing and rate-limit detection so the reset time is kept.
    if let Some(exhaustion) = detect_provider_exhaustion(stderr) {
        return exhaustion.into_error();
    }

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

        // 429: distinguish billing-quota and usage-window exhaustion from
        // transient rate limits.
        429 | 529 => classify_rate_limited(error_msg, extract_retry_after(body, source)),

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

/// Classify a 429/529 response from its error message and `Retry-After`.
///
/// Billing text wins, then usage-window exhaustion (named in the message, or a
/// `Retry-After` of at least [`EXHAUSTION_RETRY_AFTER_MS`]); anything else is a
/// transient rate limit that is worth waiting out.
#[must_use]
pub fn classify_rate_limited(message: &str, retry_after_ms: Option<u64>) -> ProviderError {
    if is_billing_message(&message.to_ascii_lowercase()) {
        return ProviderError::InsufficientCredits;
    }
    if let Some(exhaustion) = detect_provider_exhaustion(message) {
        return exhaustion.into_error();
    }
    match retry_after_ms {
        Some(delay_ms) if delay_ms >= EXHAUSTION_RETRY_AFTER_MS => {
            let detail = humanize_line(message);
            let detail = if detail.is_empty() {
                "rate limited".to_string()
            } else {
                bounded_message(&detail)
            };
            ProviderError::ProviderExhausted {
                resets_at_ms: Some(
                    Local::now()
                        .timestamp_millis()
                        .saturating_add(i64::try_from(delay_ms).unwrap_or(i64::MAX)),
                ),
                message: format!("{detail} (retry after {}s)", delay_ms / 1_000),
            }
        }
        retry_after_ms => ProviderError::RateLimit { retry_after_ms },
    }
}

// ---- Usage-window exhaustion ---------------------------------------------

/// Prefix rendered by [`ProviderError::ProviderExhausted`], so an error that
/// has already passed through `Display` is still recognised downstream.
pub const PROVIDER_EXHAUSTED_MARKER: &str = "provider usage exhausted";

/// A `Retry-After` at or above this is a usage window rather than a transient
/// rate limit: sleeping it out inside one agent turn would stall the task.
pub const EXHAUSTION_RETRY_AFTER_MS: u64 = 15 * 60 * 1_000;

/// Furthest a parsed reset time may lie ahead. Weekly plans reset within
/// seven days; anything later is a misparse and falls back to a cooldown.
const MAX_RESET_HORIZON_HOURS: i64 = 8 * 24;

/// Longest tail of the provider's message kept in an exhaustion error.
const EXHAUSTION_MESSAGE_MAX_CHARS: usize = 240;

/// A provider refused work because a subscription or usage window is used up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderExhaustion {
    /// When the provider says the window resets (unix ms), if parseable.
    pub resets_at_ms: Option<i64>,
    /// The provider's own words: the matching line, bounded to its tail.
    pub message: String,
}

impl ProviderExhaustion {
    /// Convert into the typed provider error.
    #[must_use]
    pub fn into_error(self) -> ProviderError {
        ProviderError::ProviderExhausted {
            resets_at_ms: self.resets_at_ms,
            message: self.message,
        }
    }
}

/// Detect a subscription/usage-window refusal in provider output.
///
/// Recognises the Claude CLI ("You've hit your session limit · resets 4pm
/// (Europe/Berlin)", "Claude AI usage limit reached|<epoch>", "5-hour limit
/// reached ∙ resets 3pm"), Codex ("You've hit your usage limit … try again in
/// 2 hours"), and text already rendered from
/// [`ProviderError::ProviderExhausted`]. Transient rate limits, billing, and
/// context-window errors are deliberately not matched.
///
/// Wall-clock reset times are read in the machine's local zone, which is the
/// zone the CLIs print them in.
#[must_use]
pub fn detect_provider_exhaustion(text: &str) -> Option<ProviderExhaustion> {
    detect_provider_exhaustion_at(text, Local::now())
}

fn detect_provider_exhaustion_at(text: &str, now: DateTime<Local>) -> Option<ProviderExhaustion> {
    let line = text
        .lines()
        .map(humanize_line)
        .find(|line| is_exhaustion_text(&normalize(line)))?;
    let resets_at = parse_reset_time(&normalize(&line), now)
        .or_else(|| parse_reset_time(&normalize(text), now))
        .filter(|at| *at > now && *at - now <= Duration::hours(MAX_RESET_HORIZON_HOURS));
    Some(ProviderExhaustion {
        resets_at_ms: resets_at.map(|at| at.timestamp_millis()),
        message: bounded_message(&line),
    })
}

fn is_exhaustion_text(lower: &str) -> bool {
    lower.contains(PROVIDER_EXHAUSTED_MARKER)
        || (lower.contains("hit your") && lower.contains("limit"))
        || lower.contains("usage limit reached")
        || lower.contains("usage_limit_reached")
        || lower.contains("usage limit exceeded")
        || lower.contains("usage_limit_exceeded")
        || (lower.contains("limit reached") && lower.contains("resets"))
}

/// Lowercase and straighten typographic apostrophes ("You’ve" → "you've").
fn normalize(text: &str) -> String {
    text.to_lowercase().replace(['\u{2018}', '\u{2019}'], "'")
}

/// Pull the human message out of a JSON event line; other lines pass through.
fn humanize_line(line: &str) -> String {
    let trimmed = line.trim();
    if trimmed.starts_with('{')
        && let Ok(value) = serde_json::from_str::<Value>(trimmed)
    {
        for pointer in ["/result", "/error/message", "/message", "/error"] {
            if let Some(text) = value.pointer(pointer).and_then(Value::as_str) {
                return text.trim().to_string();
            }
        }
    }
    trimmed.to_string()
}

fn bounded_message(message: &str) -> String {
    let message = message.trim();
    let count = message.chars().count();
    if count <= EXHAUSTION_MESSAGE_MAX_CHARS {
        return message.to_string();
    }
    let tail: String = message
        .chars()
        .skip(count - EXHAUSTION_MESSAGE_MAX_CHARS)
        .collect();
    format!("…{tail}")
}

/// Text following the first occurrence of `marker`.
fn after<'a>(text: &'a str, marker: &str) -> Option<&'a str> {
    text.find(marker).map(|index| &text[index + marker.len()..])
}

/// Parse the reset time out of a normalized exhaustion message.
fn parse_reset_time(lower: &str, now: DateTime<Local>) -> Option<DateTime<Local>> {
    if let Some(rest) = after(lower, "resets at ")
        && let Some(token) = rest.split(|c: char| c.is_whitespace() || c == ')').next()
        && let Ok(at) = DateTime::parse_from_rfc3339(&token.to_ascii_uppercase())
    {
        return Some(at.with_timezone(&Local));
    }
    // Older Claude CLI: "Claude AI usage limit reached|1759075200".
    if let Some(rest) = after(lower, "limit reached|") {
        let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
        let value = digits.parse::<i64>().ok()?;
        let millis = if value > 100_000_000_000 {
            value
        } else {
            value.saturating_mul(1_000)
        };
        return Local.timestamp_millis_opt(millis).single();
    }
    for marker in [
        "retry after ",
        "retry-after: ",
        "try again in ",
        "resets in ",
        "reset in ",
    ] {
        if let Some(duration) = after(lower, marker).and_then(parse_duration_phrase) {
            return now.checked_add_signed(duration);
        }
    }
    for marker in [
        "resets ",
        "reset at ",
        "try again at ",
        "available again at ",
    ] {
        if let Some(at) = after(lower, marker).and_then(|rest| parse_wall_clock(rest, now)) {
            return Some(at);
        }
    }
    None
}

/// Parse "4 days 21 hours 3 minutes", "2h 30m", "3600s", or bare seconds.
fn parse_duration_phrase(rest: &str) -> Option<Duration> {
    let mut tokens = rest
        .split(|c: char| c.is_whitespace() || c == ',')
        .map(|token| token.trim_end_matches(['.', ')', ';', ':']))
        .filter(|token| !token.is_empty())
        .peekable();
    let mut total = Duration::zero();
    let mut matched = false;
    while let Some(token) = tokens.next() {
        let digits: String = token.chars().take_while(char::is_ascii_digit).collect();
        if digits.is_empty() {
            if matched && token == "and" {
                continue;
            }
            break;
        }
        let value = digits.parse::<i64>().ok()?;
        let unit = match &token[digits.len()..] {
            "" => match tokens
                .peek()
                .copied()
                .filter(|next| unit_seconds(next).is_some())
            {
                Some(next) => {
                    tokens.next();
                    next
                }
                None => "s",
            },
            unit => unit,
        };
        let seconds = value.checked_mul(unit_seconds(unit)?)?;
        total = total.checked_add(&Duration::try_seconds(seconds)?)?;
        matched = true;
    }
    matched.then_some(total)
}

fn unit_seconds(unit: &str) -> Option<i64> {
    match unit {
        "d" | "day" | "days" => Some(86_400),
        "h" | "hr" | "hrs" | "hour" | "hours" => Some(3_600),
        "m" | "min" | "mins" | "minute" | "minutes" => Some(60),
        "s" | "sec" | "secs" | "second" | "seconds" => Some(1),
        _ => None,
    }
}

/// Parse "4pm (europe/berlin)", "4:30 pm", "16:00", "oct 3, 9am", or
/// "oct 3 at 9am" as the next such local time after `now`.
fn parse_wall_clock(rest: &str, now: DateTime<Local>) -> Option<DateTime<Local>> {
    let mut tokens = rest
        .split(|c: char| c.is_whitespace() || c == ',')
        .map(|token| token.trim_matches(['(', ')', '.', ';']))
        .filter(|token| !token.is_empty())
        .peekable();
    if tokens.peek() == Some(&"at") {
        tokens.next();
    }
    let mut month_day = None;
    if let Some(month) = tokens.peek().and_then(|token| month_number(token)) {
        tokens.next();
        let day = tokens.next()?;
        let day = day
            .trim_end_matches(|c: char| c.is_ascii_alphabetic())
            .parse::<u32>()
            .ok()?;
        month_day = Some((month, day));
        if tokens.peek() == Some(&"at") {
            tokens.next();
        }
    }
    let mut clock = tokens.next()?.replace('.', "");
    if let Some(next) = tokens.peek()
        && matches!(next.replace('.', "").as_str(), "am" | "pm")
    {
        clock.push_str(&next.replace('.', ""));
    }
    let (digits, pm) = if let Some(digits) = clock.strip_suffix("am") {
        (digits, Some(false))
    } else if let Some(digits) = clock.strip_suffix("pm") {
        (digits, Some(true))
    } else {
        (clock.as_str(), None)
    };
    let (hour, minute) = match digits.split_once(':') {
        Some((hour, minute)) => (hour.parse::<u32>().ok()?, minute.parse::<u32>().ok()?),
        // A bare "4" is ambiguous; only accept it with am/pm.
        None if pm.is_some() => (digits.parse::<u32>().ok()?, 0),
        None => return None,
    };
    let hour = match pm {
        Some(_) if hour == 0 || hour > 12 => return None,
        Some(pm) => hour % 12 + if pm { 12 } else { 0 },
        None => hour,
    };
    let time = NaiveTime::from_hms_opt(hour, minute, 0)?;
    let naive_now = now.naive_local();
    let at = match month_day {
        Some((month, day)) => {
            let this_year = NaiveDate::from_ymd_opt(now.year(), month, day)?.and_time(time);
            if this_year < naive_now - Duration::days(1) {
                NaiveDate::from_ymd_opt(now.year() + 1, month, day)?.and_time(time)
            } else {
                this_year
            }
        }
        None => {
            let today = now.date_naive().and_time(time);
            if today <= naive_now {
                today + Duration::days(1)
            } else {
                today
            }
        }
    };
    Local.from_local_datetime(&at).earliest()
}

fn month_number(token: &str) -> Option<u32> {
    Some(match token {
        "jan" | "january" => 1,
        "feb" | "february" => 2,
        "mar" | "march" => 3,
        "apr" | "april" => 4,
        "may" => 5,
        "jun" | "june" => 6,
        "jul" | "july" => 7,
        "aug" | "august" => 8,
        "sep" | "sept" | "september" => 9,
        "oct" | "october" => 10,
        "nov" | "november" => 11,
        "dec" | "december" => 12,
        _ => return None,
    })
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

// ---- Turn cap ------------------------------------------------------------

/// Prefix of the failure text an adapter emits when an agent run stops at its
/// turn cap, so the dispatcher still recognises it after the text has crossed
/// the `AgentResult` boundary. Deliberately free of "limit"/"resets" wording
/// so [`detect_provider_exhaustion`] never mistakes it for a usage window.
pub const TURN_CAP_MARKER: &str = "agent turn cap reached";

/// An agent run that stopped at its turn cap (Claude CLI `error_max_turns`).
///
/// Not a provider fault: the partial work is in the workspace, and the
/// provider is healthy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TurnCapHit {
    /// Turns the provider reported using, when it said.
    pub num_turns: Option<u32>,
    /// Turn cap the run was started with, when the adapter knew it.
    pub cap: Option<u32>,
}

impl std::fmt::Display for TurnCapHit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let field =
            |value: Option<u32>| value.map_or_else(|| "unknown".to_string(), |n| n.to_string());
        write!(
            f,
            "{TURN_CAP_MARKER} (turns={}, cap={})",
            field(self.num_turns),
            field(self.cap)
        )
    }
}

/// Recognise [`TurnCapHit`] text anywhere in `text`.
#[must_use]
pub fn detect_turn_cap(text: &str) -> Option<TurnCapHit> {
    let rest = after(text, TURN_CAP_MARKER)?;
    let number = |key: &str| {
        after(rest, key).and_then(|tail| {
            tail.chars()
                .take_while(char::is_ascii_digit)
                .collect::<String>()
                .parse()
                .ok()
        })
    };
    Some(TurnCapHit {
        num_turns: number("turns="),
        cap: number("cap="),
    })
}

// ---- Attempt timeout -----------------------------------------------------

/// Failure text a subprocess adapter emits when it kills an agent run at its
/// wall-clock timeout (`"timed out after 600000 ms"`), so the dispatcher can
/// give the retry more time after the text has crossed the `AgentResult`
/// boundary.
pub const ATTEMPT_TIMEOUT_MARKER: &str = "timed out after";

/// Whether `text` reports an agent run killed at its wall-clock timeout.
#[must_use]
pub fn detect_attempt_timeout(text: &str) -> bool {
    text.contains(ATTEMPT_TIMEOUT_MARKER)
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

    // ── Usage-window exhaustion ─────────────────────────────────────

    fn local(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> DateTime<Local> {
        Local
            .with_ymd_and_hms(year, month, day, hour, minute, 0)
            .earliest()
            .expect("valid local time")
    }

    #[test]
    fn claude_session_limit_resets_later_today() {
        let now = local(2026, 9, 26, 10, 0);
        let exhaustion = detect_provider_exhaustion_at(
            "You\u{2019}ve hit your session limit \u{b7} resets 4pm (Europe/Berlin)",
            now,
        )
        .expect("session limit");
        assert_eq!(
            exhaustion.resets_at_ms,
            Some(local(2026, 9, 26, 16, 0).timestamp_millis())
        );
        assert_eq!(
            exhaustion.message,
            "You\u{2019}ve hit your session limit \u{b7} resets 4pm (Europe/Berlin)"
        );
    }

    #[test]
    fn reset_time_already_passed_today_rolls_to_tomorrow() {
        let now = local(2026, 9, 26, 17, 30);
        let exhaustion =
            detect_provider_exhaustion_at("You've hit your session limit · resets 4:30pm", now)
                .expect("session limit");
        assert_eq!(
            exhaustion.resets_at_ms,
            Some(local(2026, 9, 27, 16, 30).timestamp_millis())
        );
    }

    #[test]
    fn claude_limit_variants_are_detected_with_reset_times() {
        let now = local(2026, 9, 26, 10, 0);
        let five_hour = detect_provider_exhaustion_at("5-hour limit reached ∙ resets 3pm", now)
            .expect("5-hour limit");
        assert_eq!(
            five_hour.resets_at_ms,
            Some(local(2026, 9, 26, 15, 0).timestamp_millis())
        );

        let weekly = detect_provider_exhaustion_at(
            "You've hit your weekly limit · resets Sep 29, 9am (Europe/Berlin)",
            now,
        )
        .expect("weekly limit");
        assert_eq!(
            weekly.resets_at_ms,
            Some(local(2026, 9, 29, 9, 0).timestamp_millis())
        );

        let epoch_ms = now.timestamp_millis() + 3_600_000;
        let legacy = detect_provider_exhaustion_at(
            &format!("Claude AI usage limit reached|{}", epoch_ms / 1_000),
            now,
        )
        .expect("legacy limit");
        assert_eq!(legacy.resets_at_ms, Some(epoch_ms));
    }

    #[test]
    fn codex_usage_limit_parses_relative_reset() {
        let now = local(2026, 9, 26, 10, 0);
        let exhaustion = detect_provider_exhaustion_at(
            "You've hit your usage limit. Upgrade to Pro (https://openai.com/chatgpt/pricing) \
             or try again in 1 day 2 hours 3 minutes.",
            now,
        )
        .expect("codex limit");
        let expected = now + Duration::days(1) + Duration::hours(2) + Duration::minutes(3);
        assert_eq!(exhaustion.resets_at_ms, Some(expected.timestamp_millis()));
    }

    #[test]
    fn exhaustion_in_json_result_line_uses_human_message() {
        let now = local(2026, 9, 26, 10, 0);
        let exhaustion = detect_provider_exhaustion_at(
            r#"{"type":"result","is_error":true,"result":"You've hit your session limit · resets 4pm"}"#,
            now,
        )
        .expect("json result");
        assert_eq!(
            exhaustion.message,
            "You've hit your session limit · resets 4pm"
        );
    }

    #[test]
    fn unparseable_reset_keeps_exhaustion_without_time() {
        let exhaustion =
            detect_provider_exhaustion("You've hit your usage limit").expect("usage limit");
        assert_eq!(exhaustion.resets_at_ms, None);
    }

    #[test]
    fn transient_and_unrelated_errors_are_not_exhaustion() {
        for text in [
            "rate limit exceeded",
            "token limit reached",
            "retries exhausted",
            "Budget exhausted",
            "context window exceeded",
            "exit 1: claude failed",
            "Rate limit reached for gpt-4o on tokens per min. Please try again in 1.2s.",
        ] {
            assert!(detect_provider_exhaustion(text).is_none(), "{text}");
        }
    }

    #[test]
    fn rendered_error_is_redetected() {
        let now = local(2026, 9, 26, 10, 0);
        let rendered =
            detect_provider_exhaustion_at("You've hit your session limit · resets 4pm", now)
                .expect("session limit")
                .into_error()
                .to_string();
        assert!(
            rendered.starts_with(PROVIDER_EXHAUSTED_MARKER),
            "{rendered}"
        );
        let again = detect_provider_exhaustion_at(&format!("exit 1: {rendered}"), now)
            .expect("rendered text stays detectable");
        assert_eq!(
            again.resets_at_ms,
            Some(local(2026, 9, 26, 16, 0).timestamp_millis())
        );
    }

    #[test]
    fn cli_session_limit_classifies_as_provider_exhausted() {
        let err = classify_cli_error(
            1,
            &json!("You've hit your session limit · resets 4pm (Europe/Berlin)"),
            "CLI",
        );
        assert!(
            matches!(err, ProviderError::ProviderExhausted { .. }),
            "got {err:?}"
        );
    }

    #[test]
    fn http_429_with_long_retry_after_is_exhaustion() {
        let long = classify_http_status(
            429,
            &json!({"retry_after": 3_600, "error": {"message": "slow down"}}),
            RetryAfterSource::BodyRetryAfter,
        );
        let ProviderError::ProviderExhausted {
            resets_at_ms: Some(resets_at_ms),
            message,
        } = long
        else {
            panic!("expected ProviderExhausted, got {long:?}");
        };
        let expected = Local::now().timestamp_millis() + 3_600_000;
        assert!((resets_at_ms - expected).abs() < 60_000);
        assert_eq!(message, "slow down (retry after 3600s)");

        let short = classify_http_status(
            429,
            &json!({"retry_after": 7}),
            RetryAfterSource::BodyRetryAfter,
        );
        assert!(matches!(
            short,
            ProviderError::RateLimit {
                retry_after_ms: Some(7_000)
            }
        ));
    }

    #[test]
    fn rate_limited_billing_body_is_insufficient_credits() {
        let err = classify_rate_limited(
            r#"{"error":{"message":"You exceeded your current quota","type":"insufficient_quota"}}"#,
            Some(20_000),
        );
        assert!(matches!(err, ProviderError::InsufficientCredits), "{err:?}");
    }
}

#[cfg(test)]
mod turn_cap_tests {
    use super::*;

    #[test]
    fn turn_cap_text_round_trips_and_is_not_an_exhaustion() {
        let hit = TurnCapHit {
            num_turns: Some(61),
            cap: Some(60),
        };
        let text = format!("exit 1: {hit}");
        assert_eq!(detect_turn_cap(&text), Some(hit));
        assert!(
            detect_provider_exhaustion(&text).is_none(),
            "a turn cap must not quarantine the provider: {text}"
        );
        let unknown = TurnCapHit {
            num_turns: None,
            cap: None,
        };
        assert_eq!(detect_turn_cap(&unknown.to_string()), Some(unknown));
        assert_eq!(detect_turn_cap("exit 1: claude failed"), None);
    }

    #[test]
    fn attempt_timeout_text_is_neither_a_turn_cap_nor_an_exhaustion() {
        let text = format!("{ATTEMPT_TIMEOUT_MARKER} 600000 ms");
        assert!(detect_attempt_timeout(&text));
        assert!(detect_attempt_timeout(
            "cursor-cli timed out after 5000 ms (collected 12 bytes)"
        ));
        assert_eq!(detect_turn_cap(&text), None);
        assert!(detect_provider_exhaustion(&text).is_none(), "{text}");
        assert!(!detect_attempt_timeout("exit 1: claude failed"));
    }
}
