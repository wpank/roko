//! Bounded retry, oscillation, and token-drift detection.

use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use serde::Serialize;

const CALL_HISTORY_CAPACITY: usize = 16;
const RETRY_THRESHOLD: u32 = 5;
const DRIFT_TOKEN_THRESHOLD: u64 = 15_000;

const RETRY_GUIDANCE: &str =
    "You have called the same tool with the same arguments 5 times. Try a different approach.";
const OSCILLATION_GUIDANCE: &str = "You are oscillating between two actions. Break the loop by choosing a third option or stopping.";
const DRIFT_GUIDANCE: &str = "You have generated 15K+ tokens without making progress. Either take a concrete action or stop.";

/// Per-session bounded detection state.
#[derive(Debug, Default)]
pub struct SessionLoopState {
    /// Last sixteen `(tool_name, argument_hash)` observations.
    pub recent_calls: VecDeque<(String, [u8; 32])>,
    /// Number of identical calls at the tail of the history.
    pub consecutive_identical: u32,
    /// Generated output since the last new progress marker.
    pub tokens_since_progress: u64,
    last_progress_marker: Option<String>,
    pending_guidance: Option<String>,
}

/// Loop-detection telemetry snapshot.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct LoopStats {
    /// Total loop patterns detected.
    pub loops_detected: u64,
    /// One-shot prompts consumed by callers.
    pub loop_injections: u64,
    /// Retry-loop detections.
    pub loop_retry_detected: u64,
    /// Oscillation-loop detections.
    pub loop_oscillation_detected: u64,
    /// Token-drift detections.
    pub loop_drift_detected: u64,
}

/// Session-keyed bounded loop detector.
#[derive(Default)]
pub struct LoopDetector {
    sessions: Mutex<HashMap<String, SessionLoopState>>,
    loops_detected: AtomicU64,
    loop_injections: AtomicU64,
    loop_retry_detected: AtomicU64,
    loop_oscillation_detected: AtomicU64,
    loop_drift_detected: AtomicU64,
}

impl LoopDetector {
    /// Record a tool call and any output tokens produced since the last call.
    pub fn record_call(
        &self,
        session_id: &str,
        tool_name: impl Into<String>,
        args_hash: [u8; 32],
        output_tokens: u64,
    ) {
        let Ok(mut sessions) = self.sessions.lock() else {
            return;
        };
        let state = sessions.entry(session_id.to_string()).or_default();
        state.tokens_since_progress = state.tokens_since_progress.saturating_add(output_tokens);
        let call = (tool_name.into(), args_hash);
        state.consecutive_identical = if state.recent_calls.back() == Some(&call) {
            state.consecutive_identical.saturating_add(1)
        } else {
            1
        };
        state.recent_calls.push_back(call);
        while state.recent_calls.len() > CALL_HISTORY_CAPACITY {
            state.recent_calls.pop_front();
        }

        if state.pending_guidance.is_some() {
            return;
        }
        if state.consecutive_identical >= RETRY_THRESHOLD {
            state.pending_guidance = Some(RETRY_GUIDANCE.to_string());
            self.detected(&self.loop_retry_detected);
        } else if oscillating(&state.recent_calls) {
            state.pending_guidance = Some(OSCILLATION_GUIDANCE.to_string());
            self.detected(&self.loop_oscillation_detected);
        } else if state.tokens_since_progress >= DRIFT_TOKEN_THRESHOLD {
            state.pending_guidance = Some(DRIFT_GUIDANCE.to_string());
            self.detected(&self.loop_drift_detected);
        }
    }

    /// Record output tokens when no tool call was observed.
    pub fn record_output(&self, session_id: &str, output_tokens: u64) {
        let Ok(mut sessions) = self.sessions.lock() else {
            return;
        };
        let state = sessions.entry(session_id.to_string()).or_default();
        state.tokens_since_progress = state.tokens_since_progress.saturating_add(output_tokens);
        if state.tokens_since_progress >= DRIFT_TOKEN_THRESHOLD && state.pending_guidance.is_none()
        {
            state.pending_guidance = Some(DRIFT_GUIDANCE.to_string());
            self.detected(&self.loop_drift_detected);
        }
    }

    /// Reset drift accounting only when genuinely new tool-result content arrives.
    pub fn record_progress(&self, session_id: &str, marker: impl Into<String>) {
        let marker = marker.into();
        let Ok(mut sessions) = self.sessions.lock() else {
            return;
        };
        let state = sessions.entry(session_id.to_string()).or_default();
        if state.last_progress_marker.as_deref() != Some(marker.as_str()) {
            state.last_progress_marker = Some(marker);
            state.tokens_since_progress = 0;
        }
    }

    /// Inspect pending guidance without consuming it.
    #[must_use]
    pub fn check_loop(&self, session_id: &str) -> Option<String> {
        self.sessions
            .lock()
            .ok()
            .and_then(|sessions| sessions.get(session_id)?.pending_guidance.clone())
    }

    /// Consume pending guidance exactly once.
    pub fn take_guidance(&self, session_id: &str) -> Option<String> {
        let guidance = self
            .sessions
            .lock()
            .ok()
            .and_then(|mut sessions| sessions.get_mut(session_id)?.pending_guidance.take());
        if guidance.is_some() {
            self.loop_injections.fetch_add(1, Ordering::Relaxed);
        }
        guidance
    }

    /// Current aggregate counters.
    #[must_use]
    pub fn stats(&self) -> LoopStats {
        LoopStats {
            loops_detected: self.loops_detected.load(Ordering::Relaxed),
            loop_injections: self.loop_injections.load(Ordering::Relaxed),
            loop_retry_detected: self.loop_retry_detected.load(Ordering::Relaxed),
            loop_oscillation_detected: self.loop_oscillation_detected.load(Ordering::Relaxed),
            loop_drift_detected: self.loop_drift_detected.load(Ordering::Relaxed),
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
                    .map(|state| state.recent_calls.len())
            })
            .unwrap_or_default()
    }

    fn detected(&self, kind: &AtomicU64) {
        self.loops_detected.fetch_add(1, Ordering::Relaxed);
        kind.fetch_add(1, Ordering::Relaxed);
    }
}

fn oscillating(calls: &VecDeque<(String, [u8; 32])>) -> bool {
    if calls.len() < 6 {
        return false;
    }
    let tail = calls.iter().skip(calls.len() - 6).collect::<Vec<_>>();
    tail[0] != tail[1]
        && tail[0] == tail[2]
        && tail[0] == tail[4]
        && tail[1] == tail[3]
        && tail[1] == tail[5]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hash(byte: u8) -> [u8; 32] {
        [byte; 32]
    }

    #[test]
    fn loop_detect_retry_guidance_is_one_shot() {
        let detector = LoopDetector::default();
        for _ in 0..5 {
            detector.record_call("s", "Read", hash(1), 10);
        }
        assert_eq!(detector.check_loop("s").as_deref(), Some(RETRY_GUIDANCE));
        assert_eq!(detector.take_guidance("s").as_deref(), Some(RETRY_GUIDANCE));
        assert_eq!(detector.take_guidance("s"), None);
        assert_eq!(detector.stats().loop_retry_detected, 1);
        assert_eq!(detector.stats().loop_injections, 1);
    }

    #[test]
    fn loop_detect_oscillation_requires_three_full_cycles() {
        let detector = LoopDetector::default();
        for (tool, byte) in [("A", 1), ("B", 2), ("A", 1), ("B", 2), ("A", 1)] {
            detector.record_call("s", tool, hash(byte), 0);
        }
        assert_eq!(detector.check_loop("s"), None);
        detector.record_call("s", "B", hash(2), 0);
        assert_eq!(
            detector.take_guidance("s").as_deref(),
            Some(OSCILLATION_GUIDANCE)
        );
    }

    #[test]
    fn loop_detect_drift_resets_only_for_new_progress() {
        let detector = LoopDetector::default();
        detector.record_progress("s", "result-a");
        detector.record_output("s", 14_999);
        assert_eq!(detector.check_loop("s"), None);
        detector.record_progress("s", "result-a");
        detector.record_output("s", 1);
        assert_eq!(detector.take_guidance("s").as_deref(), Some(DRIFT_GUIDANCE));

        detector.record_progress("s", "result-b");
        detector.record_output("s", 1);
        assert_eq!(detector.check_loop("s"), None);
    }

    #[test]
    fn loop_detect_history_is_strictly_bounded() {
        let detector = LoopDetector::default();
        for byte in 0..64 {
            detector.record_call("s", format!("tool-{byte}"), hash(byte), 0);
        }
        assert_eq!(detector.history_len("s"), 16);
    }
}
