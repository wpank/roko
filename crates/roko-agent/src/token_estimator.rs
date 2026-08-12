//! Simple heuristic token estimators for pre-dispatch size checks and cost
//! projection.
//!
//! These estimators intentionally avoid heavy tokenizer dependencies (e.g.
//! tiktoken). The heuristics are accurate enough for budget enforcement and
//! context-overflow guards; exact token counts are measured post-dispatch from
//! the provider's usage response.
//!
//! # Heuristics
//!
//! - **English prose**: ~4 characters per token (aligns with the GPT-4 /
//!   Claude tokenizer average for typical English text).
//! - **Code**: ~3.5 characters per token (code has more short identifiers,
//!   punctuation, and keywords, which tokenize more finely).
//!
//! # Context-window checks
//!
//! [`check_context_window`] compares a pre-dispatch token estimate against
//! a model's configured context window and returns a [`ContextWindowStatus`]
//! indicating whether dispatch should proceed, warn, or be rejected.
//!
//! The thresholds mirror the task spec (E48-T07):
//! - **Below 50%**: no check performed (fast path).
//! - **50%–80%**: [`ContextWindowStatus::Ok`].
//! - **80%–95%**: [`ContextWindowStatus::Warning`] — callers should log but
//!   may still dispatch.
//! - **Above 95%**: [`ContextWindowStatus::Rejected`] — dispatch would almost
//!   certainly fail at the provider; callers should try a larger-context model.

/// Estimate the token count for an arbitrary text string.
///
/// Uses a simple character-count heuristic:
/// - Source code detected by `is_code_like()` → 3.5 chars/token
/// - All other text → 4 chars/token
///
/// Returns at least 1 for any non-empty string.
#[must_use]
pub fn estimate_tokens(text: &str) -> u64 {
    if text.is_empty() {
        return 0;
    }

    let chars = text.len() as f64;
    let tokens = if is_code_like(text) {
        chars / 3.5
    } else {
        chars / 4.0
    };

    // Round up and floor at 1 so callers always get a positive count for
    // non-empty input.
    tokens.ceil() as u64
}

/// Estimate the total token count for a slice of chat messages represented
/// as [`serde_json::Value`] objects.
///
/// Each message's `content` field is extracted and passed to
/// [`estimate_tokens`]. An overhead of **4 tokens per message** is added to
/// account for the role prefix, separator tokens, and message framing that
/// providers add around each turn.
///
/// Handles both string content and content-block arrays (the `{ type, text }`
/// schema used by Claude's Messages API).
#[must_use]
pub fn estimate_prompt_tokens(messages: &[serde_json::Value]) -> u64 {
    const OVERHEAD_PER_MESSAGE: u64 = 4;

    messages.iter().fold(0u64, |acc, msg| {
        let content_tokens = extract_message_text(msg)
            .iter()
            .map(|s| estimate_tokens(s))
            .sum::<u64>();

        acc + content_tokens + OVERHEAD_PER_MESSAGE
    })
}

// ── context-window thresholds ─────────────────────────────────────────────────

/// Fraction of the context window at which we start checking.
const CONTEXT_CHECK_LOWER_THRESHOLD: f64 = 0.50;

/// Fraction of the context window at which we emit a warning.
const CONTEXT_WARN_THRESHOLD: f64 = 0.80;

/// Fraction of the context window at which we reject the dispatch.
const CONTEXT_REJECT_THRESHOLD: f64 = 0.95;

/// Result of a context-window pre-flight check.
#[derive(Debug, Clone, PartialEq)]
pub enum ContextWindowStatus {
    /// Estimated token count is well below the limit; no action needed.
    Ok {
        /// Estimated total prompt tokens.
        estimated_tokens: u64,
        /// Model context window size.
        context_window: u64,
        /// Fraction of context window consumed (0.0–1.0).
        utilization: f64,
    },
    /// Estimated token count is above 80% of the context window; callers
    /// should log a warning but may still dispatch.
    Warning {
        /// Estimated total prompt tokens.
        estimated_tokens: u64,
        /// Model context window size.
        context_window: u64,
        /// Fraction of context window consumed (0.0–1.0).
        utilization: f64,
    },
    /// Estimated token count exceeds 95% of the context window; dispatch
    /// should be rejected to avoid a provider-side context-overflow error.
    Rejected {
        /// Estimated total prompt tokens.
        estimated_tokens: u64,
        /// Model context window size.
        context_window: u64,
        /// Fraction of context window consumed (0.0–1.0).
        utilization: f64,
        /// Human-readable reason for the rejection.
        reason: String,
    },
}

impl ContextWindowStatus {
    /// Returns `true` if this status indicates a hard rejection.
    #[must_use]
    pub fn is_rejected(&self) -> bool {
        matches!(self, Self::Rejected { .. })
    }

    /// Returns `true` if this status indicates a warning (but not a
    /// rejection).
    #[must_use]
    pub fn is_warning(&self) -> bool {
        matches!(self, Self::Warning { .. })
    }

    /// Utilization as a fraction (0.0–1.0), regardless of the variant.
    #[must_use]
    pub fn utilization(&self) -> f64 {
        match self {
            Self::Ok { utilization, .. }
            | Self::Warning { utilization, .. }
            | Self::Rejected { utilization, .. } => *utilization,
        }
    }
}

/// Perform a pre-dispatch context-window check for `estimated_tokens` against
/// `context_window` (both in tokens).
///
/// Returns [`None`] when the estimate is below 50% of `context_window` (fast
/// path — no check needed). Otherwise returns a [`ContextWindowStatus`]
/// describing whether the request is safe, borderline, or should be rejected.
///
/// `model_slug` is used only for diagnostic messages; it does not affect the
/// threshold logic.
#[must_use]
pub fn check_context_window(
    estimated_tokens: u64,
    context_window: u64,
    model_slug: &str,
) -> Option<ContextWindowStatus> {
    if context_window == 0 {
        return None;
    }

    let utilization = estimated_tokens as f64 / context_window as f64;

    // Below the lower threshold: skip the check entirely (fast path).
    if utilization < CONTEXT_CHECK_LOWER_THRESHOLD {
        return None;
    }

    if utilization >= CONTEXT_REJECT_THRESHOLD {
        Some(ContextWindowStatus::Rejected {
            estimated_tokens,
            context_window,
            utilization,
            reason: format!(
                "estimated {estimated_tokens} tokens is {:.1}% of the {context_window}-token \
                 context window for model '{model_slug}' (limit: {:.0}%)",
                utilization * 100.0,
                CONTEXT_REJECT_THRESHOLD * 100.0,
            ),
        })
    } else if utilization >= CONTEXT_WARN_THRESHOLD {
        Some(ContextWindowStatus::Warning {
            estimated_tokens,
            context_window,
            utilization,
        })
    } else {
        Some(ContextWindowStatus::Ok {
            estimated_tokens,
            context_window,
            utilization,
        })
    }
}

/// Look up the known context window size (in tokens) for a model slug.
///
/// Uses substring matching against well-known model family patterns. When the
/// slug does not match any known family the function returns `None`; callers
/// should fall back to their configured `ModelProfile.context_window`.
///
/// Context windows are intentionally conservative (the commonly published
/// figures) — providers may increase them in later versions.
#[must_use]
pub fn context_window_for_slug(slug: &str) -> Option<u64> {
    let s = slug.to_ascii_lowercase();

    // Claude family — all modern Claude models share a 200K window.
    if s.contains("claude-opus") || s.contains("claude-sonnet") || s.contains("claude-haiku") {
        return Some(200_000);
    }

    // Gemini family
    if s.contains("gemini-2.5") || s.contains("gemini-3") || s.contains("gemini-1.5") {
        return Some(1_048_576);
    }
    if s.contains("gemini-1.0") || (s.contains("gemini") && s.contains("-pro")) {
        return Some(32_768);
    }

    // OpenAI family
    if s.starts_with("gpt-4o") || s.starts_with("gpt-4-turbo") {
        return Some(128_000);
    }
    if s.starts_with("gpt-4") {
        return Some(8_192);
    }
    if s.starts_with("gpt-3.5") {
        return Some(16_385);
    }

    // o-series (OpenAI reasoning)
    if s.starts_with("o1") || s.starts_with("o3") || s.starts_with("o4") {
        return Some(200_000);
    }

    // Kimi / Moonshot
    if s.starts_with("kimi-k2") || s.starts_with("moonshot") {
        return Some(128_000);
    }

    // Perplexity / Sonar
    if s.starts_with("sonar") || s.starts_with("perplexity") {
        return Some(127_072);
    }

    // Ollama / local models: conservative default
    if s.contains("llama") || s.contains("mistral") || s.contains("qwen") {
        return Some(8_192);
    }

    None
}

// ── helpers ───────────────────────────────────────────────────────────────────

/// Extract all text content from a chat message value.
///
/// Supports:
/// - `{ "content": "<string>" }` — OpenAI-style string content
/// - `{ "content": [{ "type": "text", "text": "<string>" }, …] }` — Claude
///   content-block arrays
///
/// Any unrecognised shape returns an empty vec; no panics.
fn extract_message_text(msg: &serde_json::Value) -> Vec<String> {
    let Some(content) = msg.get("content") else {
        return Vec::new();
    };

    match content {
        serde_json::Value::String(s) => vec![s.clone()],
        serde_json::Value::Array(blocks) => blocks
            .iter()
            .filter_map(|block| {
                if block.get("type").and_then(|t| t.as_str()) == Some("text") {
                    block.get("text").and_then(|t| t.as_str()).map(String::from)
                } else {
                    None
                }
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// Heuristic to decide whether a string looks like source code.
///
/// We check for a high density of common code punctuation characters
/// (`{`, `}`, `(`, `)`, `;`, `=`, `<`, `>`). If more than 5 % of characters
/// are such tokens the string is classified as code.
fn is_code_like(text: &str) -> bool {
    let code_chars = text
        .chars()
        .filter(|c| matches!(c, '{' | '}' | '(' | ')' | ';' | '=' | '<' | '>' | '[' | ']'))
        .count();

    let total = text.len();
    total > 0 && (code_chars as f64 / total as f64) > 0.05
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── estimate_tokens ──────────────────────────────────────────────────────

    #[test]
    fn empty_string_returns_zero() {
        assert_eq!(estimate_tokens(""), 0);
    }

    #[test]
    fn short_prose_rounds_up_to_one() {
        // "hi" is 2 chars; 2/4 = 0.5 → ceil → 1
        assert_eq!(estimate_tokens("hi"), 1);
    }

    #[test]
    fn prose_uses_four_chars_per_token() {
        // 40 characters of prose → 40/4 = 10 tokens
        let text = "The quick brown fox jumps over the lazy ";
        assert_eq!(text.len(), 40);
        assert_eq!(estimate_tokens(text), 10);
    }

    #[test]
    fn code_uses_three_point_five_chars_per_token() {
        // 35 chars of code-heavy text → 35/3.5 = 10 tokens
        let code = "fn main() { let x = 1; let y = 2; }";
        // Verify it's detected as code first
        assert!(is_code_like(code));
        // 36 chars / 3.5 = 10.28… → ceil → 11
        let expected = (code.len() as f64 / 3.5).ceil() as u64;
        assert_eq!(estimate_tokens(code), expected);
    }

    #[test]
    fn string_content_messages() {
        let messages = vec![
            serde_json::json!({ "role": "user", "content": "Hello world" }),
            serde_json::json!({ "role": "assistant", "content": "Hi there" }),
        ];
        // "Hello world" = 11 chars / 4 = 2.75 → ceil → 3
        // "Hi there"   = 8 chars / 4 = 2   → ceil → 2
        // overhead     = 2 * 4 = 8
        // total        = 3 + 2 + 8 = 13
        assert_eq!(estimate_prompt_tokens(&messages), 13);
    }

    #[test]
    fn content_block_messages() {
        let messages = vec![serde_json::json!({
            "role": "user",
            "content": [
                { "type": "text", "text": "Hello world" },
                { "type": "image", "source": {} }   // non-text block; ignored
            ]
        })];
        // text: "Hello world" = 11 chars / 4 = 2.75 → ceil → 3
        // overhead = 4
        // total = 7
        assert_eq!(estimate_prompt_tokens(&messages), 7);
    }

    #[test]
    fn missing_content_contributes_only_overhead() {
        let messages = vec![serde_json::json!({ "role": "system" })];
        // No content field → 0 content tokens + 4 overhead
        assert_eq!(estimate_prompt_tokens(&messages), 4);
    }

    #[test]
    fn empty_messages_slice() {
        assert_eq!(estimate_prompt_tokens(&[]), 0);
    }

    #[test]
    fn is_code_like_detects_rust() {
        assert!(is_code_like("fn foo(x: i32) -> i32 { x + 1 }"));
    }

    #[test]
    fn is_code_like_rejects_prose() {
        assert!(!is_code_like("The quick brown fox jumps over the lazy dog"));
    }

    // ── context_window_for_slug ──────────────────────────────────────────────

    #[test]
    fn token_context_window_claude_opus() {
        assert_eq!(context_window_for_slug("claude-opus-4-6"), Some(200_000));
    }

    #[test]
    fn token_context_window_claude_sonnet() {
        assert_eq!(context_window_for_slug("claude-sonnet-4-6"), Some(200_000));
    }

    #[test]
    fn token_context_window_gemini_25() {
        assert_eq!(
            context_window_for_slug("gemini-2.5-flash"),
            Some(1_048_576)
        );
    }

    #[test]
    fn token_context_window_gpt4o() {
        assert_eq!(context_window_for_slug("gpt-4o"), Some(128_000));
    }

    #[test]
    fn token_context_window_unknown_slug_returns_none() {
        assert_eq!(context_window_for_slug("my-custom-model-v99"), None);
    }

    #[test]
    fn token_context_window_case_insensitive() {
        assert_eq!(
            context_window_for_slug("Claude-Sonnet-4-6"),
            Some(200_000)
        );
    }

    // ── check_context_window ─────────────────────────────────────────────────

    #[test]
    fn token_check_skips_below_50_pct() {
        // 49% utilization → fast path, None
        let context = 100_000u64;
        let tokens = 49_000u64;
        assert_eq!(check_context_window(tokens, context, "model-x"), None);
    }

    #[test]
    fn token_check_ok_between_50_and_80_pct() {
        let context = 100_000u64;
        // 65% utilization → Ok
        let tokens = 65_000u64;
        let result = check_context_window(tokens, context, "model-x")
            .expect("should return a status for 65%");
        assert!(
            matches!(result, ContextWindowStatus::Ok { .. }),
            "expected Ok, got {result:?}"
        );
        assert!(!result.is_rejected());
        assert!(!result.is_warning());
    }

    #[test]
    fn token_check_warns_at_80_pct() {
        let context = 100_000u64;
        // 85% utilization → Warning
        let tokens = 85_000u64;
        let result = check_context_window(tokens, context, "model-x")
            .expect("should return a status for 85%");
        assert!(result.is_warning(), "expected Warning, got {result:?}");
    }

    #[test]
    fn token_check_rejects_at_95_pct() {
        let context = 100_000u64;
        // 96% utilization → Rejected
        let tokens = 96_000u64;
        let result = check_context_window(tokens, context, "claude-sonnet-4-6")
            .expect("should return a status for 96%");
        assert!(result.is_rejected(), "expected Rejected, got {result:?}");
        if let ContextWindowStatus::Rejected { reason, .. } = result {
            assert!(
                reason.contains("claude-sonnet-4-6"),
                "rejection reason should include model slug: {reason}"
            );
        }
    }

    #[test]
    fn token_check_zero_context_returns_none() {
        // Zero context window → skip to avoid division by zero
        assert_eq!(check_context_window(1_000, 0, "model-x"), None);
    }

    #[test]
    fn token_check_utilization_is_accurate() {
        let context = 200_000u64;
        let tokens = 160_000u64; // 80% exactly
        let result =
            check_context_window(tokens, context, "model-x").expect("80% should return a status");
        let utilization = result.utilization();
        assert!(
            (utilization - 0.80).abs() < 1e-9,
            "utilization should be exactly 0.80, got {utilization}"
        );
    }

    #[test]
    fn token_check_boundary_at_exactly_95_pct() {
        let context = 200_000u64;
        // 95% exactly → Rejected (the threshold is >=)
        let tokens = 190_000u64;
        let result =
            check_context_window(tokens, context, "model-x").expect("95% should return a status");
        assert!(
            result.is_rejected(),
            "exactly 95% should be rejected, got {result:?}"
        );
    }

    #[test]
    fn token_check_boundary_just_below_95_pct() {
        let context = 200_000u64;
        // 94.9% → Warning (just below the reject threshold)
        let tokens = 189_999u64;
        let result = check_context_window(tokens, context, "model-x")
            .expect("~94.9% should return a status");
        assert!(
            result.is_warning(),
            "just below 95% should be Warning, got {result:?}"
        );
    }
}
