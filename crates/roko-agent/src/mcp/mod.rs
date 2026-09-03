//! MCP (Model Context Protocol) integration (SS36.58-36.62).
//!
//! Provides a JSON-RPC stdio client for MCP servers, tool conversion from
//! MCP definitions to [`roko_core::tool::ToolDef`], multi-server dedup,
//! `.mcp.json` config discovery, and a dynamic registry that composes
//! static built-in tools with MCP-discovered tools.

pub mod bridge;
pub mod client;
pub mod config;
pub mod dedup;
pub mod dynamic_registry;
pub mod error_accumulator;
pub mod handler;
pub mod to_tool_def;

pub use bridge::{
    McpBridgeError, McpLifecycleState, McpRuntime, McpRuntimeClient, McpRuntimeTransport,
    McpTestReport, McpTestStageResult, McpTestStatus, discover_mcp_runtime, discover_mcp_tools,
    test_mcp_server,
};
pub use client::{
    MCP_PROTOCOL_VERSION, McpClient, McpRequest, McpResponse, McpToolAnnotations, McpToolDef,
    McpToolResult, StdioTransport, Transport,
};
pub use config::{
    DEFAULT_ALLOWED_COMMANDS, McpConfig, McpServerConfig, McpTransportConfig,
    SENSITIVE_ENV_PATTERNS, find_mcp_config, hardcoded_secret_values, is_command_allowed,
    is_command_on_path, sensitive_env_keys, unset_env_var_refs,
};
pub use dedup::dedup_tools;
pub use dynamic_registry::{DynamicToolRegistry, MergedToolRegistry};
pub use error_accumulator::{McpErrorAccumulator, McpErrorRecord};
pub use handler::{McpHandlerResolver, McpToolHandler};
pub use to_tool_def::mcp_to_tool_def;
