//! Unified log cache builder and signal/episode/log-entry loading.
//!
//! The `build_unified_log_cache` function merges multiple event streams
//! (signals, episodes, efficiency events, gate failures, event log) into
//! a single, time-ordered `Vec<LogEntry>` bounded by `MAX_UNIFIED_LOG`.

use std::collections::BTreeMap;
use std::time::Instant;

use ratatui::text::Line;

use super::super::dashboard::Theme;
use super::super::segment::{CachedRender, output_byte_len, render_cached_output};
use super::{LogEntry, LogEntryLevel, MAX_AGENT_STREAM_CHUNKS, MAX_UNIFIED_LOG, TuiState};
use crate::tui::display_utils::truncate as truncate_log;

// ---------------------------------------------------------------------------
// Unified log cache builder
// ---------------------------------------------------------------------------

pub(super) fn build_unified_log_cache(tui_state: &TuiState) -> Vec<LogEntry> {
    let mut entries: BTreeMap<(i64, usize), LogEntry> = BTreeMap::new();
    let mut seq = 0usize;

    for signal in &tui_state.recent_signals {
        let level = if signal.kind.contains("error") || signal.kind.contains("fail") {
            LogEntryLevel::Error
        } else if signal.kind.contains("warn") {
            LogEntryLevel::Warn
        } else if signal.kind.contains("gate:") {
            if signal.payload_preview.contains("passed") {
                LogEntryLevel::Info
            } else {
                // Gate failures are ERR severity.
                LogEntryLevel::Error
            }
        } else if signal.kind.contains("model_select") || signal.kind.contains("model_route") {
            // Model selection events are DEBUG unless there was a fallback.
            if signal.payload_preview.contains("fallback") {
                LogEntryLevel::Info
            } else {
                LogEntryLevel::Debug
            }
        } else if signal.kind.contains("debug") {
            LogEntryLevel::Debug
        } else {
            LogEntryLevel::Info
        };

        let message = if signal.payload_preview.is_empty() {
            signal.kind.clone()
        } else {
            truncate_log(&signal.payload_preview, 120)
        };

        entries.insert(
            (signal.created_at_ms, seq),
            LogEntry::new(
                format_log_timestamp_ms(signal.created_at_ms),
                level,
                format!("signal:{}", truncate_log_kind(&signal.kind)),
                message,
            ),
        );
        seq += 1;
    }

    for episode in &tui_state.episodes_cache {
        let ts_ms = episode.timestamp.timestamp_millis();
        let level = if !episode.success {
            // Agent crashes/errors and gate failures are ERR.
            LogEntryLevel::Error
        } else if episode.kind == "gate" {
            // Successful gates are INFO, not WARN.
            LogEntryLevel::Info
        } else {
            // Task completions are INFO.
            LogEntryLevel::Info
        };
        let duration_str = if episode.duration_secs > 0.0 {
            format!(" ({:.1}s)", episode.duration_secs)
        } else {
            String::new()
        };
        let gate_summary = if !episode.gate_verdicts.is_empty() {
            let passed = episode
                .gate_verdicts
                .iter()
                .filter(|gate| gate.passed)
                .count();
            let total = episode.gate_verdicts.len();
            format!(" gates:{passed}/{total}")
        } else {
            String::new()
        };
        let model = if episode.model.is_empty() {
            String::new()
        } else {
            format!("model={}", episode.model)
        };
        let message = format!(
            "{} [{}] task={}{}{} {}",
            episode.kind,
            if episode.success { "ok" } else { "FAIL" },
            truncate_log(&episode.task_id, 30),
            duration_str,
            gate_summary,
            model,
        );

        entries.insert(
            (ts_ms, seq),
            LogEntry::new(
                episode.timestamp.format("%H:%M:%S").to_string(),
                level,
                format!("episode:{}", truncate_log_kind(&episode.kind)),
                message,
            ),
        );
        seq += 1;
    }

    for event in &tui_state.efficiency_events {
        let ts_ms = chrono::DateTime::parse_from_rfc3339(&event.timestamp)
            .map(|dt| dt.timestamp_millis())
            .unwrap_or_else(|_| chrono::Utc::now().timestamp_millis());
        let level = if event.cost_usd > 1.0 {
            LogEntryLevel::Warn
        } else {
            LogEntryLevel::Debug
        };
        let cache_pct = if event.input_tokens > 0 {
            format!(
                " cache:{:.0}%",
                event.cache_read_tokens as f64 / event.input_tokens as f64 * 100.0
            )
        } else {
            String::new()
        };
        let message = format!(
            "{} model={} in={} out={} ${:.4} {}ms{}",
            event.role,
            truncate_log(&event.model, 20),
            format_log_count(event.input_tokens),
            format_log_count(event.output_tokens),
            event.cost_usd,
            event.duration_ms,
            cache_pct,
        );

        entries.insert(
            (ts_ms, seq),
            LogEntry::new(
                format_log_timestamp_ms(ts_ms),
                level,
                format!("efficiency:{}", truncate_log(&event.agent_id, 12)),
                message,
            ),
        );
        seq += 1;
    }

    for failure in &tui_state.gate_results_page.failure_rows {
        entries.insert(
            (failure.created_at_ms, seq),
            LogEntry::new(
                format_log_timestamp_ms(failure.created_at_ms),
                LogEntryLevel::Error,
                format!("gate:{}", failure.gate_name),
                format!(
                    "FAILED task={} {}",
                    failure.task_id,
                    truncate_log(&failure.error_excerpt, 80),
                ),
            ),
        );
        seq += 1;
    }

    for event in &tui_state.event_log {
        let ts_ms = event.timestamp_ms as i64;
        let level = match event.event_type.as_str() {
            "error" | "task_failed" | "gate_failed" => LogEntryLevel::Error,
            "warning" | "retry" => LogEntryLevel::Warn,
            "debug" => LogEntryLevel::Debug,
            _ => LogEntryLevel::Info,
        };
        let detail = if event.task_id.is_empty() {
            event.message.clone()
        } else {
            format!("[{}] {}", event.task_id, event.message)
        };
        entries.insert(
            (ts_ms, seq),
            LogEntry::new(
                format_log_timestamp_ms(ts_ms),
                level,
                format!("event:{}", truncate_log(&event.event_type, 16)),
                detail,
            ),
        );
        seq += 1;
    }

    let all: Vec<LogEntry> = entries.into_values().collect();
    let len = all.len();
    if len > MAX_UNIFIED_LOG {
        all.into_iter().skip(len - MAX_UNIFIED_LOG).collect()
    } else {
        all
    }
}

fn format_log_timestamp_ms(ms: i64) -> String {
    chrono::DateTime::<chrono::Utc>::from_timestamp_millis(ms)
        .map(|dt| dt.format("%H:%M:%S").to_string())
        .unwrap_or_else(|| String::from("??:??:??"))
}

fn truncate_log_kind(kind: &str) -> String {
    let parts: Vec<&str> = kind.split(':').collect();
    if parts.len() <= 2 {
        kind.to_string()
    } else {
        parts[parts.len() - 2..].join(":")
    }
}

pub(super) fn format_log_count(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

// ---------------------------------------------------------------------------
// TuiState methods for unified log, agent output, and streams
// ---------------------------------------------------------------------------

impl TuiState {
    /// Return the cached unified log entries (read-only).
    #[must_use]
    pub fn unified_log_entries(&self) -> &[LogEntry] {
        &self.cached_unified_log
    }

    /// Rebuild the unified log cache only when input revisions have changed.
    ///
    /// Each input collection (signals, episodes, efficiency events, gate results,
    /// event log) has a monotonic revision counter. The unified log is only
    /// rebuilt when the combined revision hash differs from the last build.
    pub fn refresh_cached_unified_log(&mut self) {
        let combined = self.rev_signals.get()
            ^ self.rev_episodes.get().wrapping_mul(31)
            ^ self.rev_efficiency.get().wrapping_mul(37)
            ^ self.rev_gate_results.get().wrapping_mul(41)
            ^ self.rev_event_log.get().wrapping_mul(43);
        if combined == self.unified_log_input_rev && !self.cached_unified_log.is_empty() {
            return;
        }
        self.unified_log_input_rev = combined;
        let prev_len = self.cached_unified_log.len();
        self.cached_unified_log = build_unified_log_cache(self);
        let new_len = self.cached_unified_log.len();
        if new_len < prev_len {
            self.eviction_counters.unified_log += (prev_len - new_len) as u64;
        }
    }

    /// Force a full rebuild of the unified log cache regardless of revision state.
    ///
    /// Used by tests and initial bootstrap where revision tracking is bypassed.
    pub fn force_refresh_cached_unified_log(&mut self) {
        self.cached_unified_log = build_unified_log_cache(self);
        self.unified_log_input_rev = self.rev_signals.get()
            ^ self.rev_episodes.get().wrapping_mul(31)
            ^ self.rev_efficiency.get().wrapping_mul(37)
            ^ self.rev_gate_results.get().wrapping_mul(41)
            ^ self.rev_event_log.get().wrapping_mul(43);
    }

    /// Return cached, styled agent output lines for the selected agent pane.
    #[must_use]
    pub fn render_agent_output_lines(
        &self,
        cache_key: &str,
        raw_output: &[String],
        theme: &Theme,
    ) -> Vec<Line<'static>> {
        if raw_output.is_empty() {
            if !cache_key.is_empty() {
                self.agent_output_cache.borrow_mut().remove(cache_key);
            }
            return Vec::new();
        }

        let cache_key = if cache_key.is_empty() {
            "__agent-output__"
        } else {
            cache_key
        };
        let output_len = output_byte_len(raw_output);
        let mut cache = self.agent_output_cache.borrow_mut();
        let cached = cache
            .entry(cache_key.to_string())
            .or_insert_with(CachedRender::default);

        if cached.last_len != output_len {
            *cached = render_cached_output(raw_output, theme);
        }

        let lines = cached.styled_lines.clone();

        // Evict the entire render cache when it grows beyond 64 entries.
        // This is a render-only cache -- entries rebuild on the next frame.
        const MAX_CACHE_ENTRIES: usize = 64;
        if cache.len() > MAX_CACHE_ENTRIES {
            cache.clear();
        }

        lines
    }

    /// Append one streamed chunk for the given agent, trimming to the last 200 entries.
    pub fn push_agent_chunk(&mut self, agent_id: &str, chunk: String) {
        let stream = self.agent_streams.entry(agent_id.to_string()).or_default();
        while stream.chunks.len() >= MAX_AGENT_STREAM_CHUNKS {
            stream.chunks.pop_front();
        }
        stream.chunks.push_back(chunk);
        stream.connected = true;
        stream.completed = false;
        stream.last_chunk_at = Some(Instant::now());
    }

    /// Push a typed `AgentOutputRecord` into `agent_output_history` for the
    /// given agent (P1-TUI-G4).  This is the canonical write path for
    /// streaming events received via `DashboardEvent::AgentOutput` or the
    /// per-agent sidecar WebSocket client; it ensures the structured renderer
    /// always sees up-to-date typed records rather than falling back to legacy
    /// raw-text collect paths.
    pub fn push_agent_output_record(
        &mut self,
        agent_id: &str,
        kind: super::OutputRecordKind,
        text: String,
        tool_id: Option<String>,
        tool_name: Option<String>,
    ) {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        self.agent_output_history.push(
            agent_id,
            super::AgentOutputRecord {
                seq: 0, // assigned by push()
                timestamp_ms: now_ms,
                role: "assistant".to_string(),
                kind,
                text,
                redacted: false,
                tool_id,
                tool_name,
            },
        );
    }

    /// Mark the agent's live stream as connected.
    pub fn mark_agent_stream_connected(&mut self, agent_id: &str) {
        let stream = self.agent_streams.entry(agent_id.to_string()).or_default();
        stream.connected = true;
    }

    /// Mark the agent's live stream as disconnected.
    pub fn mark_agent_stream_disconnected(&mut self, agent_id: &str) {
        let stream = self.agent_streams.entry(agent_id.to_string()).or_default();
        stream.connected = false;
    }

    /// Mark the agent's live stream as completed (no more output expected).
    pub fn mark_agent_stream_done(&mut self, agent_id: &str) {
        let stream = self.agent_streams.entry(agent_id.to_string()).or_default();
        stream.completed = true;
        stream.connected = false;
    }

    pub(super) fn prune_agent_output_cache(&self) {
        use std::collections::HashSet;
        let valid_ids = self
            .agents
            .iter()
            .map(|agent| agent.id.as_str())
            .collect::<HashSet<_>>();
        self.agent_output_cache
            .borrow_mut()
            .retain(|key, _| key == "__agent-output__" || valid_ids.contains(key.as_str()));
    }

    pub(super) fn prune_agent_streams(&mut self) {
        use std::collections::HashSet;
        let valid_ids = self
            .agents
            .iter()
            .map(|agent| agent.id.as_str())
            .collect::<HashSet<_>>();
        self.agent_streams
            .retain(|agent_id, _| valid_ids.contains(agent_id.as_str()));
    }
}
