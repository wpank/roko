//! Which tool calls act on the outside world (9130).
//!
//! An outbound effect sends, posts, pays or changes a remote system: the
//! kind of call decision 9107 holds for approval in the ops domain and in
//! chat-origin runs (9131). [`is_outbound_effect`] classifies a tool from
//! what its [`ToolDef`] already says, so `ToolDef` and `ToolPermission` keep
//! their fields:
//!
//! - an MCP tool is outbound when it is destructive, or open-world and not
//!   read-only. Its annotations are the `mcp_annotations` that
//!   [`mcp_to_tool_def`] keeps in `ToolDef.metadata`, legacy names
//!   included, and a hint it leaves out takes the MCP spec's default: open
//!   world, and destructive unless read-only. A tool that does not describe
//!   itself therefore counts as outbound: an unknown effect fails closed;
//! - a plugin tool is outbound when it declares both network and write
//!   access;
//! - built-in tools are not outbound: `web_fetch` and `web_search` only
//!   read.
//!
//! A run's [`OutboundPolicy`] says what happens to such a call (9131). The
//! dispatcher reads it from the run's contract
//! ([`AgentContract::outbound_policy`]) once the call has passed every
//! check: `allow` runs it, `deny` refuses it, and `stage` writes an
//! [`EffectHold`] under the workspace's `.roko/state/effect-holds/<run>/`
//! instead of running it. Coverage is the in-process tool loops: CLI agents
//! such as Claude Code and Codex run their own tools, which only an MCP
//! proxy could hold.
//!
//! [`mcp_to_tool_def`]: crate::mcp::to_tool_def::mcp_to_tool_def
//! [`AgentContract::outbound_policy`]: crate::safety::contract::AgentContract::outbound_policy

use std::io::Write as _;
use std::path::{Path, PathBuf};

pub use roko_core::tool::OutboundPolicy;
use roko_core::tool::{ToolCall, ToolContext, ToolDef, ToolSource};
use roko_fs::RokoLayout;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Whether a call of `tool` has an outbound effect ([module docs](self)).
#[must_use]
pub fn is_outbound_effect(tool: &ToolDef) -> bool {
    match &tool.source {
        ToolSource::Mcp { .. } => mcp_outbound(tool.metadata.as_ref()),
        ToolSource::Plugin { .. } => tool.permission.network && tool.permission.write,
        _ => false,
    }
}

/// Whether an MCP tool whose `ToolDef.metadata` is `metadata` has an
/// outbound effect.
fn mcp_outbound(metadata: Option<&Value>) -> bool {
    let annotations = metadata.and_then(|metadata| metadata.get("mcp_annotations"));
    // A hint under any of `names`: true when one of them says so, false when
    // they all say false, and `default` when none is set.
    let hint = |names: &[&str], default: bool| {
        let said: Vec<bool> = names
            .iter()
            .filter_map(|name| annotations?.get(*name)?.as_bool())
            .collect();
        match said.as_slice() {
            [] => default,
            said => said.contains(&true),
        }
    };
    let read_only = hint(&["readOnly", "readOnlyHint"], false);
    let open_world = hint(&["openWorld", "openWorldHint"], true);
    let destructive = hint(&["destructiveHint"], !read_only);
    destructive || (open_world && !read_only)
}

/// A tool call that acts on the outside world, held for a person's approval
/// instead of run (9131): the JSON file
/// `.roko/state/effect-holds/<run>/<effect_id>.json`, readable by its owner
/// alone. It holds the call's arguments as the agent gave them, so that an
/// approval can replay the call: never print them, since they may carry
/// secrets.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EffectHold {
    /// The hold's id, which an approval or a rejection names.
    pub effect_id: String,
    /// The run whose agent proposed the call.
    pub run_id: String,
    /// The plan of the task whose agent proposed the call.
    pub plan_id: String,
    /// The task whose agent proposed the call.
    pub task_id: String,
    /// The task's attempt, from 1; 0 when the call names no attempt.
    pub attempt: u32,
    /// The tool, by the name the agent called it.
    pub tool: String,
    /// The MCP server of an MCP tool.
    pub server: Option<String>,
    /// The call's arguments.
    pub arguments: Value,
    /// When the agent proposed the call, in RFC 3339.
    pub proposed_at: String,
}

impl EffectHold {
    /// The hold of `call`, a call of `tool` that `ctx`'s attempt proposes
    /// now. The attempt comes from its key, `{run}:{plan}:{task}:{attempt}`,
    /// when the call's correlation carries one.
    #[must_use]
    pub fn propose(tool: &ToolDef, call: &ToolCall, ctx: &ToolContext) -> Self {
        let correlation = &ctx.correlation;
        let key: Vec<&str> = correlation.attempt_id.split(':').collect();
        let (run_id, plan_id, task_id, attempt) = match key.as_slice() {
            [run, plan, task, attempt] => (
                (*run).to_string(),
                (*plan).to_string(),
                (*task).to_string(),
                attempt.parse().unwrap_or(0),
            ),
            _ => (
                correlation.run_id.clone(),
                String::new(),
                correlation.task_id.clone(),
                0,
            ),
        };
        let server = match &tool.source {
            ToolSource::Mcp { server } => Some(server.clone()),
            _ => None,
        };
        let now = chrono::Utc::now();
        let stamp = now.format("%Y%m%dT%H%M%S");
        Self {
            effect_id: format!("effect-{stamp}-{:08x}", rand::random::<u32>()),
            run_id,
            plan_id,
            task_id,
            attempt,
            tool: call.name.clone(),
            server,
            arguments: call.arguments.clone(),
            proposed_at: now.to_rfc3339(),
        }
    }

    /// Where the hold lives in the workspace at `root`: under its run, or
    /// under `unscoped` for a call that names no run.
    #[must_use]
    pub fn path(&self, root: &Path) -> PathBuf {
        let run = Some(self.run_id.as_str())
            .filter(|run| !run.is_empty())
            .unwrap_or("unscoped");
        RokoLayout::for_project(root).effect_hold(run, &self.effect_id)
    }

    /// Write the hold into the workspace at `root`, readable by its owner
    /// alone, and return its path.
    ///
    /// # Errors
    ///
    /// Fails when the hold's directory or file cannot be written, or a hold
    /// with its id already exists.
    pub fn write(&self, root: &Path) -> std::io::Result<PathBuf> {
        let path = self.path(root);
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let json = serde_json::to_vec_pretty(self)?;
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
        options.open(&path)?.write_all(&json)?;
        Ok(path)
    }
}

#[cfg(test)]
mod tests {
    use roko_core::tool::{ToolCategory, ToolConcurrency, ToolPermission, ToolSchema};
    use serde_json::json;

    use super::*;
    use crate::mcp::client::McpToolDef;
    use crate::mcp::to_tool_def::mcp_to_tool_def;

    /// An MCP tool `send` whose annotations are `annotations`, as an MCP
    /// server lists it, converted as the dispatcher's registry converts it.
    fn mcp_tool(annotations: Value) -> ToolDef {
        let listed = json!({
            "name": "send",
            "description": "Send a message",
            "annotations": annotations,
        });
        let mcp: McpToolDef = serde_json::from_value(listed).expect("an MCP tool");
        mcp_to_tool_def(&mcp, "mail")
    }

    /// 9130: an open-world MCP tool that is not read-only is outbound, a
    /// read-only one is not, `destructiveHint: true` is outbound, a closed
    /// world additive tool is not, and a tool with no annotations fails
    /// closed. Plugin tools need network and write; built-in tools are not
    /// outbound.
    #[test]
    fn destructive_open_world_mcp_tool_is_outbound_effect() {
        let open_world = mcp_tool(json!({ "readOnlyHint": false, "openWorldHint": true }));
        assert!(is_outbound_effect(&open_world));
        let read_only = mcp_tool(json!({ "readOnlyHint": true, "openWorldHint": true }));
        assert!(!is_outbound_effect(&read_only));
        let legacy_read_only = mcp_tool(json!({ "readOnly": true }));
        assert!(!is_outbound_effect(&legacy_read_only));
        let destructive = mcp_tool(json!({ "openWorldHint": false, "destructiveHint": true }));
        assert!(is_outbound_effect(&destructive));
        let additive = mcp_tool(json!({ "openWorldHint": false, "destructiveHint": false }));
        assert!(!is_outbound_effect(&additive));
        let silent = mcp_tool(Value::Null);
        assert!(
            is_outbound_effect(&silent),
            "an unknown effect fails closed"
        );

        let plugin = |network: bool, write: bool| ToolDef {
            name: "post".to_string(),
            description: "Post an update".to_string(),
            parameters: ToolSchema::any_object(),
            category: ToolCategory::Mcp,
            permission: ToolPermission {
                read: true,
                write,
                exec: false,
                git: false,
                network,
            },
            timeout_ms: 60_000,
            concurrency: ToolConcurrency::Parallel,
            idempotent: false,
            source: ToolSource::Plugin {
                name: "social".to_string(),
            },
            metadata: None,
        };
        assert!(is_outbound_effect(&plugin(true, true)));
        assert!(!is_outbound_effect(&plugin(true, false)));
        assert!(!is_outbound_effect(&plugin(false, true)));
        let mut builtin = plugin(true, true);
        builtin.source = ToolSource::Builtin;
        assert!(!is_outbound_effect(&builtin));
    }
}
