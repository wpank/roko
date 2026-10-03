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
//! [`mcp_to_tool_def`]: crate::mcp::to_tool_def::mcp_to_tool_def

use roko_core::tool::{ToolDef, ToolSource};
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
        assert!(is_outbound_effect(&silent), "an unknown effect fails closed");

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
