//! Argument validation against a tool's JSON schema (§36.42).
//!
//! Feeds `call.arguments` through [`ToolRegistry::validate_args`] and
//! maps the crate-local error into a [`ToolError`] the dispatcher can
//! return directly:
//!
//! - **Unknown tool** (validator's `"unknown tool: …"` message) →
//!   [`ToolError::Other`]. This lets the dispatcher produce a uniform
//!   "no such tool" error whether the name wasn't registered or the
//!   registry's schema check rejected it for another reason.
//! - **Schema mismatch** → [`ToolError::SchemaInvalid`] carrying the
//!   validator's message verbatim (so the LLM can self-correct).
//!
//! This module runs **before** the handler is ever invoked — a failed
//! validation burns zero handler budget.

use roko_core::tool::{ToolCall, ToolError, ToolRegistry};

/// Validate a call's arguments against the registry's schema.
///
/// Returns `Ok(())` on success; on failure, returns the appropriate
/// [`ToolError`] variant ([`SchemaInvalid`](ToolError::SchemaInvalid) for
/// bad args, [`Other`](ToolError::Other) for an unregistered name).
pub fn validate(call: &ToolCall, registry: &dyn ToolRegistry) -> Result<(), ToolError> {
    match registry.validate_args(&call.name, &call.arguments) {
        Ok(()) => Ok(()),
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("unknown tool") {
                Err(ToolError::Other(format!("unknown tool: {}", call.name)))
            } else {
                Err(ToolError::SchemaInvalid(msg))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use roko_core::tool::{ToolCall, ToolCategory, ToolDef, ToolPermission, VecToolRegistry};

    fn read_file_def() -> ToolDef {
        ToolDef::new(
            "read_file",
            "read a file",
            ToolCategory::Read,
            ToolPermission::read_only(),
        )
    }

    #[test]
    fn validate_unknown_tool_returns_other() {
        let registry = VecToolRegistry::from_tools(vec![read_file_def()]);
        let call = ToolCall::new("c1", "does_not_exist", serde_json::json!({}));
        let err = validate(&call, &registry).expect_err("expected Err for unknown tool");
        match err {
            ToolError::Other(msg) => assert!(msg.contains("does_not_exist")),
            other => panic!("expected Other, got {other:?}"),
        }
    }

    #[test]
    fn validate_known_tool_passes_default_registry() {
        let registry = VecToolRegistry::from_tools(vec![read_file_def()]);
        let call = ToolCall::new("c1", "read_file", serde_json::json!({"path": "x.rs"}));
        validate(&call, &registry).expect("default validate_args should accept known tool");
    }
}
