# Plugin Loading and Admission

> Depth file for [19-TOOLS-PLUGINS](../../19-TOOLS-PLUGINS.md) -- discovery-first
> loading, manifest validation, tier-specific sandboxing, and strict admission
> controls for the five-tier SPI.

---

## 1. Overview

Plugin loading follows a discovery-first model: manifests are the source of
truth, and `roko.toml` provides runtime overrides rather than serving as the
plugin catalog. The loader validates each manifest, selects the tier-appropriate
sandbox, and registers exposed capabilities with the relevant subsystem.

**Current status:** Signed dependency graphs, bounded typed WASM hooks, strict
plugin admission, verified relay/install, and current CLI/MCP targets satisfy
the E32 8/8 manifest. WIT/Component hostcalls and OpenClaw/legacy adapter parity
remain separate roadmap work.

---

## 2. Loading Lifecycle

```
discover -> validate -> sandbox -> instantiate -> register -> monitor -> unload
```

1. **Discover** manifests from plugin roots or installed metadata
2. **Validate** declared tier, capabilities, dependencies, and permissions
3. **Select sandbox** for the tier (see Section 4)
4. **Instantiate** the extension (spawn process, load WASM, or merge data)
5. **Register** exposed capabilities with the relevant subsystem
6. **Monitor** health and resource usage
7. **Unload** on shutdown, policy failure, or explicit disable

---

## 3. Admission Checks

Every plugin is validated before activation:

- Manifest parses as valid TOML with correct schema
- Tier matches the entrypoint type (data tiers cannot declare executables)
- Declared capabilities are internally consistent
- Dependencies resolve without cycles or version conflicts
- WASM modules pass hook signature validation (see `wasm-hooks-23.md`)
- Package files contain no path traversal (`../`)
- Ed25519 signature verifies against the publisher key
- SHA-256 checksum matches declared package checksum
- Total package size within 64 MB, file count within 1,024

### 3.1 Fail-Fast Startup

Plugin validation errors are surfaced at startup. A plugin that fails admission
is rejected before registration -- no partial loading. The CLI `roko config
plugins audit` command reports all validation issues without modifying state.

---

## 4. Sandbox Model

The sandbox is selected from the manifest tier, not from the call site:

| Tier | Sandbox | Constraints |
|---|---|---|
| 1 Untrusted | Most restricted | No FS, no network, 64 MB / 5s |
| 2 Sandboxed | Read-only worktree | Deny `.env`, secrets, `.git/config`; no network, 128 MB / 10s |
| 3 Standard | Worktree r/w | Deny `.env`, secrets; network allowed, 256 MB / 30s |
| 4 Trusted | Full access | Full FS, full network, 512 MB / 120s |
| 5 Kernel | Unrestricted | No caps |

### 4.1 SandboxConfig

```rust
// crates/roko-std/src/tool/sandbox_config.rs
pub struct SandboxConfig {
    pub allowed_paths: Vec<String>,   // glob patterns
    pub denied_paths: Vec<String>,    // deny takes priority
    pub network_access: bool,
    pub max_memory_mb: u64,           // 0 = unlimited
    pub max_cpu_seconds: u64,         // 0 = unlimited
}
```

Named constructors:
- `SandboxConfig::most_restricted()` -- tier 1
- `SandboxConfig::for_tier_level(n)` -- standard tier defaults
- `SandboxConfig::unrestricted()` -- tier 5

### 4.2 Validation

`SandboxConfig::validate()` checks for:
- Exact overlap between `allowed_paths` and `denied_paths`
- Path traversal (`../`) in allowed or denied paths
- Deny-all wildcard with non-empty allowed paths

`SandboxConfig::validate_command()` rejects shell metacharacters:
`|`, `;`, `&&`, `||`, `` ` ``, `$(`, `>`, `<`, `>>`, `<<`, `&`.

---

## 5. Tier-Specific Loading

### Tier 1 and 2 (Pure Data)

The loader reads the manifest, loads the data bundle (Markdown prompts or
profile TOML), and merges it into the prompt or profile surface. No code is
executed.

For tier 2, multiple profile bundles compose:
- Tools merge by union
- Roles merge by union with collision warnings
- Gates stack unless scoped to a profile name

### Tier 3 (Declarative)

The loader resolves the entrypoint, spawns the subprocess or MCP server, and
converts declared tool schemas into `ToolDef` entries via
`DynamicToolRegistry.register_plugin()`.

### Tier 4 (Native)

The loader resolves the ABI bridge and checks the ABI version. If the ABI
version mismatches, the plugin is rejected before registration.

### Tier 5 (WASM)

The loader instantiates the WASM module via `wasmi` with declared capability
grants and resource caps. Module size is limited to 32 MB. Fuel metering caps
execution at 10 million fuel units. Memory is bounded to 256 MB.

---

## 6. DynamicToolRegistry

Plugin tools are registered through `DynamicToolRegistry`:

```rust
impl DynamicToolRegistry {
    pub fn register_plugin(
        &mut self,
        plugin_name: &str,
        tools: Vec<ToolDef>,
        sandbox: SandboxConfig,
    );
    pub fn sandbox_for(&self, plugin_name: &str) -> Option<&SandboxConfig>;
    pub fn sandbox_for_tool(&self, tool_name: &str) -> Option<&SandboxConfig>;
}
```

Deduplication: when a plugin tool shares a name with an existing entry, the new
entry replaces the old one with a `tracing::warn`. Safety-critical built-ins
(`bash`, `read_file`, `write_file`) produce an elevated warning.

---

## 7. Catalog Validation

`validate_tool_catalog()` checks for four issue types:

| Issue | Description |
|---|---|
| `UnhandledTool` | Tool has no handler and is not MCP-dispatched |
| `DuplicateName` | Two tools share the same canonical name |
| `MissingHandler` | Handler exists but no corresponding `ToolDef` |
| `DeprecatedTool` | Tool name matches a deprecated prefix (`legacy.`, `deprecated.`, `old_`) |

MCP tools are exempt from `UnhandledTool` checks since they are dispatched
externally.

---

## 8. Source Locations

| Component | Path |
|---|---|
| Plugin manifest parser | `crates/roko-plugin/src/manifest.rs` |
| Signed registry contract | `crates/roko-plugin/src/registry.rs` |
| SandboxConfig | `crates/roko-std/src/tool/sandbox_config.rs` |
| DynamicToolRegistry | `crates/roko-std/src/tool/registry.rs` |
| Extension loader | `crates/roko-cli/src/runner/extension_loader.rs` |
| Extension registry | `crates/roko-cli/src/runner/extension_registry.rs` |

---

*Derived from: v1/18-tools/16-plugin-loading.md. Discovery roots and CLI surface
updated to match E32 8/8 implementation. Path traversal and shell metacharacter
validation added per actual SandboxConfig implementation.*
