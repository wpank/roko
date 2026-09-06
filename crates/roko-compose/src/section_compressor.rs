//! P4-10: Section-level context compression.
//!
//! When token budget is tight, compresses low-priority sections by
//! extracting key sentences rather than dropping them entirely.
//! Uses a simple extractive summarization approach based on sentence
//! importance heuristics.

use serde::{Deserialize, Serialize};

/// Maximum compression ratio (0.0 = delete, 1.0 = no compression).
const DEFAULT_MIN_RATIO: f64 = 0.3;

/// Compression result for a single section.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressedSection {
    /// Original section name.
    pub section_name: String,
    /// Compressed content.
    pub content: String,
    /// Original token count estimate.
    pub original_tokens: usize,
    /// Compressed token count estimate.
    pub compressed_tokens: usize,
    /// Compression ratio achieved.
    pub ratio: f64,
}

/// Section compression configuration.
#[derive(Debug, Clone)]
pub struct CompressionConfig {
    /// Minimum compression ratio (floor).
    pub min_ratio: f64,
    /// Target compression ratio.
    pub target_ratio: f64,
    /// Whether to preserve the first sentence (usually a summary).
    pub preserve_first_sentence: bool,
    /// Whether to preserve the last sentence (often a conclusion).
    pub preserve_last_sentence: bool,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            min_ratio: DEFAULT_MIN_RATIO,
            target_ratio: 0.5,
            preserve_first_sentence: true,
            preserve_last_sentence: false,
        }
    }
}

/// Compress a section's content to fit within a token budget.
///
/// Uses extractive summarization: scores sentences by importance
/// and keeps the most important ones until the budget is reached.
#[must_use]
pub fn compress_section(
    section_name: &str,
    content: &str,
    config: &CompressionConfig,
) -> CompressedSection {
    let original_tokens = estimate_tokens(content);
    let target_tokens = (original_tokens as f64 * config.target_ratio) as usize;

    if target_tokens >= original_tokens || original_tokens < 20 {
        return CompressedSection {
            section_name: section_name.to_string(),
            content: content.to_string(),
            original_tokens,
            compressed_tokens: original_tokens,
            ratio: 1.0,
        };
    }

    let sentences = split_sentences(content);
    if sentences.len() <= 2 {
        return CompressedSection {
            section_name: section_name.to_string(),
            content: content.to_string(),
            original_tokens,
            compressed_tokens: original_tokens,
            ratio: 1.0,
        };
    }

    // Score sentences by importance.
    let mut scored: Vec<(usize, f64, &str)> = sentences
        .iter()
        .enumerate()
        .map(|(i, &sentence)| {
            let mut score = score_sentence(sentence, i, sentences.len());

            // Boost first and last if configured.
            if config.preserve_first_sentence && i == 0 {
                score += 10.0;
            }
            if config.preserve_last_sentence && i == sentences.len() - 1 {
                score += 5.0;
            }

            (i, score, sentence)
        })
        .collect();

    // Sort by score descending.
    scored.sort_by(|a, b| b.1.total_cmp(&a.1));

    // Select top sentences until budget is reached.
    let mut selected: Vec<(usize, &str)> = Vec::new();
    let mut token_count = 0usize;

    for (idx, _score, sentence) in &scored {
        let sentence_tokens = estimate_tokens(sentence);
        if token_count + sentence_tokens > target_tokens && !selected.is_empty() {
            break;
        }
        selected.push((*idx, sentence));
        token_count += sentence_tokens;
    }

    // Sort selected by original order to maintain coherence.
    selected.sort_by_key(|(idx, _)| *idx);

    let compressed: String = selected
        .iter()
        .map(|(_, s)| *s)
        .collect::<Vec<_>>()
        .join(" ");

    let compressed_tokens = estimate_tokens(&compressed);
    let ratio = if original_tokens > 0 {
        compressed_tokens as f64 / original_tokens as f64
    } else {
        1.0
    };

    CompressedSection {
        section_name: section_name.to_string(),
        content: compressed,
        original_tokens,
        compressed_tokens,
        ratio: ratio.max(config.min_ratio),
    }
}

fn split_sentences(text: &str) -> Vec<&str> {
    // Simple sentence splitter on period/question/exclamation followed by space or EOL.
    let mut sentences = Vec::new();
    let mut start = 0;

    for (i, c) in text.char_indices() {
        if (c == '.' || c == '?' || c == '!') && i + 1 < text.len() {
            let next_char = text[i + 1..].chars().next();
            if next_char.is_some_and(|nc| nc == ' ' || nc == '\n') {
                let sentence = text[start..=i].trim();
                if !sentence.is_empty() {
                    sentences.push(sentence);
                }
                start = i + 1;
            }
        }
    }

    // Last segment.
    let last = text[start..].trim();
    if !last.is_empty() {
        sentences.push(last);
    }

    sentences
}

fn score_sentence(sentence: &str, position: usize, total: usize) -> f64 {
    let mut score = 0.0;

    // Length bonus: prefer medium-length sentences.
    let words = sentence.split_whitespace().count();
    if (5..30).contains(&words) {
        score += 2.0;
    } else if words >= 30 {
        score += 1.0;
    }

    // Position bonus: earlier sentences tend to be more important.
    if total > 0 {
        let pos_ratio = position as f64 / total as f64;
        score += (1.0 - pos_ratio) * 2.0;
    }

    // Keyword bonus: sentences with key terms are more important.
    let lower = sentence.to_lowercase();
    let keywords = [
        "must", "should", "important", "critical", "error", "warning",
        "note", "required", "ensure", "always", "never",
    ];
    for keyword in &keywords {
        if lower.contains(keyword) {
            score += 1.5;
        }
    }

    // Code content bonus: sentences with code are likely technical and important.
    if sentence.contains('`') || sentence.contains("fn ") || sentence.contains("struct ") {
        score += 2.0;
    }

    score
}

fn estimate_tokens(text: &str) -> usize {
    text.len() / 4 + 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_content_not_compressed() {
        let result = compress_section("test", "Short.", &CompressionConfig::default());
        assert_eq!(result.ratio, 1.0);
    }

    #[test]
    fn long_content_compressed() {
        let content = "This is the first important sentence. \
                        This is less important filler text here. \
                        This is another filler sentence with no real content. \
                        Yet another sentence that adds nothing useful. \
                        Finally this must always be included because it is critical.";

        let result = compress_section(
            "test",
            content,
            &CompressionConfig {
                target_ratio: 0.5,
                ..CompressionConfig::default()
            },
        );
        assert!(result.compressed_tokens < result.original_tokens);
        // Should preserve the first sentence and the one with "must".
        assert!(result.content.contains("first important"));
    }

    #[test]
    fn sentence_splitting() {
        let text = "First sentence. Second sentence. Third one.";
        let sentences = split_sentences(text);
        assert_eq!(sentences.len(), 3);
    }
}
