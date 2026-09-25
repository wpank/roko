# Plugin SDK

> Depth file for [19-TOOLS-PLUGINS](../../19-TOOLS-PLUGINS.md) -- the five-tier
> SPI for extending Roko with prompts, profiles, declarative tools, native trait
> implementations, and WASM sandboxed extensions.

---

## 1. Overview

The `roko-plugin` crate provides the extension SDK. Plugins are manifest-first:
a TOML manifest declares the plugin's identity, tier, capabilities,
dependencies, and entrypoint. The runtime validates the manifest before
activation and applies tier-appropriate sandboxing.

**Crate location:** `crates/roko-plugin/`

**Key modules:**
- `manifest.rs` -- TOML manifest parsing, validation, and normalization
- `registry.rs` -- signed package envelopes, checksums, WASM hook validation
- `tool_registry.rs` -- plugin-declared tool registration
- `dependency.rs` -- semantic version resolution across dependency graphs
- `trigger_protocol.rs` -- event source trigger declarations

---

## 2. Five-Tier SPI

| Tier | Extension Shape | Sandbox | Payload |
|---|---|---|---|
| 1 | Prompt/template bundle | None (pure data) | Markdown, role prompts |
| 2 | Configuration profile | None (pure data) | Tool profiles, model presets |
| 3 | Declarative tool or MCP | Tool safety layer | subprocess wrappers, MCP servers |
| 4 | Native trait implementation | Process + ABI isolation | Substrate, Gate, Scorer |
| 5 | WASM sandboxed extension | Capability sandbox | Bounded logic with host imports |

The loader selects the lowest tier that can satisfy the requested capability.
Higher tiers get broader authority but stricter admission.

---

## 3. Manifest Format

```toml
[plugin]
name = "my-plugin"
version = "1.0.0"
description = "A description of the plugin"
author = "Author Name"

# Tier 1: Prompt templates
[[prompts]]
name = "pr-review"
role = "reviewer"
template = "You are a code reviewer..."

# Tier 2: Tool profile bundles
[[profiles]]
name = "read-only"
allowed_tools = ["read_file", "grep", "glob", "web_search"]
denied_tools = ["bash", "write_file", "edit_file"]

# Tier 3: Declarative tool definitions
[[tools]]
name = "lint-check"
description = "Run linter on the current file"
command = "cargo clippy -- -D warnings"
timeout_ms = 30000

# Event source triggers
[[triggers]]
kind = "cron"
expression = "0 */5 * * * *"
description = "Run every 5 minutes"
```

The manifest supports:
- Plugin metadata (name, version, description, author)
- Prompt template declarations with role bindings
- Profile bundles with allow/deny tool lists
- Declarative tool definitions with subprocess commands
- Event source trigger bindings (cron, file_watch)
- Dependency declarations with semver requirements

---

## 4. Plugin Capabilities

```rust
pub enum PluginCapability {
    PromptTemplate,
    ToolProfile,
    DeclarativeTool,
    McpServer,
    EventSource,
    WasmExtension,
}
```

Capabilities are declared in the manifest and validated at admission time. A
plugin that declares `DeclarativeTool` but provides no `[[tools]]` section is
rejected.

---

## 5. Dependency Resolution

The `dependency.rs` module provides semantic version resolution across plugin
dependency graphs:

- Each plugin declares dependencies as `name = "version_req"` pairs
- The resolver constructs a DAG and checks for cycles
- Version requirements use standard semver syntax (`^1.0`, `>=2.0, <3.0`)
- The relay and CLI both validate dependency graphs before installation

The `RegistryPackage` wire format carries `dependency_requirements: BTreeMap<String, String>` so the installer can verify range compatibility without
re-fetching manifests.

---

## 6. Signed Package Contract

The `registry.rs` module implements the signed extension registry:

```rust
pub struct RegistryPackage {
    pub schema_version: u32,        // currently 2
    pub name: String,
    pub version: String,
    pub sha256: String,             // deterministic package checksum
    pub capabilities: Vec<String>,
    pub dependencies: Vec<String>,
    pub dependency_requirements: BTreeMap<String, String>,
    pub publisher: String,
    pub publisher_public_key: String, // Ed25519 base64
    pub signature: String,          // Ed25519 base64 over canonical envelope
    pub files: Vec<RegistryFile>,
}
```

Validation checks:
- Ed25519 signature verification against publisher key
- SHA-256 checksum of canonical envelope matches declared checksum
- Schema version compatibility (supports v1 and v2)
- File count within `MAX_PACKAGE_FILES` (1,024)
- Total decoded size within `MAX_PACKAGE_BYTES` (64 MB)
- Path traversal detection (no `../` in file paths)

---

## 7. CLI Surface

```bash
roko config plugins list          # List installed plugins
roko config plugins install <id>  # Install from registry
roko config plugins remove <id>   # Remove a plugin
roko config plugins audit         # Report permissions and conflicts
roko config plugins publish       # Publish to registry
```

---

## 8. DynamicToolRegistry Integration

Plugin-declared tools are registered via `DynamicToolRegistry.register_plugin()`:

```rust
pub fn register_plugin(
    &mut self,
    plugin_name: &str,
    tools: Vec<ToolDef>,
    sandbox: SandboxConfig,
) {
    self.sandbox_configs.insert(plugin_name.to_string(), sandbox);
    for tool in tools {
        self.register(tool);
    }
}
```

Each plugin's tools inherit the plugin's `SandboxConfig`. The sandbox config is
retrievable via `sandbox_for(plugin_name)` or `sandbox_for_tool(tool_name)`.
Safety-critical built-in tools (`bash`, `read_file`, `write_file`) log a warning
when overridden by a plugin.

---

## 9. Source Locations

| Component | Path |
|---|---|
| Plugin manifest | `crates/roko-plugin/src/manifest.rs` |
| Signed registry | `crates/roko-plugin/src/registry.rs` |
| Tool registry | `crates/roko-plugin/src/tool_registry.rs` |
| Dependency resolver | `crates/roko-plugin/src/dependency.rs` |
| Trigger protocol | `crates/roko-plugin/src/trigger_protocol.rs` |
| Dynamic tool registry | `crates/roko-std/src/tool/registry.rs` |
| Sandbox config | `crates/roko-std/src/tool/sandbox_config.rs` |

---

*Derived from: v1/18-tools/14-plugin-sdk.md. Native ABI (tier 4) and full WASM
host surface (tier 5) remain target-state. Component-model Store/Bus hostcalls
plus OpenClaw/legacy one-shot parity are open product work.*
