//! Bounded response-convergence detection.

use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use serde::Serialize;

use crate::cache::hamming_distance;

const HISTORY_CAPACITY: usize = 8;
const SIMILAR_DISTANCE: u32 = 2;
const SIMILAR_THRESHOLD: u32 = 3;
const GUIDANCE: &str =
    "Your recent responses are converging. Try a different angle or move to the next step.";

/// Per-session convergence history.
#[derive(Debug, Default)]
pub struct ConvergenceState {
    /// Last eight response fingerprints.
    pub recent_hashes: VecDeque<u64>,
    /// Current run of near-identical responses.
    pub consecutive_similar: u32,
    pending_guidance: Option<String>,
    alerted_until_dissimilar: bool,
}

/// Convergence telemetry.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct ConvergenceStats {
    /// Convergent runs detected.
    pub convergence_detected: u64,
    /// Guidance prompts consumed.
    pub convergence_injections: u64,
}

/// Session-keyed convergence detector.
#[derive(Default)]
pub struct ConvergenceDetector {
    sessions: Mutex<HashMap<String, ConvergenceState>>,
    convergence_detected: AtomicU64,
    convergence_injections: AtomicU64,
}

impl ConvergenceDetector {
    /// Record one response SimHash.
    pub fn record_response(&self, session_id: &str, response_simhash: u64) {
        let Ok(mut sessions) = self.sessions.lock() else {
            return;
        };
        let state = sessions.entry(session_id.to_string()).or_default();
        state.consecutive_similar = match state.recent_hashes.back() {
            None => 1,
            Some(previous) if hamming_distance(*previous, response_simhash) <= SIMILAR_DISTANCE => {
                state.consecutive_similar.saturating_add(1)
            }
            Some(_) => {
                state.alerted_until_dissimilar = false;
                state.pending_guidance = None;
                0
            }
        };
        state.recent_hashes.push_back(response_simhash);
        while state.recent_hashes.len() > HISTORY_CAPACITY {
            state.recent_hashes.pop_front();
        }
        if state.consecutive_similar >= SIMILAR_THRESHOLD && !state.alerted_until_dissimilar {
            state.pending_guidance = Some(GUIDANCE.to_string());
            state.alerted_until_dissimilar = true;
            self.convergence_detected.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Consume convergence guidance exactly once per convergent run.
    pub fn take_guidance(&self, session_id: &str) -> Option<String> {
        let guidance = self
            .sessions
            .lock()
            .ok()
            .and_then(|mut sessions| sessions.get_mut(session_id)?.pending_guidance.take());
        if guidance.is_some() {
            self.convergence_injections.fetch_add(1, Ordering::Relaxed);
        }
        guidance
    }

    /// Current aggregate counters.
    #[must_use]
    pub fn stats(&self) -> ConvergenceStats {
        ConvergenceStats {
            convergence_detected: self.convergence_detected.load(Ordering::Relaxed),
            convergence_injections: self.convergence_injections.load(Ordering::Relaxed),
        }
    }

    /// Bounded history size for diagnostics/tests.
    #[must_use]
    pub fn history_len(&self, session_id: &str) -> usize {
        self.sessions
            .lock()
            .ok()
            .and_then(|sessions| {
                sessions
                    .get(session_id)
                    .map(|state| state.recent_hashes.len())
            })
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convergence_fires_after_three_similar_responses_and_once() {
        let detector = ConvergenceDetector::default();
        detector.record_response("s", 0b0000);
        detector.record_response("s", 0b0001);
        detector.record_response("s", 0b0011);
        assert_eq!(detector.take_guidance("s").as_deref(), Some(GUIDANCE));
        assert_eq!(detector.take_guidance("s"), None);
        detector.record_response("s", 0b0010);
        assert_eq!(detector.take_guidance("s"), None);
    }

    #[test]
    fn convergence_dissimilarity_resets_and_history_is_bounded() {
        let detector = ConvergenceDetector::default();
        for hash in [0, 1, 3, u64::MAX, u64::MAX - 1, u64::MAX - 3, u64::MAX - 7] {
            detector.record_response("s", hash);
        }
        assert_eq!(detector.stats().convergence_detected, 2);
        for hash in 0..20 {
            detector.record_response("bounded", hash);
        }
        assert_eq!(detector.history_len("bounded"), 8);
    }
}
