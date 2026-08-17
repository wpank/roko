//! Observation-gated removal of unused tool schemas.

use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use serde::Serialize;

use crate::ToolSchema;

const OBSERVATION_THRESHOLD: u64 = 50;
const TOKENS_PER_SCHEMA_ESTIMATE: u64 = 300;
const NEVER_PRUNE_NAMES: [&str; 13] = [
    "Bash",
    "Read",
    "Write",
    "Edit",
    "Glob",
    "Grep",
    "WebSearch",
    "WebFetch",
    "TaskCreate",
    "TaskUpdate",
    "TaskList",
    "Agent",
    "SendMessage",
];

/// Result of one pruning decision.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct PruneStats {
    /// Number of schemas removed.
    pub tools_pruned: u32,
    /// Estimated prompt tokens avoided.
    pub tool_tokens_saved: u64,
}

#[derive(Default)]
struct UsageState {
    per_session_usage: HashMap<String, HashMap<String, u32>>,
    global_usage: HashMap<String, u64>,
    session_requests: HashMap<String, u64>,
    global_requests: u64,
}

/// Tracks tool use and applies session/global observation thresholds.
pub struct ToolPruner {
    state: Mutex<UsageState>,
    never_prune: HashSet<String>,
    total_tools_pruned: AtomicU64,
    total_tool_tokens_saved: AtomicU64,
}

impl Default for ToolPruner {
    fn default() -> Self {
        Self {
            state: Mutex::new(UsageState::default()),
            never_prune: NEVER_PRUNE_NAMES.into_iter().map(normalize_name).collect(),
            total_tools_pruned: AtomicU64::new(0),
            total_tool_tokens_saved: AtomicU64::new(0),
        }
    }
}

impl ToolPruner {
    /// Record a tool invocation at both session and global scope.
    pub fn record_tool_use(&self, session_id: &str, tool_name: &str) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        let name = normalize_name(tool_name);
        *state
            .per_session_usage
            .entry(session_id.to_string())
            .or_default()
            .entry(name.clone())
            .or_default() += 1;
        *state.global_usage.entry(name).or_default() += 1;
    }

    /// Count this request and return a newly filtered tool vector.
    #[must_use]
    pub fn prune(&self, session_id: &str, tools: &[ToolSchema]) -> (Vec<ToolSchema>, PruneStats) {
        let (session_requests, global_requests, session_used, global_used) = {
            let Ok(mut state) = self.state.lock() else {
                return (tools.to_vec(), PruneStats::default());
            };
            state.global_requests = state.global_requests.saturating_add(1);
            let session_requests = state
                .session_requests
                .entry(session_id.to_string())
                .or_default();
            *session_requests = session_requests.saturating_add(1);
            (
                *session_requests,
                state.global_requests,
                state
                    .per_session_usage
                    .get(session_id)
                    .cloned()
                    .unwrap_or_default(),
                state.global_usage.clone(),
            )
        };

        let session_tier = session_requests >= OBSERVATION_THRESHOLD;
        let global_tier = !session_tier && global_requests >= OBSERVATION_THRESHOLD;
        if !session_tier && !global_tier {
            return (tools.to_vec(), PruneStats::default());
        }

        let kept = tools
            .iter()
            .filter(|tool| {
                let name = normalize_name(&tool.name);
                self.never_prune.contains(&name)
                    || if session_tier {
                        session_used.contains_key(&name)
                    } else {
                        global_used.contains_key(&name)
                    }
            })
            .cloned()
            .collect::<Vec<_>>();
        let removed = tools.len().saturating_sub(kept.len());
        let stats = PruneStats {
            tools_pruned: removed.try_into().unwrap_or(u32::MAX),
            tool_tokens_saved: (removed as u64).saturating_mul(TOKENS_PER_SCHEMA_ESTIMATE),
        };
        self.total_tools_pruned
            .fetch_add(u64::from(stats.tools_pruned), Ordering::Relaxed);
        self.total_tool_tokens_saved
            .fetch_add(stats.tool_tokens_saved, Ordering::Relaxed);
        (kept, stats)
    }

    /// Aggregate pruning totals.
    #[must_use]
    pub fn totals(&self) -> PruneStats {
        PruneStats {
            tools_pruned: self
                .total_tools_pruned
                .load(Ordering::Relaxed)
                .try_into()
                .unwrap_or(u32::MAX),
            tool_tokens_saved: self.total_tool_tokens_saved.load(Ordering::Relaxed),
        }
    }
}

fn normalize_name(name: &str) -> String {
    let normalized = name
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect::<String>();
    match normalized.as_str() {
        "readfile" => "read".to_string(),
        "writefile" => "write".to_string(),
        "editfile" => "edit".to_string(),
        _ => normalized,
    }
}

#[cfg(test)]
mod tests {
    use roko_core::tool::{ToolCategory, ToolDef, ToolPermission};

    use super::*;

    fn tool(name: &str) -> ToolDef {
        ToolDef::new(name, name, ToolCategory::Read, ToolPermission::read_only())
    }

    #[test]
    fn tool_prune_waits_for_fifty_requests_and_never_removes_core_tools() {
        let pruner = ToolPruner::default();
        let tools = vec![tool("read_file"), tool("Bash"), tool("unused")];
        for _ in 0..49 {
            let (kept, stats) = pruner.prune("session", &tools);
            assert_eq!(kept.len(), 3);
            assert_eq!(stats.tools_pruned, 0);
        }
        let (kept, stats) = pruner.prune("session", &tools);
        assert_eq!(
            kept.iter()
                .map(|tool| tool.name.as_str())
                .collect::<Vec<_>>(),
            ["read_file", "Bash"]
        );
        assert_eq!(stats.tools_pruned, 1);
        assert_eq!(stats.tool_tokens_saved, 300);
    }

    #[test]
    fn tool_prune_session_usage_wins_after_threshold() {
        let pruner = ToolPruner::default();
        let tools = vec![tool("custom_used"), tool("custom_unused")];
        pruner.record_tool_use("session", "custom_used");
        for _ in 0..50 {
            let _ = pruner.prune("session", &tools);
        }
        let (kept, _) = pruner.prune("session", &tools);
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].name, "custom_used");
    }

    #[test]
    fn tool_prune_global_tier_uses_cross_session_evidence() {
        let pruner = ToolPruner::default();
        let tools = vec![tool("globally_used"), tool("never_used")];
        pruner.record_tool_use("first", "globally_used");
        for index in 0..49 {
            let _ = pruner.prune(&format!("s-{index}"), &tools);
        }
        let (kept, _) = pruner.prune("new-session", &tools);
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].name, "globally_used");
    }
}
