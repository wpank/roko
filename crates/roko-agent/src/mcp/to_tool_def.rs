//! MCP tool-to-[`ToolDef`] converter (SS36.59).
//!
//! Converts an [`McpToolDef`] received from an MCP server into a
//! [`roko_core::tool::ToolDef`] that plugs directly into any
//! [`ToolRegistry`].

use roko_core::tool::{
    ToolCategory, ToolConcurrency, ToolDef, ToolPermission, ToolSchema, ToolSource,
};

use super::client::McpToolDef;

/// Convert an MCP tool definition into a Roko [`ToolDef`].
///
/// The tool name is prefixed with `{server_prefix}.` to avoid collisions
/// when multiple MCP servers expose identically-named tools.
///
/// MCP tools are categorised as [`ToolCategory::Mcp`] and granted
/// write permissions by default. A `readOnly: true` MCP annotation maps
/// the tool back down to read-only access.
#[must_use]
pub fn mcp_to_tool_def(mcp_tool: &McpToolDef, server_prefix: &str) -> ToolDef {
    let prefixed_name = format!("{server_prefix}.{}", mcp_tool.name);

    let description = mcp_tool
        .description
        .clone()
        .unwrap_or_else(|| format!("MCP tool: {}", mcp_tool.name));

    let schema = mcp_tool
        .input_schema
        .as_ref()
        .map_or_else(ToolSchema::any_object, |v| {
            ToolSchema::from_value(v.clone())
        });

    let annotations = mcp_tool.annotations.as_ref();
    // Accept both legacy (readOnly/openWorld) and MCP-2025 spec names
    // (readOnlyHint/openWorldHint) via the helper methods on McpToolAnnotations.
    let read_only = annotations.map_or(false, |a| a.is_read_only());
    let open_world = annotations.map_or(false, |a| a.is_open_world());
    let idempotent = annotations.and_then(|a| a.idempotent).unwrap_or(false);

    ToolDef {
        name: prefixed_name,
        description,
        parameters: schema,
        category: ToolCategory::Mcp,
        permission: ToolPermission {
            read: true,
            write: !read_only,
            exec: false,
            git: false,
            network: open_world,
        },
        timeout_ms: 60_000,
        concurrency: ToolConcurrency::Parallel,
        idempotent,
        source: ToolSource::Mcp {
            server: server_prefix.to_string(),
        },
        metadata: annotations.map(|annotations| {
            serde_json::json!({
                "mcp_annotations": annotations,
            })
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_mcp_tool() -> McpToolDef {
        McpToolDef {
            name: "read_file".to_string(),
            description: Some("Read a file from disk".to_string()),
            input_schema: Some(serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string"}
                },
                "required": ["path"]
            })),
            annotations: None,
        }
    }

    #[test]
    fn mcp_to_tool_def_prefixes_name() {
        let mcp = sample_mcp_tool();
        let def = mcp_to_tool_def(&mcp, "filesystem");
        assert_eq!(def.name, "filesystem.read_file");
    }

    #[test]
    fn mcp_to_tool_def_copies_description() {
        let mcp = sample_mcp_tool();
        let def = mcp_to_tool_def(&mcp, "fs");
        assert_eq!(def.description, "Read a file from disk");
    }

    #[test]
    fn mcp_to_tool_def_maps_input_schema() {
        let mcp = sample_mcp_tool();
        let def = mcp_to_tool_def(&mcp, "fs");
        let schema = def.parameters.as_value();
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["path"].is_object());
    }

    #[test]
    fn mcp_to_tool_def_missing_description_uses_fallback() {
        let mcp = McpToolDef {
            name: "search".to_string(),
            description: None,
            input_schema: None,
            annotations: None,
        };
        let def = mcp_to_tool_def(&mcp, "code");
        assert_eq!(def.description, "MCP tool: search");
    }

    #[test]
    fn mcp_to_tool_def_missing_schema_uses_any_object() {
        let mcp = McpToolDef {
            name: "list".to_string(),
            description: Some("List items".to_string()),
            input_schema: None,
            annotations: None,
        };
        let def = mcp_to_tool_def(&mcp, "srv");
        let schema = def.parameters.as_value();
        assert_eq!(schema["type"], "object");
        assert_eq!(schema["additionalProperties"], true);
    }

    #[test]
    fn mcp_to_tool_def_sets_mcp_category() {
        let mcp = sample_mcp_tool();
        let def = mcp_to_tool_def(&mcp, "fs");
        assert_eq!(def.category, ToolCategory::Mcp);
    }

    #[test]
    fn mcp_to_tool_def_defaults() {
        let mcp = sample_mcp_tool();
        let def = mcp_to_tool_def(&mcp, "fs");
        assert_eq!(def.timeout_ms, 60_000);
        assert_eq!(def.concurrency, ToolConcurrency::Parallel);
        assert!(!def.idempotent);
        assert!(def.permission.read);
        assert!(def.permission.write);
        assert!(!def.permission.network);
        assert_eq!(
            def.source,
            ToolSource::Mcp {
                server: "fs".to_string()
            }
        );
        assert_eq!(def.metadata, None);
    }

    #[test]
    fn mcp_to_tool_def_maps_read_only_annotation() {
        let mut mcp = sample_mcp_tool();
        mcp.annotations = Some(super::super::client::McpToolAnnotations {
            read_only: Some(true),
            read_only_hint: None,
            open_world: Some(true),
            open_world_hint: None,
            idempotent: Some(true),
            title: Some("Read file".to_string()),
        });

        let def = mcp_to_tool_def(&mcp, "fs");

        assert!(def.permission.read);
        assert!(!def.permission.write);
        assert!(def.permission.network);
        assert!(def.idempotent);
        assert_eq!(
            def.metadata.as_ref().unwrap()["mcp_annotations"]["title"],
            "Read file"
        );
    }

    #[test]
    fn mcp_to_tool_def_maps_read_only_hint_annotation() {
        // Verify that the MCP-2025 spec names (readOnlyHint / openWorldHint) are
        // accepted and produce the same permission outcome as the legacy names.
        let mut mcp = sample_mcp_tool();
        mcp.annotations = Some(super::super::client::McpToolAnnotations {
            read_only: None,
            read_only_hint: Some(true),
            open_world: None,
            open_world_hint: Some(true),
            idempotent: Some(false),
            title: None,
        });

        let def = mcp_to_tool_def(&mcp, "code");

        assert!(def.permission.read);
        assert!(
            !def.permission.write,
            "readOnlyHint:true must set write=false"
        );
        assert!(
            def.permission.network,
            "openWorldHint:true must set network=true"
        );
        assert!(!def.idempotent);
    }

    #[test]
    fn mcp_to_tool_def_legacy_and_hint_names_are_disjunctive() {
        // Either name being true is sufficient to trigger the flag.
        let mut mcp = sample_mcp_tool();
        mcp.annotations = Some(super::super::client::McpToolAnnotations {
            read_only: Some(false),
            read_only_hint: Some(true), // hint takes precedence when legacy is false
            open_world: Some(false),
            open_world_hint: Some(false),
            idempotent: None,
            title: None,
        });

        let def = mcp_to_tool_def(&mcp, "code");
        assert!(
            !def.permission.write,
            "readOnlyHint:true must override readOnly:false"
        );
        assert!(!def.permission.network);
    }
}
