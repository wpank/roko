//! Extensions bundle -- plugin, MCP, and integration runtime handles.
//!
//! This bundle is optional for all profiles. When present, it provides
//! access to loaded plugin manifests and MCP server configurations.

use std::sync::Arc;

use roko_agent::mcp::McpRuntime;
use roko_agent::provider::LocalToolRuntime;
use serde::{Deserialize, Serialize};

/// Extension runtime handles for plugins and MCP.
///
/// Optional for all profiles -- the builder populates it only when
/// plugin/MCP configuration is present and initialization succeeds.
#[derive(Debug, Clone)]
pub struct ExtensionsBundle {
    /// Pre-discovered MCP runtime (tools + clients), if available.
    pub mcp_runtime: Option<Arc<McpRuntime>>,
    /// Declarative plugin tool runtime, if loaded.
    pub local_tool_runtime: Option<Arc<LocalToolRuntime>>,
}

/// Serializable summary of the extensions bundle for diagnostics.
#[derive(Debug, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct ExtensionsBundleSummary {
    pub has_mcp_runtime: bool,
    pub mcp_tool_count: usize,
    pub has_local_tool_runtime: bool,
}

impl ExtensionsBundle {
    /// Create an empty extensions bundle for testing.
    pub fn for_test() -> Self {
        Self {
            mcp_runtime: None,
            local_tool_runtime: None,
        }
    }

    /// Produce a serializable summary for diagnostics / snapshot tests.
    pub fn summary(&self) -> ExtensionsBundleSummary {
        ExtensionsBundleSummary {
            has_mcp_runtime: self.mcp_runtime.is_some(),
            mcp_tool_count: self
                .mcp_runtime
                .as_ref()
                .map(|r| r.tools().len())
                .unwrap_or(0),
            has_local_tool_runtime: self.local_tool_runtime.is_some(),
        }
    }
}
