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

// ── helpers ──────────────────────────────────────────────────────────────────

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
}
