# Tool Architecture

> Depth file for [19-TOOLS-PLUGINS](../../19-TOOLS-PLUGINS.md) -- ToolDef pattern,
> ToolContext, ToolResult, three permission tiers, tool composition, and the
> cognitive loop integration.

---

## 1. The ToolDef Pattern

Every tool in Roko is a module exporting a `ToolDef` struct. `ToolDef` is the
canonical declaration format -- a compile-time constant describing the tool's
name, description, category, permission flags, parameter schema, and source
provenance.

### 1.1 Core Struct

```rust
// crates/roko-core/src/tool.rs
pub struct ToolDef {
    pub name: String,           // snake_case or namespace.action
    pub description: String,    // LLM-facing: when to call, what it returns
    pub category: ToolCategory, // drives profile filtering
    pub permission: ToolPermission, // read/write/exec/network flags
    pub parameters: ToolSchema, // JSON Schema for input
    pub source: ToolSource,     // Builtin, Mcp, or Plugin
}
```

### 1.2 ToolPermission Flags

```rust
pub struct ToolPermission {
    pub read: bool,
    pub write: bool,
    pub execute: bool,
    pub network: bool,
}
```

Named constructors provide ergonomic defaults:

| Constructor | read | write | execute | network | Typical tools |
|---|---|---|---|---|---|
| `read_only()` | true | false | false | false | `read_file`, `grep`, `glob`, `ls` |
| `writes()` | true | true | false | false | `write_file`, `edit_file`, `notebook_edit` |
| `executes()` | true | true | true | false | `bash`, `run_tests` |
| `network()` | true | false | false | true | `web_fetch`, `web_search` |

### 1.3 ToolCategory Taxonomy

| Category | Description | Examples |
|---|---|---|
| `Read` | Read-only data access | `read_file`, `glob`, `grep`, `ls` |
| `Write` | File creation/modification | `write_file`, `edit_file`, `multi_edit` |
| `Exec` | Process execution | `bash`, `run_tests` |
| `Web` | Network access | `web_fetch`, `web_search` |
| `Planning` | Task/plan management | `todo_write`, `exit_plan_mode` |
| `Agent` | Sub-agent delegation | `task` |
| `Mcp` | MCP-proxied tools | All `github.*` tools |

### 1.4 ToolSource Provenance

```rust
pub enum ToolSource {
    Builtin,                    // compiled into roko-std
    Mcp { server: String },     // discovered from MCP server
    Plugin { name: String },    // declared by plugin manifest
}
```

MCP-sourced tools are dispatched through an external MCP client and are exempt
from local handler checks in `validate_tool_catalog()`.

---

## 2. ToolResult Format

All tools return `ToolResult`, which includes expected/actual fields for ground
truth verification on write tools:

```rust
pub struct ToolResult {
    pub data: serde_json::Value,
    pub is_error: bool,
    pub schema_version: u32,
    pub expected_outcome: Option<String>,
    pub actual_outcome: Option<String>,
    pub ground_truth_source: Option<String>,
}

impl ToolResult {
    pub fn read<T: Serialize>(data: T) -> Self { /* ... */ }
    pub fn write<T: Serialize>(
        data: T, expected: impl Into<String>,
        actual: impl Into<String>, source: impl Into<String>,
    ) -> Self { /* ... */ }
}
```

The `expected_outcome` and `actual_outcome` fields feed the Gate pipeline
(Chapter 07). When expected and actual diverge, the episode is tagged for
Dream replay and heuristic revision.

---

## 3. ToolContext

The runtime injects a `ToolContext` providing access to services needed during
tool execution. The context is domain-parameterized -- a coding agent receives
different context fields than a chain agent.

Core fields available to all agents:

- `event_bus: Arc<EventBus>` -- typed event emission for TUI and telemetry
- `neuro: Option<Arc<NeuroStore>>` -- durable knowledge store
- `config: Arc<ToolConfig>` -- current session config

Domain-specific fields (chain plugin) add providers, signers, Revm forks, and
subgraph clients. These are injected by the domain plugin, not the kernel.

---

## 4. Tools in the Cognitive Loop

Tools operate at the ACT step (step 5) of the nine-step cognitive loop:

```
1. SENSE        -> Substrate.query()       What is happening?
2. ASSESS       -> Scorer.score()          How relevant/important?
3. ATTEND       -> Router.select()         What matters most?
4. COMPOSE      -> Composer.compose()      Build context under budget
5. ACT          -> Agent.execute()         <- TOOLS INVOKED HERE
6. VERIFY       -> Gate.verify()           Did it work?
7. PERSIST      -> Substrate.put()         Store output with lineage
8. ADAPT        -> Policy.decide()         What patterns emerged?
9. META-COGNIZE -> Daimon.assess()         Am I doing this well?
```

The VERIFY step compares `ToolResult.expected_outcome` against
`ToolResult.actual_outcome` to produce a `Verdict`. This closes the
perception-action loop: act via tools, verify via gates, adapt via policies.

---

## 5. Tool Composition Patterns

Tools compose into three patterns:

**Sequential (pipe):** output of A feeds input of B.
```
read_file -> grep -> edit_file
```

**Parallel (fan-out):** multiple tools execute concurrently, results merged.
```
web_search("rust async") -+-> merge -> compose_answer
web_search("tokio guide") -+
```

**Iterative (loop):** repeated calls until a condition is met.
```
loop { edit_file -> run_tests -> if pass then break }
```

The predominant composition in Roko agents is ReAct (Yao et al. 2023):
interleaved reasoning traces and tool actions. The LLM decides at each step
which tool to call based on prior observations. No explicit pipeline
definition is needed -- the LLM is the pipeline controller.

---

## 6. Tool Audit Methodology

Tool surface reduction follows the SkillReducer methodology (Chen et al. 2026,
arXiv:2603.29919). Systematic tool pruning -- removing redundant or low-impact
tools -- improves both task completion rates and token efficiency. Roko applies
this through:

- **Profile-based filtering** -- each profile loads only relevant categories
- **Role-based access** -- `StaticToolRegistry.for_role()` restricts per role
- **Catalog validation** -- `validate_tool_catalog()` detects unhandled tools,
  duplicates, missing handlers, and deprecated entries

---

## 7. LLM-Optimized Descriptions

Tool descriptions serve two audiences: the LLM selecting which tool to call,
and the LLM filling in parameters.

**Selection guidance** (the `description` field):
- Starts with the tool's purpose in a single phrase
- Lists specific intents that map to this tool
- States what it does NOT do (disambiguates from similar tools)

**Anti-patterns** that degrade tool selection accuracy:
- Generic descriptions ("interact with files")
- Missing parameter docs
- Ambiguous scope between similar tools
- Response payloads exceeding ~25,000 tokens

---

## 8. OpenAPI 3.1 Compatibility

Roko tool schemas are JSON Schema Draft 2020-12 compliant, aligning with
OpenAPI 3.1. The `schemars` crate derives JSON Schema from Rust parameter
structs at compile time.

| Roko Field | Claude API | OpenAI API | MCP |
|---|---|---|---|
| `name` | `name` | `name` | `name` |
| `description` | `description` | `description` | `description` |
| `input_schema` | `input_schema` | `parameters` | `inputSchema` |

The conversion is mechanical -- no information is lost in either direction.

---

## 9. Source Locations

| Component | Path |
|---|---|
| ToolDef struct | `crates/roko-core/src/tool.rs` |
| Builtin definitions | `crates/roko-std/src/tool/builtin/` |
| Static registry | `crates/roko-std/src/tool/registry.rs` |
| Handler dispatch | `crates/roko-std/src/tool/handlers.rs` |
| Sandbox config | `crates/roko-std/src/tool/sandbox_config.rs` |
| MCP client | `crates/roko-agent/src/mcp/` |
| Safety layer | `crates/roko-agent/src/safety/` |

---

*Derived from: v1/18-tools/00-tool-architecture.md. Chain-specific content
(Capability<T> tokens, Revm simulation, Alloy providers) moved to the chain
domain plugin documentation. Death/mortality framing removed.*
