# D -- Agent Lifecycle & Infrastructure (Docs 05, 06, 13)

Parity analysis of `docs/02-agents/05-agent-pools.md`, `06-mcp-integration.md`, `13-creation-sites.md` vs actual codebase.

---

## D.01 -- AgentPool sequential execution (Doc 05)

- **Status**: DONE
- **Priority**: P3
- **Estimated LOC**: 0 (complete)
- **Dependencies**: None
- **Files to modify**: None

### What the doc says

`AgentPool` at `crates/roko-agent/src/pool.rs` manages a queue of tasks for a single agent role. Tasks execute sequentially. If the primary agent fails, the pool retries with a fallback agent (different model).

### What exists

Fully implemented at `crates/roko-agent/src/pool.rs:148-360`. The struct has:
- `role: AgentRole` (line 150)
- `primary: Arc<dyn Agent>` (line 152)
- `fallback: Option<Arc<dyn Agent>>` (line 154)
- `pending: VecDeque<AgentTask>` (line 156)
- `statuses: Vec<(AgentInstanceId, InstanceStatus)>` (line 158)
- `completed: VecDeque<TaskOutcome>` (line 160)
- `active_task: Option<AgentInstanceId>` (line 162)

Methods implemented: `submit`, `submit_all`, `poll`, `drain_completed`, `cancel`, `execute_next` (with fallback retry logic at line 302), `execute_all`. 12 tests pass covering success, fallback, cancel, and ordering scenarios (lines 362-586).

Publicly exported from `crates/roko-agent/src/lib.rs:81`.

### Gaps

| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| D.01a | Pool is not used by orchestrator -- `orchestrate.rs` constructs agents ad-hoc via `AgentRunConfig` + `spawn_agent_with_layer`, never uses `AgentPool` | `crates/roko-cli/src/orchestrate.rs:1168` | Low (design intent) |

### Verify

```bash
grep -n 'AgentPool' crates/roko-agent/src/pool.rs | head -5
cargo test -p roko-agent pool -- --nocapture 2>&1 | tail -5
```

---

## D.02 -- AgentInstanceId (Doc 05)

- **Status**: DONE
- **Priority**: P3
- **Estimated LOC**: 0 (complete)
- **Dependencies**: None
- **Files to modify**: None

### What the doc says

Every agent instance gets a unique `AgentInstanceId` with `role: AgentRole` and `instance: String`. The `key()` method produces `"{role}-{instance}"` and `matches()` supports plan-based filtering.

### What exists

Fully implemented at `crates/roko-agent/src/pool.rs:21-69`:
- `role: AgentRole` (line 24)
- `instance: String` (line 26)
- `new(role, instance)` (line 32)
- `default_for(role)` (line 41)
- `key()` returning `"{role.label()}-{instance}"` (line 50-52)
- `matches(needle)` using `key().contains(needle)` (line 56-58)
- `Display` impl at line 61-69 (omits "default" suffix for default instances)

Tests at lines 379-403 cover display, key, and matches.

### Gaps

| ID | Gap | Where | Severity |
|----|-----|-------|----------|

No gaps. Matches doc exactly.

### Verify

```bash
grep -n 'AgentInstanceId' crates/roko-agent/src/pool.rs | head -5
```

---

## D.03 -- InstanceStatus lifecycle states (Doc 05)

- **Status**: PARTIAL
- **Priority**: P3
- **Estimated LOC**: 0
- **Dependencies**: None
- **Files to modify**: None

### What the doc says

Six states: `Warm`, `Pending`, `Running`, `Completed`, `Failed`, `Killed`. The lifecycle flows Warm -> Pending -> Running, then branches to Completed or Failed, with Failed optionally going to TryFallback or Killed.

### What exists

Implemented at `crates/roko-agent/src/pool.rs:74-101` with slightly different names:
- `Warm` -- matches doc
- `Pending` -- matches doc
- `Active` -- doc says `Running`
- `Done` -- doc says `Completed`
- `Failed` -- matches doc
- `Cancelled` -- doc says `Killed`

The semantic behavior is identical; only the variant names differ. The display strings are lowercase versions of the code names (`"active"`, `"done"`, `"cancelled"`).

### Gaps

| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| D.03a | Variant names differ from doc: `Active` vs `Running`, `Done` vs `Completed`, `Cancelled` vs `Killed` | `crates/roko-agent/src/pool.rs:82-88` | Trivial (doc drift) |

### Verify

```bash
grep -A8 'pub enum InstanceStatus' crates/roko-agent/src/pool.rs
```

---

## D.04 -- MultiAgentPool parallel execution (Doc 05)

- **Status**: DONE
- **Priority**: P3
- **Estimated LOC**: 0 (complete)
- **Dependencies**: None
- **Files to modify**: None

### What the doc says

`MultiAgentPool` at `crates/roko-agent/src/multi_pool.rs` manages multiple `AgentPool` instances across roles for concurrent execution. Contains `active: HashMap<AgentInstanceId, ActiveEntry>`, `warm: HashMap<(AgentRole, String), WarmEntry>`, `fallbacks: HashMap<AgentRole, Arc<dyn Agent>>`, `concurrency_limits: HashMap<AgentRole, usize>`, `default_concurrency: usize` (default 4).

### What exists

Fully implemented at `crates/roko-agent/src/multi_pool.rs:48-629`. The struct matches the doc specification exactly (lines 48-59):
- `active: HashMap<AgentInstanceId, ActiveEntry>` (line 50)
- `warm: HashMap<(AgentRole, String), WarmEntry>` (line 52)
- `fallbacks: HashMap<AgentRole, Arc<dyn Agent>>` (line 54)
- `concurrency_limits: HashMap<AgentRole, usize>` (line 56)
- `default_concurrency: usize` defaulting to 4 (line 58, constructor at line 71)

`WarmEntry` at line 19 and `ActiveEntry` at line 28 match doc specs. Note that the pool does NOT embed `AgentPool` instances internally -- it manages its own active/warm maps directly. The doc's claim that it "manages multiple `AgentPool` instances" is misleading; it's a standalone parallel implementation.

Additional methods beyond what doc describes:
- `ensure_active_instance` (line 215) -- auto-activates from warm or spawns fresh
- `run_task_with_auto_activation` (line 397) -- combines activation + execution
- `recycle_terminal_to_warm` (line 428) -- recycles done/failed agents back to warm pool
- `reap_terminal_active` (line 455) -- garbage collects terminal instances
- `evict_warm_all` (line 269) -- flush all warm for a role

Publicly exported from `crates/roko-agent/src/lib.rs:74`.

35 tests pass covering all functionality (lines 652-1007).

### Gaps

| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| D.04a | Pool is not used by orchestrator -- same as D.01a, `orchestrate.rs` never uses `MultiAgentPool` | `crates/roko-cli/src/orchestrate.rs` | Low (design intent) |
| D.04b | Doc says "manages multiple `AgentPool` instances" but actual impl manages its own flat maps, not delegating to `AgentPool` | `crates/roko-agent/src/multi_pool.rs:48-59` | Trivial (doc imprecision) |

### Verify

```bash
grep -n 'MultiAgentPool' crates/roko-agent/src/multi_pool.rs | head -5
cargo test -p roko-agent multi_pool -- --nocapture 2>&1 | tail -5
```

---

## D.05 -- Warm pool pre-spawning (Doc 05)

- **Status**: DONE
- **Priority**: P3
- **Estimated LOC**: 0 (complete)
- **Dependencies**: D.04
- **Files to modify**: None

### What the doc says

`MultiAgentPool` supports warm-pool pre-spawning. `WarmEntry` holds `agent: Arc<dyn Agent>` and `spawned_at: Instant`. `evict_stale_warm` removes entries idle longer than a configurable timeout (default 5 min).

### What exists

`WarmEntry` at `crates/roko-agent/src/multi_pool.rs:19-24`:
```rust
struct WarmEntry {
    agent: Arc<dyn Agent>,
    spawned_at: Instant,
}
```

Pre-spawn methods:
- `pre_spawn_warm(role, count, agent_fn)` at line 107-122
- `pre_spawn_warm_named(role, instance, agent)` at line 125-137

Promote methods:
- `promote_warm(role)` at line 145-159
- `promote_warm_named(role, instance)` at line 163-180
- `promote_warm_if_capacity(role)` at line 186-191
- `promote_warm_named_if_capacity(role, instance)` at line 195-204

Eviction:
- `evict_warm(role, max_idle)` at line 251-266 (named `evict_warm`, not `evict_stale_warm`)
- `evict_warm_all(role)` at line 269-281

Warm queries: `warm_count(role)` at line 285, `total_warm_count()` at line 291, `has_warm(role)` at line 614.

The default 5-minute timeout mentioned in the doc is not hardcoded -- the caller passes `max_idle: Duration` to `evict_warm`. No default constant exists.

### Gaps

| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| D.05a | No default timeout constant for warm eviction -- doc says "default: 5 minutes" but caller must pass explicitly | `crates/roko-agent/src/multi_pool.rs:251` | Trivial (doc aspirational) |
| D.05b | Method named `evict_warm` not `evict_stale_warm` as doc suggests | `crates/roko-agent/src/multi_pool.rs:251` | Trivial (doc drift) |

### Verify

```bash
grep -n 'evict_warm\|pre_spawn_warm\|promote_warm' crates/roko-agent/src/multi_pool.rs
```

---

## D.06 -- Pool concurrency control and bulk operations (Doc 05)

- **Status**: DONE
- **Priority**: P3
- **Estimated LOC**: 0 (complete)
- **Dependencies**: D.04
- **Files to modify**: None

### What the doc says

Per-role concurrency limits. `kill_all()`, `kill_by_plan(plan_id)`, `kill_by_role(role)`. These work through `ProcessSupervisor` in `bardo-runtime` for subprocess agents.

### What exists

Concurrency control:
- `set_concurrency_limit(role, limit)` at `crates/roko-agent/src/multi_pool.rs:82`
- `concurrency_limit(role)` at line 93
- `at_capacity(role)` at line 620
- `with_default_concurrency(limit)` at line 76

Kill operations:
- `kill_all(deadline)` at line 481-516, returns `KillReport` (line 634-650)
- `kill_plan_agents(plan_id)` at line 523-547 (doc says `kill_by_plan`)
- `kill_role(role)` at line 550-574 (doc says `kill_by_role`)

All kill operations work by removing entries from the `active` and `warm` HashMaps. They do NOT go through `ProcessSupervisor` -- they simply drop the `Arc<dyn Agent>`. The doc's claim that these "work through the `ProcessSupervisor`" is incorrect for the pool layer; the `ProcessSupervisor` integration is in `orchestrate.rs:2168` where the `PlanRunner` struct holds its own supervisor.

### Gaps

| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| D.06a | Kill methods named differently: `kill_plan_agents` vs doc's `kill_by_plan`, `kill_role` vs doc's `kill_by_role` | `crates/roko-agent/src/multi_pool.rs:523,550` | Trivial (doc drift) |
| D.06b | Pool kill operations do not go through `ProcessSupervisor` -- they just drop `Arc<dyn Agent>` | `crates/roko-agent/src/multi_pool.rs:481-574` | Medium (subprocess leaks if the dropped agent holds a child process) |

### Verify

```bash
grep -n 'kill_all\|kill_plan\|kill_role' crates/roko-agent/src/multi_pool.rs
```

---

## D.07 -- McpClient JSON-RPC transport (Doc 06)

- **Status**: DONE
- **Priority**: P3
- **Estimated LOC**: 0 (complete)
- **Dependencies**: None
- **Files to modify**: None

### What the doc says

`McpClient` struct manages JSON-RPC connection. `Transport` trait abstracts communication. `StdioTransport` spawns MCP server as child process. Wire types: `McpRequest`, `McpResponse`, `McpToolDef`, `McpToolResult`.

### What exists

Fully implemented at `crates/roko-agent/src/mcp/client.rs`.

Wire types:
- `McpRequest` at line 18-28 -- matches doc. Uses `serde_json::Value` for `params` (not `Option<Value>` as doc shows).
- `McpResponse` at line 31-43 -- matches doc. Error field is `JsonRpcError` (line 47-55), not `McpError` as doc claims.
- `McpToolDef` at line 58-68 -- `description` and `input_schema` are `Option<...>` (doc shows non-optional).
- `McpToolResult` at line 71-79 -- matches doc.
- `McpContent` at line 82-90 -- extra type not in doc.

Transport trait at line 98-102:
```rust
pub trait Transport: Send + Sync {
    async fn roundtrip(&self, request: &McpRequest) -> Result<McpResponse, McpError>;
}
```
Doc shows separate `send`/`receive` methods. Actual impl uses single `roundtrip` method (request-response in one call).

`StdioTransport` at line 129-205 -- spawns child process with piped stdin/stdout. Matches doc concept.

`McpClient<T: Transport>` at line 213-301 -- generic over `Transport`. Methods:
- `initialize()` at line 258 -- sends MCP initialize handshake
- `list_tools()` at line 274 -- returns `Vec<McpToolDef>`
- `call_tool(name, arguments)` at line 285 -- returns `McpToolResult`

12 tests at lines 306-492 cover all methods including error propagation and ID incrementing.

### Gaps

| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| D.07a | Transport trait uses `roundtrip` not separate `send`/`receive` | `crates/roko-agent/src/mcp/client.rs:101` | Trivial (doc drift) |
| D.07b | `McpResponse.error` is `JsonRpcError` not `McpError` as doc says | `crates/roko-agent/src/mcp/client.rs:40` | Trivial (doc drift) |
| D.07c | `McpToolDef.description` and `input_schema` are `Option` in code, non-optional in doc | `crates/roko-agent/src/mcp/client.rs:64-67` | Trivial (doc drift) |

### Verify

```bash
cargo test -p roko-agent mcp -- --nocapture 2>&1 | tail -10
```

---

## D.08 -- MCP config discovery (Doc 06)

- **Status**: DONE
- **Priority**: P3
- **Estimated LOC**: 0 (complete)
- **Dependencies**: None
- **Files to modify**: None

### What the doc says

`McpConfig` and `McpServerConfig` structs. `find_mcp_config` searches: (1) cwd, (2) project root, (3) home directory. Parses `.mcp.json` with servers array.

### What exists

Implemented at `crates/roko-agent/src/mcp/config.rs`.

`McpServerConfig` at line 11-23 matches doc: `name`, `command`, `args`, `env`.

`McpConfig` at line 27-31: `servers: Vec<McpServerConfig>` -- matches doc.

`find_mcp_config` at line 54-66 walks UP from `start_dir` to filesystem root, checking for `.mcp.json` in each directory. This is a walk-up strategy, NOT the three-location search described in the doc (cwd, project root, home). It is simpler and more general.

`McpConfig::load(path)` at line 39-41 for explicit path loading.

6 tests at lines 103-211 cover parsing, walk-up discovery, not-found, and invalid JSON.

The `roko.toml` `agent.mcp_config` field exists in the CLI config layer at `crates/roko-cli/src/config.rs:194` (not in `roko-core`'s `AgentConfig` schema). The orchestrator reads it at `crates/roko-cli/src/orchestrate.rs:2907` with fallback to `find_mcp_config`.

### Gaps

| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| D.08a | Discovery uses walk-up from start_dir, not the 3-location search (cwd, project root, home) described in doc | `crates/roko-agent/src/mcp/config.rs:54-66` | Trivial (doc describes aspirational behavior, walk-up is functionally equivalent or better) |
| D.08b | `agent.mcp_config` is in CLI config layer (`crates/roko-cli/src/config.rs:194`), not in `roko-core`'s `AgentConfig` schema. The doc's `roko.toml` `[agent] mcp_config = ".mcp.json"` reference applies to the CLI config, not the core schema | `crates/roko-core/src/config/schema.rs:1256-1293` vs `crates/roko-cli/src/config.rs:194` | Low (split config layers) |

### Verify

```bash
grep -n 'find_mcp_config\|McpConfig' crates/roko-agent/src/mcp/config.rs | head -10
```

---

## D.09 -- Tool conversion: mcp_to_tool_def (Doc 06)

- **Status**: DONE
- **Priority**: P3
- **Estimated LOC**: 0 (complete)
- **Dependencies**: D.07
- **Files to modify**: None

### What the doc says

`mcp_to_tool_def` converts `McpToolDef` into `ToolDef`. MCP tools get `ToolCategory::Custom` and `ToolPermission::read_only()`.

### What exists

Implemented at `crates/roko-agent/src/mcp/to_tool_def.rs:22-51`.

The function signature differs from doc:
```rust
// Actual (takes server_prefix for namespacing):
pub fn mcp_to_tool_def(mcp_tool: &McpToolDef, server_prefix: &str) -> ToolDef

// Doc shows:
pub fn mcp_to_tool_def(mcp_tool: &McpToolDef) -> ToolDef
```

The function prefixes tool names with `{server_prefix}__` (line 23). Category is `ToolCategory::Mcp` (not `ToolCategory::Custom` as doc says). Permission is `ToolPermission::read_only()` (matches doc). Sets `ToolSource::Mcp { server }` for provenance tracking. Default timeout is 60s. Concurrency is `Parallel`.

8 tests at lines 53-142 verify name prefixing, description, schema mapping, fallbacks, category, and defaults.

### Gaps

| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| D.09a | Category is `ToolCategory::Mcp`, not `ToolCategory::Custom` as doc says | `crates/roko-agent/src/mcp/to_tool_def.rs:41` | Trivial (doc says Custom, code uses Mcp -- code is better) |
| D.09b | Function takes `server_prefix` parameter for namespacing; doc shows no prefix param | `crates/roko-agent/src/mcp/to_tool_def.rs:22` | Trivial (code is richer than doc) |

### Verify

```bash
grep -n 'mcp_to_tool_def' crates/roko-agent/src/mcp/to_tool_def.rs
```

---

## D.10 -- Multi-server deduplication (Doc 06)

- **Status**: DONE
- **Priority**: P3
- **Estimated LOC**: 0 (complete)
- **Dependencies**: D.09
- **Files to modify**: None

### What the doc says

`dedup_tools` resolves collisions: (1) unique names kept as-is, (2) collisions get server-name prefix, (3) built-in tools take precedence. Takes `Vec<(String, McpToolDef)>`.

### What exists

Implemented at `crates/roko-agent/src/mcp/dedup.rs:21-43`.

Signature: `pub fn dedup_tools(tools: Vec<(String, Vec<ToolDef>)>) -> Vec<ToolDef>`

The actual strategy differs from the doc:
- Takes `Vec<(server_name, Vec<ToolDef>)>` (already converted to `ToolDef`, already prefixed)
- Uses last-writer-wins for name collisions while preserving insertion order
- Tools are expected to already be prefixed with `server_name__` by `mcp_to_tool_def`
- No runtime prefixing logic -- that already happened during conversion

The doc's description of runtime prefixing on collision is aspirational; the actual implementation relies on upstream prefixing. Built-in precedence is handled by `DynamicToolRegistry`, not `dedup_tools`.

6 tests at lines 45-133 verify empty input, single server, no overlap, last-writer-wins, insertion order, and mixed overlap.

### Gaps

| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| D.10a | Doc describes runtime collision-triggered prefixing, but actual impl does last-writer-wins on already-prefixed tools | `crates/roko-agent/src/mcp/dedup.rs:21-43` | Low (doc describes a different algorithm than what was built; both work) |
| D.10b | Built-in precedence is NOT in `dedup_tools` -- it's in `DynamicToolRegistry.rebuild()` | `crates/roko-agent/src/mcp/dynamic_registry.rs:89-139` | Trivial (doc attributes it to wrong module) |

### Verify

```bash
cargo test -p roko-agent mcp_dedup -- --nocapture 2>&1 | tail -5
```

---

## D.11 -- DynamicToolRegistry (Doc 06)

- **Status**: DONE
- **Priority**: P3
- **Estimated LOC**: 0 (complete)
- **Dependencies**: D.09, D.10
- **Files to modify**: None

### What the doc says

`DynamicToolRegistry` composes static built-in tools with MCP-discovered tools. Implements `ToolRegistry` trait. Has `static_tools: Vec<ToolDef>` and `mcp_tools: Vec<ToolDef>`.

### What exists

Implemented at `crates/roko-agent/src/mcp/dynamic_registry.rs:18-150`.

The struct is richer than the doc describes:
```rust
pub struct DynamicToolRegistry {
    base: Vec<ToolDef>,
    mcp_servers: HashMap<String, Vec<ToolDef>>,  // keyed by server name, not flat
    prefer_mcp: bool,                             // collision preference flag
    all_tools: Vec<ToolDef>,                      // pre-flattened for fast lookup
}
```

Key differences from doc:
- `base` instead of `static_tools`
- `mcp_servers` is a `HashMap<String, Vec<ToolDef>>` keyed by server name, not a flat `mcp_tools: Vec<ToolDef>`
- Has `prefer_mcp` flag for controlling collision behavior
- Pre-builds `all_tools` on each mutation for O(1) lookup

Implements `ToolRegistry` at line 142-150 with `get` and `all`.

Additional methods beyond doc:
- `add_mcp_tools(server, tools)` at line 66 -- per-server tool registration
- `remove_server(name)` at line 74 -- dynamic server removal
- `server_count()` at line 84 -- query registered servers
- `empty()` at line 53 -- construct without base tools
- `with_preference(base, prefer_mcp)` at line 40 -- explicit collision preference

The `rebuild()` method at line 89-139 handles built-in vs MCP collision:
- Default: built-ins win (line 119-136)
- With `prefer_mcp=true`: MCP tools win (line 100-118)
- Warns on collision via `tracing::warn` in both cases

14 tests at lines 152-296 verify all functionality including collision behavior.

### Gaps

| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| D.11a | Internal structure is richer than doc (per-server HashMap, prefer_mcp flag, pre-flattened cache) | `crates/roko-agent/src/mcp/dynamic_registry.rs:18-27` | Trivial (code is better, doc is simplified) |

### Verify

```bash
cargo test -p roko-agent mcp_dynamic_registry -- --nocapture 2>&1 | tail -5
```

---

## D.12 -- MCP handler and bridge (Doc 06)

- **Status**: DONE
- **Priority**: P3
- **Estimated LOC**: 0 (complete)
- **Dependencies**: D.07, D.11
- **Files to modify**: None

### What the doc says

`McpHandler` wraps `McpClient` for tool execution inside the `ToolDispatcher`. The handler implements `ToolHandler` and routes calls through the MCP client. The ToolLoop in `orchestrate.rs` follows a 7-step pipeline: discover -> connect -> list -> dedup -> convert -> registry -> loop.

### What exists

Two files implement this:

**Handler** at `crates/roko-agent/src/mcp/handler.rs`:
- `McpHandlerResolver<T: Transport>` at line 22-25 -- implements `HandlerResolver` (line 42-57). Falls back from static handlers to MCP clients using `{server}__{tool}` naming convention.
- `McpToolHandler<T: Transport>` at line 61-65 -- implements `ToolHandler` (line 84-102). Calls `client.call_tool(remote_name, arguments)` and renders results.
- `split_prefixed_tool_name` at line 104 -- splits `server__tool` back into parts.
- `render_mcp_result` at line 112 -- converts `McpToolResult` to `ToolResult`.

4 tests at lines 140-273 cover routing through the dispatcher, static handler preference, and unknown server handling.

**Bridge** at `crates/roko-agent/src/mcp/bridge.rs`:
- `discover_mcp_tools(config: &McpConfig)` at line 33-85 -- async function that spawns MCP servers, initializes them with timeouts, lists tools, converts via `mcp_to_tool_def`, and deduplicates. This is the 7-step pipeline the doc describes.
- `McpBridgeError` at line 18-29 -- typed errors for spawn, initialize timeout, initialize failure, list timeout, list failure.
- Uses `MCP_DISCOVERY_TIMEOUT` of 5 seconds (line 14).

The doc mentions `McpHandler` as a struct but the actual implementation is `McpToolHandler`. The doc's `HandlerResolver` concept is implemented as `McpHandlerResolver`.

### Gaps

| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| D.12a | Handler struct named `McpToolHandler`, not `McpHandler` as doc says | `crates/roko-agent/src/mcp/handler.rs:61` | Trivial (naming) |

### Verify

```bash
grep -n 'McpToolHandler\|McpHandlerResolver\|discover_mcp_tools' crates/roko-agent/src/mcp/handler.rs crates/roko-agent/src/mcp/bridge.rs
```

---

## D.13 -- Claude CLI MCP passthrough (Doc 06)

- **Status**: DONE
- **Priority**: P3
- **Estimated LOC**: 0 (complete)
- **Dependencies**: D.08
- **Files to modify**: None

### What the doc says

For `ClaudeCliAgent`, MCP config is passed through as `--mcp-config <path>` flag. At `orchestrate.rs` line 469. Auto-discovery fallback searches for `.mcp.json`. Configured via `roko.toml` `[agent] mcp_config = ".mcp.json"`.

### What exists

The MCP passthrough works through the `SpawnAgentSpec.mcp_config: Option<PathBuf>` field (at `crates/roko-cli/src/agent_spawn.rs:25`), which is converted to `AgentOptions.mcp_config` (line 50) and forwarded to the provider adapter.

In `orchestrate.rs`:
- `AgentRunConfig.mcp_config: Option<PathBuf>` at line 1147
- Passed through `SpawnAgentSpec` at lines 1199, 1247, 1321

MCP config resolution happens at `crates/roko-cli/src/orchestrate.rs:2907-2928`:
1. If `config.agent.mcp_config` is set explicitly, load from that path (line 2907-2914)
2. Otherwise, `find_mcp_config(workdir)` walk-up discovery (line 2916-2923)

The `roko.toml` `[agent] mcp_config` field is in the CLI config layer at `crates/roko-cli/src/config.rs:194`. Can be set via `roko config set agent.mcp_config <path>` (line 1491-1493).

The doc says line 469 -- actual line is 1147 (AgentRunConfig) and 2907 (resolution). Line numbers drifted substantially.

### Gaps

| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| D.13a | Doc cites line 469 for passthrough; actual lines are 1147 (struct), 1199/1247/1321 (usage), 2907 (resolution) | `crates/roko-cli/src/orchestrate.rs` | Trivial (line numbers drifted) |

### Verify

```bash
grep -n 'mcp_config' crates/roko-cli/src/orchestrate.rs | head -10
```

---

## D.14 -- Eight creation sites consolidation (Doc 13)

- **Status**: PARTIAL
- **Priority**: P1
- **Estimated LOC**: ~50 (remaining migration work)
- **Dependencies**: None
- **Files to modify**: `crates/roko-cli/src/main.rs`, `crates/roko-cli/src/research.rs`

### What the doc says

Eight places construct agents. The target state is consolidation into `create_agent_for_model`. Current status per the doc:

| Site | Doc Status |
|---|---|
| orchestrate.rs run_prepared_agent | Migrated (routed + no-routing) |
| orchestrate.rs model selection | Partially migrated |
| run.rs | Migrated (routed + no-routing) |
| prd.rs | Not migrated |
| research.rs | Partially migrated |
| agent_exec.rs | Migrated |
| Tests | N/A |
| provider/mod.rs factory | Implemented |

### What exists

The factory function `create_agent_for_model` exists at `crates/roko-agent/src/provider/mod.rs:102-196`. It resolves model -> provider -> adapter and constructs agents. All 6 provider adapters are registered (OpenAiCompat, ClaudeCli, AnthropicApi, CursorAcp, Perplexity, Gemini) at lines 87-96.

The shared helper layer `crates/roko-cli/src/agent_spawn.rs` provides:
- `spawn_agent_scoped` (line 64) -- wraps `create_agent_for_model` with safety scope
- `spawn_agent_with_layer` (line 78) -- wraps with explicit safety layer

**Actual creation site status** (verified against code):

1. **`orchestrate.rs::run_prepared_agent`** (line 1168): MIGRATED. Uses `spawn_agent_with_layer` via `SpawnAgentSpec` at lines 1189, 1237, 1274, 1311. Has 4 branches: routing config, claude CLI, known-protocol, and generic subprocess. All route through `create_agent_for_model`.

2. **`orchestrate.rs` model selection**: MIGRATED. `AgentRunConfig` construction at line 6399 feeds into `run_prepared_agent` which routes through the factory.

3. **`run.rs`** (dispatch_agent at line 306): MIGRATED. Four `spawn_agent_scoped` calls at lines 327, 373, 406, 437 for routing/claude/known-protocol/generic branches. All go through factory.

4. **`prd.rs`**: MIGRATED (doc says "Not migrated" -- STALE). Uses `run_agent_logged` from `agent_exec.rs` at lines 224 and 751, which calls `spawn_agent_scoped` internally. No direct agent construction.

5. **`research.rs`**: PARTIALLY MIGRATED. The `research.rs` module itself does NOT construct agents. However, `main.rs` `cmd_research` function at lines 3593-3964 uses direct `create_agent_for_model` calls for specialty research paths:
   - Perplexity deep research at `main.rs:3638`
   - Gemini grounding at `main.rs:3826`
   - Perplexity search-grounded at `main.rs:3928`
   These bypass `spawn_agent_scoped` and call `create_agent_for_model` directly -- missing safety scoping.

6. **`agent_exec.rs`**: MIGRATED. `run_agent_capture_impl` at line 82 uses `spawn_agent_scoped` at line 125.

7. **Tests**: N/A (correct).

8. **`provider/mod.rs`**: IMPLEMENTED. The factory is complete and tested (10 tests at lines 709-998).

### Gaps

| ID | Gap | Where | Severity |
|----|-----|-------|----------|
| D.14a | Doc says prd.rs is "Not migrated" -- this is stale; prd.rs now routes through `agent_exec.rs` -> `spawn_agent_scoped` -> `create_agent_for_model` | doc `13-creation-sites.md:217` | Low (doc stale) |
| D.14b | `main.rs` research paths (lines 3638, 3826, 3928) call `create_agent_for_model` directly, bypassing `spawn_agent_scoped` and its safety layer scoping | `crates/roko-cli/src/main.rs:3638,3826,3928` | Medium (safety layer may not be active for research agents) |
| D.14c | `PerplexitySearchClient::new` at `main.rs:4307` constructs a non-Agent HTTP client directly -- outside the factory entirely | `crates/roko-cli/src/main.rs:4307` | Low (specialty HTTP client, not an Agent) |
| D.14d | `PerplexityEmbedAgent` at `crates/roko-cli/src/research.rs:655,683` is constructed directly for embedding -- specialty backend, not a general agent | `crates/roko-cli/src/research.rs:655` | Low (specialty embedding, intentional) |

### Verify

```bash
grep -rn 'create_agent_for_model\|spawn_agent_scoped\|spawn_agent_with_layer' crates/roko-cli/src/ --include='*.rs' | grep -v test | grep -v '//' | wc -l
grep -rn 'ClaudeCliAgent::new\|ExecAgent::new' crates/roko-cli/src/ --include='*.rs' | grep -v test | grep -v '//'
```

---

## Summary

| ID | Item | Status | Priority |
|----|------|--------|----------|
| D.01 | AgentPool sequential execution | DONE | P3 |
| D.02 | AgentInstanceId | DONE | P3 |
| D.03 | InstanceStatus lifecycle states | DONE (naming drift) | P3 |
| D.04 | MultiAgentPool parallel execution | DONE | P3 |
| D.05 | Warm pool pre-spawning | DONE | P3 |
| D.06 | Pool concurrency control + bulk ops | DONE | P3 |
| D.07 | McpClient JSON-RPC transport | DONE | P3 |
| D.08 | MCP config discovery | DONE | P3 |
| D.09 | Tool conversion: mcp_to_tool_def | DONE | P3 |
| D.10 | Multi-server deduplication | DONE | P3 |
| D.11 | DynamicToolRegistry | DONE | P3 |
| D.12 | MCP handler and bridge | DONE | P3 |
| D.13 | Claude CLI MCP passthrough | DONE | P3 |
| D.14 | Eight creation sites consolidation | PARTIAL | P1 |

**Overall**: 13 of 14 items are DONE. The only PARTIAL item is D.14 (creation site consolidation) where 3 research paths in `main.rs` call `create_agent_for_model` directly without safety scoping. Additionally, the pool layer (D.01, D.04) is fully built and tested but not wired into the orchestrator -- the orchestrator creates agents ad-hoc through `AgentRunConfig` + `spawn_agent_with_layer` instead.

**Key doc staleness**: The doc for creation sites (13-creation-sites.md) claims prd.rs is "Not migrated" -- this is outdated. prd.rs now routes through `agent_exec.rs` -> `spawn_agent_scoped` and is fully migrated.

**Actionable items**:
1. (P1) Wrap the 3 research `create_agent_for_model` calls in `main.rs:3638,3826,3928` with `spawn_agent_scoped` or `with_scoped_safety_layer` for consistent safety scoping.
2. (P2) Update doc 13 to mark prd.rs as migrated and note the `agent_spawn.rs` helper layer.
3. (P3) Wire `MultiAgentPool` into the orchestrator as the doc's "Future" flow describes, replacing ad-hoc agent construction.

---

## Agent Execution Notes

### D.14 — Creation-Site Consolidation

This is a good unattended batch because it is narrow and easy to verify.

Recommended slice:

1. migrate the remaining direct research-path creation calls,
2. re-run a search for direct `create_agent_for_model` usage in CLI entrypoints,
3. document any intentional non-Agent specialty clients separately.

Acceptance criteria:

- remaining research agent entrypoints use scoped helpers,
- search results clearly show what direct calls remain and why,
- the patch does not widen into pool activation or orchestrator redesign.

### D.01 / D.04 — Pools

Do not try to wire pools into the orchestrator in batch `02` unless the user asks for it explicitly.

The pool implementation is real, but runtime ownership belongs with batch `01`.
