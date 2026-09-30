//! Log scrubbing middleware (items 43.15--43.16).
//!
//! [`LogScrubber`] redacts known secret patterns (API keys, tokens, auth
//! headers) from arbitrary text. Used by the tracing layer and tool-output
//! pipeline to prevent secrets from leaking into logs, traces, and signal
//! bodies.
//!
//! Built-in patterns cover the most common leak vectors:
//! - `sk-...` (`API_KEY`)
//! - `ghp_...` / `gho_...` / `ghs_...` / `ghu_...` / `ghr_...` (`GitHub` tokens)
//! - `xoxb-...` (`Slack` bot tokens)
//! - `Bearer ...` (Authorization headers)
//! - `ANTHROPIC_API_KEY=...` / `OPENAI_API_KEY=...` (env-var leaks)
//!
//! Custom patterns can be added at runtime via [`LogScrubber::add_pattern`].
//!
//! The process also keeps one scrubber holding its known secrets, the values
//! roko loaded from `.env` files and provider keys: see
//! [`install_secret_scrubber`]. Persistence writers pass what they write
//! through [`scrub_secrets`], [`scrub_secrets_in_json`] or
//! [`scrub_secrets_in_jsonl`], which remove those exact values and never
//! apply the heuristic patterns: a heuristic such as `sk-...` also matches
//! ordinary text (the task id `mask-the-...`), and a record's ids must
//! survive being written.

use std::borrow::Cow;
use std::sync::Arc;

use parking_lot::RwLock;

/// Replacement text inserted in place of scrubbed secrets.
pub const REDACTED: &str = "[REDACTED]";

/// Shortest value treated as a secret. Shorter values (`true`, `8080`,
/// `debug`) also occur in ordinary text, so redacting them would mangle it.
pub const MIN_SECRET_LEN: usize = 8;

/// A compiled regex pattern used by the scrubber.
struct ScrubPattern {
    regex: regex::Regex,
    replacement: String,
    /// Length of the exact secret for literal patterns; `None` for heuristics.
    literal_len: Option<usize>,
}

impl ScrubPattern {
    fn new(pattern: &str) -> Result<Self, regex::Error> {
        Self::with_replacement(pattern, REDACTED)
    }

    fn with_replacement(
        pattern: &str,
        replacement: impl Into<String>,
    ) -> Result<Self, regex::Error> {
        Ok(Self {
            regex: regex::Regex::new(pattern)?,
            replacement: replacement.into(),
            literal_len: None,
        })
    }

    fn scrub<'a>(&self, text: &'a str) -> std::borrow::Cow<'a, str> {
        self.regex.replace_all(text, self.replacement.as_str())
    }
}

/// Redacts known secret patterns from log/trace output.
///
/// Thread-safe: custom patterns can be added from any thread via
/// [`LogScrubber::add_pattern`].
pub struct LogScrubber {
    patterns: RwLock<Vec<ScrubPattern>>,
}

impl std::fmt::Debug for LogScrubber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let len = self.patterns.read().len();
        f.debug_struct("LogScrubber")
            .field("pattern_count", &len)
            .finish_non_exhaustive()
    }
}

/// Built-in patterns covering common secret formats.
fn builtin_patterns() -> Vec<ScrubPattern> {
    // Each pattern captures the secret portion and replaces the whole match.
    let raw = [
        // GitHub personal access tokens.
        (r"ghp_[A-Za-z0-9]{36}", "[REDACTED:GITHUB_PAT]"),
        // API keys (require 20+ chars after prefix to avoid false positives like `sk-short`).
        (r"sk-[A-Za-z0-9-]{20,}", "[REDACTED:API_KEY]"),
        // Slack bot tokens.
        (r"xoxb-[0-9]+-[A-Za-z0-9]+", "[REDACTED:SLACK_BOT_TOKEN]"),
        // Anthropic / OpenAI API keys: sk-ant-..., sk-proj-..., sk-... (20+ chars)
        (r"sk-[A-Za-z0-9_-]{20,}", REDACTED),
        // Generic `key-...` API key prefix (Perplexity pplx-..., and similar services that
        // use a `key-<token>` format with 20+ chars after the prefix).
        (r"key-[A-Za-z0-9_-]{20,}", "[REDACTED:API_KEY]"),
        // GitHub tokens beyond the PAT shape above.
        (r"gh[pousr]_[A-Za-z0-9_]{16,}", REDACTED),
        // Bearer tokens in authorization headers (value after "Bearer ")
        (r"(?i)Bearer\s+[A-Za-z0-9_.+/=-]{10,}", REDACTED),
        // Env-var leak: ANTHROPIC_API_KEY=<value>
        (r"ANTHROPIC_API_KEY=[^\s]+", REDACTED),
        // Env-var leak: OPENAI_API_KEY=<value>
        (r"OPENAI_API_KEY=[^\s]+", REDACTED),
        // Env-var leak: PERPLEXITY_API_KEY=<value>
        (r"PERPLEXITY_API_KEY=[^\s]+", REDACTED),
        // Env-var leak: CEREBRAS_API_KEY=<value>
        (r"CEREBRAS_API_KEY=[^\s]+", REDACTED),
        // Env-var leak: GEMINI_API_KEY=<value>
        (r"GEMINI_API_KEY=[^\s]+", REDACTED),
    ];
    raw.iter()
        .map(|p| {
            ScrubPattern::with_replacement(p.0, p.1).unwrap_or_else(|e| {
                panic!("built-in scrub pattern {:?} failed to compile: {e}", p.0)
            })
        })
        .collect()
}

impl LogScrubber {
    /// Create a scrubber pre-loaded with built-in patterns.
    #[must_use]
    pub fn new() -> Self {
        Self {
            patterns: RwLock::new(builtin_patterns()),
        }
    }

    /// Create a scrubber with no patterns (useful for tests that add custom
    /// patterns only).
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            patterns: RwLock::new(Vec::new()),
        }
    }

    /// Add a custom regex pattern. Returns an error if the pattern is invalid.
    ///
    /// # Errors
    ///
    /// Returns the underlying `regex::Error` if the pattern fails to compile.
    pub fn add_pattern(&self, pattern: &str) -> Result<(), regex::Error> {
        let compiled = ScrubPattern::new(pattern)?;
        self.patterns.write().push(compiled);
        Ok(())
    }

    /// Add a custom regex pattern with a specific replacement string.
    ///
    /// This is used for `.env` values, where the replacement must identify the
    /// loaded variable name.
    pub fn add_pattern_with_replacement(
        &self,
        pattern: &str,
        replacement: impl Into<String>,
    ) -> Result<(), regex::Error> {
        let compiled = ScrubPattern::with_replacement(pattern, replacement)?;
        self.patterns.write().push(compiled);
        Ok(())
    }

    /// Add a literal value to the scrubber, redacting it as `name`.
    ///
    /// Literal values are exact known secrets, so they run before the
    /// heuristic patterns, longest first: a pattern matching only part of the
    /// value would otherwise leave the rest of the secret in the output and
    /// hide its name. Adding a value already present does nothing, so the
    /// first name given for a value is the one shown.
    pub fn add_literal_value(&self, value: &str, name: &str) -> Result<(), regex::Error> {
        if value.is_empty() {
            return Ok(());
        }
        let escaped = regex::escape(value);
        let mut literal = ScrubPattern::with_replacement(&escaped, format!("[REDACTED:{name}]"))?;
        literal.literal_len = Some(value.len());
        let mut patterns = self.patterns.write();
        if patterns.iter().any(|pattern| {
            pattern.literal_len == Some(value.len()) && pattern.regex.as_str() == escaped
        }) {
            return Ok(());
        }
        let position = patterns
            .iter()
            .position(|pattern| pattern.literal_len.is_none_or(|len| len < value.len()))
            .unwrap_or(patterns.len());
        patterns.insert(position, literal);
        Ok(())
    }

    /// Scrub all known patterns from the input text, replacing matches with
    /// [`REDACTED`].
    #[must_use]
    pub fn scrub(&self, text: &str) -> String {
        let patterns = self.patterns.read();
        let mut result = text.to_string();
        for pattern in patterns.iter() {
            let scrubbed = pattern.scrub(&result);
            if let std::borrow::Cow::Owned(s) = scrubbed {
                result = s;
            }
        }
        drop(patterns);
        result
    }

    /// Scrub only the literal values added with
    /// [`LogScrubber::add_literal_value`], leaving the heuristic patterns
    /// out. Borrows `text` when no literal occurs in it.
    #[must_use]
    pub fn scrub_literals<'a>(&self, text: &'a str) -> Cow<'a, str> {
        let patterns = self.patterns.read();
        let mut result = Cow::Borrowed(text);
        for pattern in patterns.iter().filter(|p| p.literal_len.is_some()) {
            let scrubbed = pattern.scrub(&result);
            if let Cow::Owned(s) = scrubbed {
                result = Cow::Owned(s);
            }
        }
        drop(patterns);
        result
    }

    /// Whether any literal value occurs in `text`.
    #[must_use]
    pub fn contains_literal(&self, text: &str) -> bool {
        self.patterns
            .read()
            .iter()
            .any(|pattern| pattern.literal_len.is_some() && pattern.regex.is_match(text))
    }

    /// Number of patterns currently registered.
    #[must_use]
    pub fn pattern_count(&self) -> usize {
        self.patterns.read().len()
    }
}

impl Default for LogScrubber {
    fn default() -> Self {
        Self::new()
    }
}

// ─── The process's secret scrubber (T036) ────────────────────────────────────

static SECRET_SCRUBBER: RwLock<Option<Arc<LogScrubber>>> = RwLock::new(None);

/// Serializes the tests that swap the process's secret scrubber.
#[cfg(test)]
pub(crate) static PROCESS_SCRUBBER_LOCK: parking_lot::Mutex<()> = parking_lot::Mutex::new(());

/// Install `scrubber` as the process's secret scrubber and return the one it
/// replaces. roko installs one at startup
/// (`roko_fs::observability::RunScrubber::install`); a test that installs
/// its own should put the previous one back.
pub fn install_secret_scrubber(scrubber: Option<Arc<LogScrubber>>) -> Option<Arc<LogScrubber>> {
    std::mem::replace(&mut *SECRET_SCRUBBER.write(), scrubber)
}

/// The process's secret scrubber, when one is installed.
#[must_use]
pub fn secret_scrubber() -> Option<Arc<LogScrubber>> {
    SECRET_SCRUBBER.read().clone()
}

/// Add the values of the environment variables `names` to the process's
/// secret scrubber, each redacted under its name. Unset variables and values
/// shorter than [`MIN_SECRET_LEN`] are skipped, and so is everything when no
/// scrubber is installed.
pub fn add_secret_env_values<'a>(names: impl IntoIterator<Item = &'a str>) {
    let Some(scrubber) = secret_scrubber() else {
        return;
    };
    for name in names {
        if let Ok(value) = std::env::var(name)
            && value.len() >= MIN_SECRET_LEN
            && let Err(error) = scrubber.add_literal_value(&value, name)
        {
            tracing::warn!(name, %error, "secret scrubber: failed to add a secret value");
        }
    }
}

/// Add `secrets`, `(name, value)` pairs such as the secrets a config holds,
/// to the process's secret scrubber, each value redacted under its name.
/// Values shorter than [`MIN_SECRET_LEN`] are skipped, and so is everything
/// when no scrubber is installed.
pub fn add_secret_values<'a>(secrets: impl IntoIterator<Item = (&'a str, &'a str)>) {
    let Some(scrubber) = secret_scrubber() else {
        return;
    };
    for (name, value) in secrets {
        if value.len() >= MIN_SECRET_LEN
            && let Err(error) = scrubber.add_literal_value(value, name)
        {
            tracing::warn!(name, %error, "secret scrubber: failed to add a secret value");
        }
    }
}

/// `text` with the process's secrets redacted, borrowed when it holds none.
/// For plain text; JSON goes through [`scrub_secrets_in_json`] or
/// [`scrub_secrets_in_jsonl`].
#[must_use]
pub fn scrub_secrets(text: &str) -> Cow<'_, str> {
    secret_scrubber().map_or(Cow::Borrowed(text), |scrubber| {
        scrubber.scrub_literals(text)
    })
}

/// One JSON document with the process's secrets redacted from its strings
/// (values and object keys) and nowhere else, so it still parses: replacing
/// a numeric secret inside a number, or a secret that holds a quote, would
/// break it. A rewritten document is pretty-printed when `json` spans
/// several lines and compact otherwise, and keeps a trailing newline. Text
/// that is not JSON is scrubbed as plain text. Borrowed when unchanged.
#[must_use]
pub fn scrub_secrets_in_json(json: &str) -> Cow<'_, str> {
    let Some(scrubber) = secret_scrubber() else {
        return Cow::Borrowed(json);
    };
    scrub_json_document(&scrubber, json)
}

/// JSONL text (one or more lines) with each line scrubbed as by
/// [`scrub_secrets_in_json`]. Borrowed when unchanged.
#[must_use]
pub fn scrub_secrets_in_jsonl(jsonl: &str) -> Cow<'_, str> {
    let Some(scrubber) = secret_scrubber() else {
        return Cow::Borrowed(jsonl);
    };
    let mut changed = false;
    let mut result = String::with_capacity(jsonl.len());
    for line in jsonl.split_inclusive('\n') {
        let (record, ending) = line
            .strip_suffix('\n')
            .map_or((line, ""), |record| (record, "\n"));
        let scrubbed = scrub_json_document(&scrubber, record);
        changed |= matches!(scrubbed, Cow::Owned(_));
        result.push_str(&scrubbed);
        result.push_str(ending);
    }
    if changed {
        Cow::Owned(result)
    } else {
        Cow::Borrowed(jsonl)
    }
}

fn scrub_json_document<'a>(scrubber: &LogScrubber, json: &'a str) -> Cow<'a, str> {
    let has_literals = scrubber
        .patterns
        .read()
        .iter()
        .any(|pattern| pattern.literal_len.is_some());
    // A secret holding a quote, a backslash or a control character is
    // escaped in JSON text, so text with escapes is checked value by value.
    if !has_literals || (!json.contains('\\') && !scrubber.contains_literal(json)) {
        return Cow::Borrowed(json);
    }
    let Ok(mut value) = serde_json::from_str::<serde_json::Value>(json) else {
        return scrubber.scrub_literals(json);
    };
    if !scrub_json_strings(scrubber, &mut value) {
        return Cow::Borrowed(json);
    }
    let rewritten = if json.trim_end().contains('\n') {
        serde_json::to_string_pretty(&value)
    } else {
        serde_json::to_string(&value)
    };
    match rewritten {
        Ok(mut text) => {
            if json.ends_with('\n') {
                text.push('\n');
            }
            Cow::Owned(text)
        }
        Err(_) => scrubber.scrub_literals(json),
    }
}

/// Redact the literal secrets in every string of `value`; true when any
/// string changed.
fn scrub_json_strings(scrubber: &LogScrubber, value: &mut serde_json::Value) -> bool {
    match value {
        serde_json::Value::String(text) => {
            let Cow::Owned(scrubbed) = scrubber.scrub_literals(text) else {
                return false;
            };
            *text = scrubbed;
            true
        }
        serde_json::Value::Array(items) => items.iter_mut().fold(false, |changed, item| {
            scrub_json_strings(scrubber, item) | changed
        }),
        serde_json::Value::Object(map) => {
            let mut changed = map.values_mut().fold(false, |changed, item| {
                scrub_json_strings(scrubber, item) | changed
            });
            if map.keys().any(|key| scrubber.contains_literal(key)) {
                *map = std::mem::take(map)
                    .into_iter()
                    .map(|(key, item)| (scrubber.scrub_literals(&key).into_owned(), item))
                    .collect();
                changed = true;
            }
            changed
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scrubs_anthropic_api_key() {
        let scrubber = LogScrubber::new();
        let input = "Using key sk-ant-api03-abcdefghijklmnopqrstuvwxyz1234567890 for request";
        let output = scrubber.scrub(input);
        assert!(!output.contains("sk-ant-api03"));
        assert!(output.contains("[REDACTED:API_KEY]"));
        assert!(output.contains("Using key"));
        assert!(output.contains("for request"));
    }

    #[test]
    fn scrubs_openai_api_key() {
        let scrubber = LogScrubber::new();
        let input = "key=sk-proj-abcdefghijklmnopqrstuvwxyz";
        let output = scrubber.scrub(input);
        assert!(!output.contains("sk-proj-"));
        assert!(output.contains("[REDACTED:API_KEY]"));
    }

    #[test]
    fn scrubs_github_token() {
        let scrubber = LogScrubber::new();
        let input = "Authorization: token ghp_ABCDEFGHIJKLMNOPqrstuvwxyz1234567890";
        let output = scrubber.scrub(input);
        assert!(!output.contains("ghp_"));
        assert!(output.contains("[REDACTED:GITHUB_PAT]"));
    }

    #[test]
    fn scrubs_slack_bot_token() {
        let scrubber = LogScrubber::new();
        let input = "Authorization: xoxb-1234567890-abcdefghijklmnopqrstuv";
        let output = scrubber.scrub(input);
        assert!(!output.contains("xoxb-"));
        assert!(output.contains("[REDACTED:SLACK_BOT_TOKEN]"));
    }

    #[test]
    fn scrubs_bearer_token() {
        let scrubber = LogScrubber::new();
        let input = "Header: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.test.signature";
        let output = scrubber.scrub(input);
        assert!(!output.contains("eyJhbGciOi"));
        assert!(output.contains(REDACTED));
    }

    #[test]
    fn scrubs_anthropic_env_var() {
        let scrubber = LogScrubber::new();
        let input = "export ANTHROPIC_API_KEY=sk-ant-secret-key-12345";
        let output = scrubber.scrub(input);
        assert!(!output.contains("sk-ant-secret-key"));
        assert!(output.contains("[REDACTED]"));
    }

    #[test]
    fn scrubs_openai_env_var() {
        let scrubber = LogScrubber::new();
        let input = "OPENAI_API_KEY=sk-proj-myverysecretkey12345678";
        let output = scrubber.scrub(input);
        assert!(!output.contains("sk-proj-"));
        assert!(output.contains("[REDACTED]"));
    }

    #[test]
    fn leaves_clean_text_unchanged() {
        let scrubber = LogScrubber::new();
        let input = "Just a normal log line with no secrets at all.";
        let output = scrubber.scrub(input);
        assert_eq!(output, input);
    }

    #[test]
    fn scrubs_multiple_secrets_in_one_line() {
        let scrubber = LogScrubber::new();
        let input =
            "keys: sk-ant-abcdefghijklmnopqrstuvwxyz and ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890";
        let output = scrubber.scrub(input);
        assert!(!output.contains("sk-ant-"));
        assert!(!output.contains("ghp_"));
        assert_eq!(
            output.matches("[REDACTED:API_KEY]").count()
                + output.matches("[REDACTED:GITHUB_PAT]").count(),
            2
        );
    }

    #[test]
    fn custom_pattern_works() {
        let scrubber = LogScrubber::empty();
        scrubber.add_pattern(r"my-secret-\d+").unwrap();
        let input = "found my-secret-42 in config";
        let output = scrubber.scrub(input);
        assert!(!output.contains("my-secret-42"));
        assert!(output.contains(REDACTED));
    }

    #[test]
    fn invalid_custom_pattern_returns_error() {
        let scrubber = LogScrubber::empty();
        let result = scrubber.add_pattern(r"[invalid");
        assert!(result.is_err());
    }

    #[test]
    fn pattern_count_tracks_additions() {
        let scrubber = LogScrubber::empty();
        assert_eq!(scrubber.pattern_count(), 0);
        scrubber.add_pattern(r"foo").unwrap();
        assert_eq!(scrubber.pattern_count(), 1);
        scrubber.add_pattern(r"bar").unwrap();
        assert_eq!(scrubber.pattern_count(), 2);
    }

    #[test]
    fn default_has_builtin_patterns() {
        let scrubber = LogScrubber::default();
        assert!(
            scrubber.pattern_count() >= 11,
            "should have at least 11 built-in patterns"
        );
    }

    #[test]
    fn scrubs_github_org_token() {
        let scrubber = LogScrubber::new();
        let input = "org token: gho_ABCDEFGHIJKLMNOPqrstuvwx";
        let output = scrubber.scrub(input);
        assert!(!output.contains("gho_"));
        assert!(output.contains(REDACTED));
    }

    #[test]
    fn bearer_case_insensitive() {
        let scrubber = LogScrubber::new();
        let input = "header: bearer abcdefghijklmnopqrstuvwxyz12";
        let output = scrubber.scrub(input);
        assert!(output.contains(REDACTED));
    }

    #[test]
    fn empty_string_stays_empty() {
        let scrubber = LogScrubber::new();
        assert_eq!(scrubber.scrub(""), "");
    }

    #[test]
    fn scrubs_key_prefix_api_key() {
        let scrubber = LogScrubber::new();
        let input = "using key-abcdefghijklmnopqrstuvwxyz to authenticate";
        let output = scrubber.scrub(input);
        assert!(!output.contains("key-abcdefghijklmnopqrstuvwxyz"));
        assert!(output.contains("[REDACTED:API_KEY]"));
    }

    #[test]
    fn scrubs_perplexity_env_var() {
        let scrubber = LogScrubber::new();
        let input = "PERPLEXITY_API_KEY=pplx-abcdefghijklmnopqrstuvwxyz";
        let output = scrubber.scrub(input);
        assert!(!output.contains("pplx-abcdefghijklmnopqrstuvwxyz"));
        assert!(output.contains("[REDACTED]"));
    }

    #[test]
    fn scrubs_cerebras_env_var() {
        let scrubber = LogScrubber::new();
        let input = "CEREBRAS_API_KEY=csk-abcdefghijklmnopqrstuvwxyz123456";
        let output = scrubber.scrub(input);
        assert!(!output.contains("csk-abcdefghijklmnopqrstuvwxyz"));
        assert!(output.contains("[REDACTED]"));
    }

    #[test]
    fn scrubs_gemini_env_var() {
        let scrubber = LogScrubber::new();
        let input = "GEMINI_API_KEY=AIzaSyAbcDefGhiJklMnoPqrStuvWxyz-12345";
        let output = scrubber.scrub(input);
        assert!(!output.contains("AIzaSyAbcDefGhiJklMnoPqrStuvWxyz-12345"));
        assert!(output.contains("[REDACTED]"));
    }

    #[test]
    fn short_sk_prefix_not_scrubbed() {
        // Short `sk-` identifiers are not real API keys — avoid false positives.
        let scrubber = LogScrubber::new();
        let input = "key sk-short should stay";
        let output = scrubber.scrub(input);
        assert_eq!(output, input, "sk-short must not be scrubbed");
    }

    #[test]
    fn long_sk_prefix_is_scrubbed() {
        let scrubber = LogScrubber::new();
        let input = "key sk-ant-api03-aBcDeFgHiJkLmNoPqRsTuVwXyZ0123456789ABCD should be redacted";
        let output = scrubber.scrub(input);
        assert!(!output.contains("sk-ant-api03-"));
        assert!(output.contains("[REDACTED:API_KEY]"));
    }

    #[test]
    fn literal_value_uses_named_redaction() {
        let scrubber = LogScrubber::empty();
        scrubber
            .add_literal_value("super-secret-value", "TEST_ENV")
            .unwrap();
        let output = scrubber.scrub("value super-secret-value leaked");
        assert!(output.contains("[REDACTED:TEST_ENV]"));
        assert!(!output.contains("super-secret-value"));
    }

    #[test]
    fn literal_value_is_redacted_whole_before_overlapping_patterns() {
        let scrubber = LogScrubber::new();
        scrubber.add_literal_value("my-api", "SHORT").unwrap();
        scrubber
            .add_literal_value("my-api-key-12345678901234567890", "CUSTOM_KEY")
            .unwrap();
        let output = scrubber.scrub("config: my-api-key-12345678901234567890");
        assert_eq!(output, "config: [REDACTED:CUSTOM_KEY]");
    }

    #[test]
    fn literal_value_added_twice_is_kept_once() {
        let scrubber = LogScrubber::empty();
        scrubber
            .add_literal_value("repeated-secret", "FIRST")
            .unwrap();
        scrubber
            .add_literal_value("repeated-secret", "SECOND")
            .unwrap();
        assert_eq!(scrubber.pattern_count(), 1);
        assert_eq!(scrubber.scrub("x repeated-secret"), "x [REDACTED:FIRST]");
    }

    #[test]
    fn scrub_literals_leaves_heuristic_patterns_out() {
        let scrubber = LogScrubber::new();
        scrubber
            .add_literal_value("canary-literal-9f3e", "CANARY")
            .unwrap();
        // `sk-...` inside a task id matches a heuristic, not a literal.
        let text = "task mask-the-secret-values-in-logs saw canary-literal-9f3e";
        assert_eq!(
            scrubber.scrub_literals(text),
            "task mask-the-secret-values-in-logs saw [REDACTED:CANARY]"
        );
        assert!(scrubber.scrub(text).contains("[REDACTED:API_KEY]"));
        assert!(matches!(
            scrubber.scrub_literals("nothing secret"),
            Cow::Borrowed(_)
        ));
        assert!(scrubber.contains_literal(text));
        assert!(!scrubber.contains_literal("mask-the-secret-values-in-logs"));
    }

    fn literal_scrubber(secrets: &[(&str, &str)]) -> LogScrubber {
        let scrubber = LogScrubber::new();
        for (name, value) in secrets {
            scrubber.add_literal_value(value, name).unwrap();
        }
        scrubber
    }

    #[test]
    fn json_scrub_changes_only_strings() {
        // A numeric secret must not be cut out of a number.
        let scrubber = literal_scrubber(&[("TIMEOUT", "30000000"), ("TOKEN", "json-secret-1")]);
        let json = r#"{"created_at_ms":1727730000000,"note":"saw json-secret-1 and 30000000","json-secret-1":[1,"json-secret-1"]}"#;
        let scrubbed = scrub_json_document(&scrubber, json);
        let value: serde_json::Value = serde_json::from_str(&scrubbed).expect("still JSON");
        assert_eq!(value["created_at_ms"], 1_727_730_000_000_u64);
        assert_eq!(value["note"], "saw [REDACTED:TOKEN] and [REDACTED:TIMEOUT]");
        assert_eq!(value["[REDACTED:TOKEN]"][1], "[REDACTED:TOKEN]");
        assert!(!scrubbed.contains("json-secret-1"));
    }

    #[test]
    fn json_scrub_finds_secrets_json_escapes() {
        let secret = "pa\"ss\\word-with-quote";
        let scrubber = literal_scrubber(&[("QUOTED", secret)]);
        let json = serde_json::json!({ "output": format!("leaked {secret}") }).to_string();
        assert!(!json.contains(secret), "the secret is escaped in JSON text");
        let scrubbed = scrub_json_document(&scrubber, &json);
        let value: serde_json::Value = serde_json::from_str(&scrubbed).expect("still JSON");
        assert_eq!(value["output"], "leaked [REDACTED:QUOTED]");
    }

    #[test]
    fn json_scrub_keeps_layout_and_borrows_when_clean() {
        let scrubber = literal_scrubber(&[("TOKEN", "layout-secret")]);
        let pretty = "{\n  \"a\": \"layout-secret\"\n}\n";
        assert_eq!(
            scrub_json_document(&scrubber, pretty),
            "{\n  \"a\": \"[REDACTED:TOKEN]\"\n}\n"
        );
        assert_eq!(
            scrub_json_document(&scrubber, r#"{"a":"layout-secret"}"#),
            r#"{"a":"[REDACTED:TOKEN]"}"#
        );
        for clean in [r#"{"a":"clean\ntext"}"#, r#"{"a":1}"#, "not json"] {
            assert!(matches!(
                scrub_json_document(&scrubber, clean),
                Cow::Borrowed(_)
            ));
        }
        // Text that is not JSON is scrubbed as text.
        assert_eq!(
            scrub_json_document(&scrubber, "plain layout-secret"),
            "plain [REDACTED:TOKEN]"
        );
    }

    #[test]
    fn process_scrubber_scrubs_records_and_restores() {
        let _guard = PROCESS_SCRUBBER_LOCK.lock();
        let scrubber = Arc::new(literal_scrubber(&[("TOKEN", "process-secret-7")]));
        let previous = install_secret_scrubber(Some(Arc::clone(&scrubber)));

        assert_eq!(
            scrub_secrets("saw process-secret-7"),
            "saw [REDACTED:TOKEN]"
        );
        assert_eq!(
            scrub_secrets_in_json("{\"a\":\"process-secret-7\"}"),
            "{\"a\":\"[REDACTED:TOKEN]\"}"
        );
        let jsonl = "{\"a\":\"clean\"}\n{\"b\":\"process-secret-7\"}\n";
        assert_eq!(
            scrub_secrets_in_jsonl(jsonl),
            "{\"a\":\"clean\"}\n{\"b\":\"[REDACTED:TOKEN]\"}\n"
        );
        // An environment value joins the installed scrubber under its name.
        let path = std::env::var("PATH").expect("PATH is set");
        if path.len() >= MIN_SECRET_LEN {
            add_secret_env_values(["PATH"]);
            assert_eq!(scrub_secrets(&path), "[REDACTED:PATH]");
        }
        // So does a config's secret, under its field; a short value does not.
        add_secret_values([
            ("serve.auth.api_key", "config-secret-5a6636"),
            ("x", "tiny"),
        ]);
        assert_eq!(
            scrub_secrets("config-secret-5a6636 tiny"),
            "[REDACTED:serve.auth.api_key] tiny"
        );

        install_secret_scrubber(previous);
        if secret_scrubber().is_none() {
            assert!(matches!(
                scrub_secrets("saw process-secret-7"),
                Cow::Borrowed(_)
            ));
        }
    }
}
