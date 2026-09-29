//! Maximum-iteration guard (§36.54).
//!
//! Prevents the tool loop from running indefinitely when the backend
//! keeps emitting tool calls without converging on a final answer.

pub use roko_core::defaults::DEFAULT_MAX_TOOL_ITERATIONS as DEFAULT_MAX_ITERATIONS;

use crate::provider::error_classify::TurnCapHit;

/// Returns `true` when the loop has exhausted its iteration budget.
#[inline]
#[must_use]
pub const fn is_exhausted(iterations: usize, max: usize) -> bool {
    iterations >= max
}

/// Failure text of a loop that stopped after `iterations` iterations.
///
/// A stop at the caller's `turn_cap` reads as a [`TurnCapHit`], so the
/// dispatcher resumes the partial work with a raised cap, as it does for
/// the Claude CLI's `error_max_turns`. A stop at a lower model cap keeps the
/// plain message, since raising the turn cap would not move it.
#[must_use]
pub fn exhausted_message(iterations: usize, turn_cap: Option<u32>) -> String {
    match turn_cap {
        Some(cap) if iterations >= cap as usize => TurnCapHit {
            num_turns: Some(u32::try_from(iterations).unwrap_or(u32::MAX)),
            cap: Some(cap),
        }
        .to_string(),
        _ => format!("Max iterations ({iterations}) reached"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::error_classify::detect_turn_cap;

    #[test]
    fn a_stop_at_the_turn_cap_reads_as_a_turn_cap_hit() {
        let text = exhausted_message(7, Some(7));
        assert_eq!(
            detect_turn_cap(&text),
            Some(TurnCapHit {
                num_turns: Some(7),
                cap: Some(7),
            }),
            "{text}"
        );
    }

    #[test]
    fn a_stop_below_the_turn_cap_keeps_the_plain_message() {
        for text in [exhausted_message(20, Some(60)), exhausted_message(50, None)] {
            assert!(detect_turn_cap(&text).is_none(), "{text}");
            assert!(text.starts_with("Max iterations ("), "{text}");
        }
    }

    #[test]
    fn zero_iterations_not_exhausted() {
        assert!(!is_exhausted(0, 25));
    }

    #[test]
    fn at_limit_is_exhausted() {
        assert!(is_exhausted(25, 25));
    }

    #[test]
    fn past_limit_is_exhausted() {
        assert!(is_exhausted(30, 25));
    }

    #[test]
    fn zero_limit_always_exhausted() {
        assert!(is_exhausted(0, 0));
    }
}
