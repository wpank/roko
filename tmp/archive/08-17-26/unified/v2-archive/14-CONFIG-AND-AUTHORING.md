# 14 — Config and Authoring

> TOML is the lingua franca. Every Graph, Rack, Trigger, Space, and Agent is declared in TOML. The 5-tier SPI enables contributions from pure Markdown prompts to compiled Rust, with progressive capability and progressive isolation. Domain profiles are complete cognitive postures, not string labels.

**Depends on**: [01-SIGNAL](01-SIGNAL.md) (Signal/Pulse), [02-CELL](02-CELL.md) (9 protocols, typed I/O, capabilities), [03-GRAPH](03-GRAPH.md) (composition), [04-SPECIALIZATIONS](04-SPECIALIZATIONS.md) (Rack, Space, Agent), [08-EXTENSION-SYSTEM](08-EXTENSION-SYSTEM.md) (CaMeL IFC), [17-SECURITY-MODEL](17-SECURITY-MODEL.md) (capability intersection)

---

## 1. Directory Layout

```
<workspace>/
+-- workspace.toml                    # Space config: capabilities, models, deploy targets
+-- .roko/
    +-- graphs/                       # Graph definitions (TOML)
    |   +-- doc-ingest.toml
    |   +-- prd-draft.toml
    |   +-- deploy.toml
    +-- cells/                       # Cell definitions (non-built-in)
    |   +-- markdown-classify.toml
    |   +-- perplexity-search.toml
    +-- triggers/                     # Trigger bindings
    |   +-- ingest-on-new-doc.toml
    +-- racks/                        # Parameterized Graphs (Racks)
    |   +-- visual-quality.toml
    +-- extensions/                   # Extension manifests + implementations
    |   +-- custom-linter/
    |   |   +-- manifest.toml
    |   |   +-- linter.wasm
    |   +-- report-writer/
    |       +-- manifest.toml
    |       +-- config.toml
    +-- prompts/                      # Tier 1: Prompt packages
    |   +-- code-review-system.md
    |   +-- research-protocol.md
    +-- profiles/                     # Tier 2: Config profile bundles
    |   +-- defi-trading.toml
    |   +-- security-audit.toml
    +-- tools/                        # Tier 3: Declarative tool manifests
    |   +-- github-pr-review.toml
    |   +-- docker-deploy.toml
    +-- plugins/                      # Tier 4: WASM plugin binaries + manifests
        +-- my-org.markdown-classify-1.2.3.wasm
        +-- manifest.toml

~/.roko/
+-- config.toml                       # User-level configuration
+-- graphs/                           # User-level (across Spaces)
+-- cells/
+-- triggers/
+-- racks/
+-- prompts/
+-- profiles/
+-- tools/
+-- plugins/

<roko-install>/
+-- builtin/
    +-- graphs/                       # Ships with roko
    +-- cells/
    +-- racks/
    +-- prompts/
```

Resolution order at load time: **workspace > user > builtin**. By name and semver requirement. Workspace-level definitions shadow user-level, which shadow built-in.

---

## 2. Cell TOML Schema

A Cell TOML declares a Cell and points at its implementation.

```toml
# .roko/cells/markdown-classify.toml

[block]
name        = "markdown-classify"
version     = "1.0.0"
description = "Classifies markdown segments by intent (context, task, spec, reference)"
publisher   = "@wpank"
license     = "CC-BY-4.0"
tags        = ["doc", "ingest", "classify"]

[block.input]
schema = """
type: object
required: [segments]
properties:
  segments:
    type: array
    items:
      type: object
      required: [text, source]
      properties:
        text: { type: string }
        source:
          type: object
          required: [path, start_line, end_line]
"""

[block.output]
schema = """
type: object
required: [classifications]
properties:
  classifications:
    type: array
    items:
      type: object
      required: [segment_index, kind, confidence]
"""

[cell.capabilities]
required = ["llm"]                      # only LLM access; no fs / net / shell

[block.estimate_cost]
usd_per_unit    = 0.002
seconds_per_unit = 1.5

# Implementation tier -- exactly one of the 5 tiers
[block.impl]
tier   = "rust"
crate  = "roko-builtin-doc"
type   = "MarkdownClassifyCell"
```

Rust type backing the TOML schema:

```rust
pub struct CellManifest {
    pub name: String,
    pub version: Version,
    pub description: String,
    pub publisher: Option<String>,
    pub license: Option<String>,
    pub tags: Vec<String>,
    pub input: TypeSchema,
    pub output: TypeSchema,
    pub capabilities: CapabilityDeclaration,
    pub estimate_cost: Option<CostEstimate>,
    pub impl_tier: ImplTier,
}

pub enum ImplTier {
    Prompt,                                // Tier 1
    Config,                                // Tier 2
    Declarative { invoke: InvokeSpec },    // Tier 3
    Wasm { path: PathBuf, checksum: String, memory_mb: u32, fuel: u64 },  // Tier 4
    Rust { crate_name: String, type_name: String },  // Tier 5
}
```

---

## 3. Graph TOML Schema

A Graph TOML declares the composition of Cells into a node/edge graph with Macros, Slots, and policy ([doc-03](03-GRAPH.md)).

```toml
# .roko/graphs/doc-ingest.toml

[graph]
name        = "doc-ingest"
version     = "1.0.0"
description = "Ingest a directory of markdown into PRDs, plans, and tasks"
publisher   = "@wpank"
license     = "CC-BY-4.0"
tags        = ["doc", "ingest", "authoring"]
forked_from = "@nunchi/doc-ingest@0.9.0"

[graph.input]
schema = """
type: object
required: [source_dir]
properties:
  source_dir:    { type: string }
  incremental:   { type: boolean, default: true }
  new_files:     { type: array, items: { type: string } }
"""

[graph.output]
schema = """
type: object
properties:
  created_prds:  { type: array }
  created_plans: { type: array }
  audit_report:  { type: string }
"""
```

### 3.1 Nodes

```toml
[[graph.node]]
id     = "walk"
kind   = "block"
block  = "fs-walk@^1"
[graph.node.params]
patterns = ["**/*.md"]
ignore   = ["**/.git/**"]

[[graph.node]]
id     = "classify"
kind   = "block"
block  = "markdown-classify@^1"

[[graph.node]]
id     = "synthesize-prd"
kind   = "block"
block  = "prd-synthesize@^1"
[graph.node.params]
model = "claude-opus-4-7"
role  = "strategist"

[[graph.node]]
id     = "enrich"
kind   = "block"
block  = "{{ slot.researcher }}"
condition = "macros.enable_web_research == true"

[[graph.node]]
id     = "refine-loop"
kind   = "loop"
body   = "synthesize-prd"
until  = "audit.findings.severity_max < 'high'"
max_iterations = 2
```

Node kinds:
- `block` -- execute a Cell
- `sub-graph` -- recursively execute another Graph
- `branch` -- conditional fan-out (evaluate condition, walk matching edges)
- `fan-out` -- parallel fan-out (iterate expression, spawn one child per element)
- `fan-in` -- merge parallel branches (strategies: concat, vote, first, reduce)
- `loop` -- repeat body until predicate or max iterations
- `human-input` -- pause for user input (feeds Agent Inbox surface, [doc-16](16-SURFACES.md))
- `wait` -- pause for a duration or external signal

### 3.2 Edges

```toml
[[graph.edge]]
from = "walk"
to   = "classify"
[[graph.edge.maps]]
from = "files"
to   = "segments"

[[graph.edge]]
from      = "synthesize-prd"
to        = "enrich"
condition = "macros.enable_web_research == true"

[[graph.edge]]
from = "plan"
to   = "persist"
```

Edges carry:
- **Maps**: field-level routing from source output to target input
- **Conditions**: Expr-language predicates controlling edge traversal
- **Adapters**: optional adapter Cell reference for type conversion

### 3.3 Policy

```toml
[graph.policy]
budget_usd        = 5.0
deadline_seconds  = 1800
on_block_failure  = "retry-with-escalation"
max_retries       = 2
human_input_default = "human"
parallelism       = 4
```

---

## 4. Rack TOML Schema (Macros and Slots)

A Rack is a Graph with exposed Macros (knobs) and Slots (jacks) ([doc-04](04-SPECIALIZATIONS.md)). Rack-specific fields extend the Graph schema. The DAW metaphor: Macros are the performer's knobs, Slots are the patch jacks.

### 4.1 Macros (knobs)

```toml
[[graph.macro]]
name        = "enable_audit"
label       = "Audit findings"
description = "Run an audit pass after synthesis"
kind        = "boolean"
default     = true
bindings    = [{ node = "audit", param = "enabled" }]

[[graph.macro]]
name        = "synthesizer_model"
kind        = "model-ref"
default     = "claude-opus-4-7"
bindings    = [
  { node = "synthesize-prd", param = "model" },
  { node = "refine",         param = "model" },
]

[[graph.macro]]
name        = "budget_usd"
kind        = "money"
currency    = "USD"
max         = 50.0
default     = 5.0
bindings    = [{ graph = ".policy.budget_usd" }]
```

A single Macro can fan out across multiple internal Cells. Setting `macro.strictness = "high"` might bind to `auditor.threshold = 0.9`, `synthesizer.temperature = 0.3`, and `reviewer.iterations = 3` simultaneously.

Macro kinds:

```rust
pub enum MacroKind {
    Boolean,                                    // toggle
    Enum { variants: Vec<String> },             // segmented control
    Integer { min: i64, max: i64, step: i64 },  // stepper
    Float { min: f64, max: f64, step: f64 },    // rotary knob
    Text { pattern: Option<String> },           // text input
    Money { currency: String, max: f64 },       // budget slider
    ModelRef,                                    // searchable model dropdown
    AgentRef,                                    // agent picker
    SlotRef,                                     // the Macro IS the Slot's filling
}
```

### 4.2 Slots (jacks)

```toml
[[graph.slot]]
name           = "researcher"
label          = "Web Researcher"
description    = "Cell that performs web search; defaults to Perplexity"
accepts        = "any-cell-with-tag"
tag            = "web-research"
input_schema   = { type = "object", required = ["query"] }
output_schema  = { type = "object", required = ["citations", "summary"] }
required       = false
default_filling = { block = "perplexity-search", version = "^1" }
```

Slots accept:
- `any-block` -- any Cell whose types match
- `any-graph` -- any Graph whose types match
- `any-cell-with-tag { tag }` -- Cell matching a tag
- `specific-capability { capability }` -- Cell with a specific capability

Slots are the composability hinge. A `research-pipeline` Rack has slots for "Researcher" and "Verifier" -- consumers plug in any Cell whose types match, without forking the parent. Marketplace artifacts can be Slot fillings ([doc-15](15-MARKETPLACE-AND-SHARING.md)).

---

## 5. The 5-Tier Package SPI

The Service Provider Interface defines five tiers for authoring Cells, Extensions, Graphs, and agent capabilities. Each tier balances expressiveness against isolation. Progressive capability with progressive trust.

### Tier 1: Prompts (pure Markdown/TOML front-matter, no execution)

The lowest-friction tier. A prompt package is a Markdown file with TOML front-matter that declares metadata. No code executes -- the content is injected into the system prompt or context assembly.

```markdown
---
name = "code-review-system"
version = "1.0.0"
description = "System prompt for thorough code reviews"
tags = ["coding", "review"]
target = "system_prompt"
layer = "cognition"
---

# Code Review Protocol

When reviewing code changes, follow these steps:

1. **Read the diff completely** before commenting.
2. **Check for security issues** first: injection, auth bypass, data exposure.
3. **Check for correctness**: edge cases, error handling, resource leaks.
4. **Check for clarity**: naming, structure, documentation.
5. **Suggest improvements** with concrete alternatives, not vague criticism.
```

**Sandboxing**: None needed. No execution occurs.
**Distribution**: Marketplace ([doc-15](15-MARKETPLACE-AND-SHARING.md)). Anyone can publish prompts.
**Use cases**: System prompt customization, review protocols, domain instructions, research methodologies.

### Tier 2: Config Profiles (TOML bundles layering onto roko.toml)

A config profile is a TOML bundle that layers onto `roko.toml`, customizing agent behavior without writing code. Profiles configure existing capabilities -- they do not add new ones.

```toml
# .roko/profiles/defi-trading.toml

[profile]
name        = "defi-trading"
version     = "1.0.0"
description = "Config profile for DeFi trading agents"
tags        = ["trading", "defi", "finance"]
base        = "trading"  # extends built-in trading profile

[profile.clock]
gamma_ms = 200           # fast gamma for price ticks
theta_ms = 2000
delta_ms = 60000

[profile.extensions]
enabled = ["chain-reader", "risk-manager", "price-feed", "safety", "cost-tracker"]

[profile.models]
primary   = "claude-opus-4-6"
fallback  = "claude-sonnet-4-6"
reflexive = "claude-haiku-4-5"

[profile.gates]
compile = false
test    = false
risk    = { enabled = true, max_position_usd = 10000 }

[profile.context_weights]
neuro     = 0.4
task      = 0.3
research  = 0.2
heuristic = 0.1
```

**Sandboxing**: None. Config profiles do not execute; they configure.
**Distribution**: Marketplace. Profiles are reviewed for sanity (no absurd budgets, no disabled safety gates without justification).

### Tier 3: Declarative Tools (TOML manifests for subprocess/HTTP/MCP, sandboxed)

A declarative tool wraps a subprocess, HTTP endpoint, or MCP server as a Cell. The manifest declares I/O schemas, capability requirements, and invocation details. The runtime handles sandboxing.

```toml
# .roko/tools/github-pr-review.toml

[tool]
name        = "github-pr-review"
version     = "1.0.0"
description = "Fetch and review GitHub pull requests"
tags        = ["github", "review", "coding"]

[tool.input]
schema = """
type: object
required: [repo, pr_number]
properties:
  repo:      { type: string }
  pr_number: { type: integer }
"""

[tool.output]
schema = """
type: object
required: [diff, files, comments]
"""

[tool.capabilities]
required = ["net"]

# Invocation: subprocess
[tool.invoke]
kind    = "subprocess"
command = ["gh", "pr", "view", "{{input.pr_number}}", "--repo", "{{input.repo}}", "--json", "body,files,comments,diff"]
stdin   = "none"
stdout  = "json"
timeout_seconds = 30

# Alternative: HTTP invocation
# [tool.invoke]
# kind    = "http"
# method  = "GET"
# url     = "https://api.github.com/repos/{{input.repo}}/pulls/{{input.pr_number}}"
# headers = { "Authorization" = "Bearer {{secrets.github_token}}" }

# Alternative: MCP invocation
# [tool.invoke]
# kind    = "mcp"
# server  = "github"
# method  = "get_pull_request"
```

**Sandboxing**: OS-level process isolation. Subprocess inherits only declared capabilities. Network restricted to declared domains. Filesystem restricted to declared paths.
**Distribution**: Verified publishers. Declarative tools can invoke subprocesses, so they require publisher verification.

### Tier 4: WASM (sandboxed, fuel-metered, marketplace-recommended)

WebAssembly modules using a roko-defined ABI (`wit-bindgen` interfaces). The recommended tier for marketplace artifacts: deterministic builds, sandboxing for free, capability declarations enforced by the host.

```toml
[block.impl]
tier      = "wasm"
path      = ".roko/plugins/markdown-classify-1.2.3.wasm"
checksum  = "blake3:abc123..."
memory_mb = 64
fuel      = 100_000_000              # WASM execution-fuel cap
```

**Sandboxing**: WASM sandbox. Memory isolated. Fuel-metered (execution bounded). Capability-gated -- the host only exposes declared capabilities via the ABI.
**Distribution**: Marketplace (recommended default). WASM modules are portable, deterministic, safe.

### Tier 5: Native Rust (compiled, full trust, in-tree only)

Compiled Rust code implementing `impl Cell for MyCell` or `impl Extension for MyExt`. Highest performance, no sandboxing. Reserved for built-in components and trusted in-tree plugins.

```toml
[block.impl]
tier  = "rust"
crate = "roko-builtin-doc"
type  = "MarkdownClassifyCell"
```

**Sandboxing**: Process-level only. Full trust.
**Distribution**: Compiled into the binary. Marketplace artifacts may NOT use this tier directly -- they must compile to WASM.

### Tier comparison table

| Property | 1. Prompts | 2. Config | 3. Declarative | 4. WASM | 5. Rust |
|---|---|---|---|---|---|
| Executes code | No | No | Subprocess/HTTP | WASM sandbox | Native |
| Sandboxing | N/A | N/A | OS-level process | WASM + fuel | Process-level |
| Capability control | N/A | N/A | Declared, enforced | ABI-gated | Full trust |
| Marketplace | Anyone | Anyone | Verified publisher | Anyone | Not directly |
| Performance | N/A | N/A | Subprocess overhead | Near-native | Native |
| Deterministic builds | N/A | N/A | No | Yes | Depends |
| Friction | Lowest | Low | Medium | Medium | Highest |

### Tier selection guidance

| Use case | Tier | Why |
|---|---|---|
| System prompt customization | 1. Prompts | No execution risk, instant iteration |
| Domain specialization (clock, models, gates) | 2. Config | Configuration, not code |
| Wrapping existing CLI tools or APIs | 3. Declarative tools | No code needed, sandboxed |
| Marketplace artifact, custom logic | 4. WASM | Sandboxed, deterministic, portable |
| In-tree built-in (gates, routers, composers) | 5. Rust | Performance, full access |

The visual editor ([doc-16](16-SURFACES.md)) only writes TOML (Tiers 1-3). WASM and Rust tiers require build tools.

---

## 6. Domain Profiles as Cognitive Postures

A domain profile is a **complete cognitive posture** -- not just a string label like `"coding"`. It bundles clock configuration, extensions, wakeup events, context weights, gate configuration, and infrastructure settings into a coherent whole that determines the Agent's entire behavioral surface.

### Profile schema

```rust
pub struct DomainProfile {
    pub name: String,
    pub version: Version,
    pub description: String,
    pub base: Option<String>,           // extends another profile
    pub clock: ClockConfig,
    pub extensions: ExtensionConfig,
    pub wakeup: WakeupConfig,
    pub context_weights: ContextWeights,
    pub gates: GateConfig,
    pub infrastructure: InfraConfig,
    pub models: ModelConfig,
}

pub struct ClockConfig {
    pub gamma_ms: u64,                  // perception tick
    pub theta_ms: u64,                  // inference tick
    pub delta_ms: u64,                  // reflection tick
    pub regime: Regime,                 // Calm / Normal / Volatile / Crisis
}

pub struct ContextWeights {
    pub neuro: f64,                     // knowledge store context
    pub task: f64,                      // task-specific context
    pub research: f64,                  // research findings
    pub heuristic: f64,                 // learned heuristics
    pub episode: f64,                   // past episodes
    pub pheromone: f64,                 // stigmergic signals
    pub affect: f64,                    // somatic markers
    pub system: f64,                    // system instructions
}
```

### Profile TOML

```toml
[profile]
name        = "security-audit"
version     = "1.0.0"
description = "Complete cognitive posture for security auditing"

[profile.clock]
gamma_ms = 300
theta_ms = 5000
delta_ms = 300000         # 5 min reflection
regime   = "normal"

[profile.extensions]
enabled  = ["safety", "vuln-scanner", "dependency-audit", "cost-tracker", "circuit-breaker", "camel-monitor"]

[profile.wakeup]
events = [
  "signal:kind:Code",
  "pulse:gate.verdict.emitted",
  "trigger:cron:0 */4 * * *",
  "trigger:webhook:/hooks/audit",
]

[profile.context_weights]
neuro     = 0.15
task      = 0.20
research  = 0.30
heuristic = 0.10
episode   = 0.10
pheromone = 0.05
affect    = 0.05
system    = 0.05

[profile.gates]
compile   = true
test      = true
clippy    = true
diff      = true
vuln_scan = { enabled = true, severity = "medium" }
llm_judge = { enabled = true, model = "claude-sonnet-4-6" }

[profile.infrastructure]
execution  = "in-process"
budget_usd = 25.0
max_tokens = 200000
max_duration = "2h"

[profile.models]
primary   = "claude-opus-4-6"
fallback  = "claude-sonnet-4-6"
reflexive = "claude-haiku-4-5"
```

### Profile comparison table

Setting `profile = "X"` configures every dimension simultaneously. The profile IS the Agent's cognitive posture.

| Dimension | Coding | Security Audit | Research | Trading |
|---|---|---|---|---|
| **Clock (gamma)** | 200ms | 300ms | 500ms | 100ms |
| **Clock (theta)** | 3000ms | 5000ms | 10000ms | 1000ms |
| **Extensions** | git, compiler, test-runner | vuln-scanner, dep-audit | web-search, citation-check | chain-reader, risk-mgr |
| **Wakeup** | Code changes, PR events | Scheduled scans, vuln feeds | Manual trigger, cron | Price ticks, chain events |
| **Context** | High task (0.4) | High research (0.3) | High research (0.4) | High task (0.4), high neuro (0.3) |
| **Gates** | compile, test, clippy | vuln_scan, llm_judge, diff | fact-check, citation | risk, position-limit |
| **Budget** | $10, ephemeral | $25, persistent | $15, ephemeral | $50, persistent |

### Profile inheritance

Profiles can extend other profiles via `base`:

```toml
[profile]
name = "defi-security-audit"
base = "security-audit"           # inherits all settings

[profile.extensions]
enabled = ["chain-reader", "slither-analyzer"]   # added on top of base

[profile.gates]
vuln_scan = { severity = "low" }                 # stricter threshold
```

Deep merge: arrays are concatenated (Extensions), objects are merged (gates config), scalars are overridden (clock values).

---

## 7. Workspace Scoping

Roko supports multi-workspace operation. A single daemon can serve multiple workspaces, each with its own capability grants, knowledge scope, and resource limits.

### workspace.toml

```toml
[space]
name           = "nunchi-dashboard"
schema_version = 1
extends        = "~/.roko/templates/web-app"

[space.capabilities]
fs_read       = true
fs_write      = true
net           = { domains = ["*"] }
llm           = true
shell         = { commands = ["cargo", "git", "npm"] }
chain_write   = false
secrets       = { keys = ["anthropic_key", "openai_key", "railway_token"] }

[space.models]
strategist = "claude-opus-4-7"
researcher = "claude-sonnet-4-6"
scribe     = "claude-haiku-4-5"
default    = "claude-sonnet-4-6"

[[space.deploy]]
name    = "railway"
default = true
[space.deploy.params]
service = "nunchi-dashboard"

[space.knowledge]
scope       = "workspace"             # workspace | user | global
share_with  = ["tag:nunchi"]
import_from = ["roko"]
```

### Multi-workspace daemon

```toml
# ~/.roko/daemon.toml
[daemon]
port = 6677
workspaces = [
  { path = "/Users/will/dev/nunchi/roko/roko",   name = "roko" },
  { path = "/Users/will/dev/nunchi/dashboard",    name = "dashboard" },
  { path = "/Users/will/dev/nunchi/relay",        name = "relay" },
]

[daemon.limits]
max_agents_per_workspace = 20
max_total_agents = 50
max_budget_per_workspace_usd = 100.0
```

### Per-workspace capability grants

Each workspace has its own capability surface. An Agent in the `roko` workspace cannot access the `dashboard` workspace's filesystem unless the daemon grants cross-workspace access.

### Cross-workspace knowledge sharing

Knowledge Signals ([doc-11](11-MEMORY-AND-KNOWLEDGE.md)) are scoped to their workspace by default. Cross-workspace sharing is explicit:

```toml
# roko workspace shares coding heuristics
[space.knowledge]
share_with = ["tag:nunchi"]
share_kinds = ["Heuristic", "Insight"]

# dashboard workspace imports from roko
[space.knowledge]
import_from = ["roko"]
import_filter = { min_tier = "Consolidated" }
```

Shared Signals carry their origin workspace tag in CaMeL provenance ([doc-08](08-EXTENSION-SYSTEM.md), [doc-17](17-SECURITY-MODEL.md)). The receiving workspace can query but not modify the original.

---

## 8. Capability Declarations (Three-Layer Intersection)

Capabilities are declared on Cells and Triggers, granted at the Space, and intersected at runtime.

### Layer 1: Cell declarations

```toml
[cell.capabilities]
required = [
  { "FsRead"  = { paths = ["docs/**", "src/**"] } },
  { "FsWrite" = { paths = [".roko/artifacts/**"] } },
  { "Shell"   = { commands = ["cargo", "rustc", "git"] } },
  { "Net"     = { domains = ["api.openai.com", "api.anthropic.com"] } },
  { "Secrets" = { keys = ["openai_key", "anthropic_key"] } },
]
```

### Layer 2: Graph allow-list

```toml
[graph.capabilities]
allowed = ["FsRead", "FsWrite", "Llm"]   # only these capabilities available to nodes
```

### Layer 3: Space grants

```toml
[space.capabilities]
fs_read       = true
fs_write      = true
net           = { domains = ["*"] }
llm           = true
shell         = false
chain_write   = false
```

### Three-layer intersection

A Cell may run only when all three layers permit:

```
Cell declaration  (intersection)  Graph allow-list  (intersection)  Space grant  =  effective capabilities
```

Missing at any layer = denied. The system fails closed. CaMeL IFC ([doc-17](17-SECURITY-MODEL.md)) tags capability provenance through the execution chain.

```rust
pub fn effective_capabilities(
    block: &CapabilityDeclaration,
    graph: &GraphCapabilities,
    space: &SpaceGrants,
) -> EffectiveCapabilities {
    let intersection = block.required.iter()
        .filter(|cap| graph.allows(cap))
        .filter(|cap| space.grants(cap))
        .cloned()
        .collect();
    EffectiveCapabilities { granted: intersection }
}
```

### CLI disclosure on install

```
$ roko cell install @my-org/markdown-classify
Cell: @my-org/markdown-classify@1.2.3
Tier: WASM (sandboxed, fuel-metered)
Capabilities required:
  - Llm                   (any provider)
  - FsRead                (any path)
  - Net                   (api.openai.com, api.perplexity.ai)
This Space currently grants: Llm, FsRead, Net (api.* allowed).
All capabilities granted. Continue? [Y/n]
```

---

## 9. Plugin Discovery and Loading

At Space open, the engine:

1. Loads `<workspace>/.roko/plugins/manifest.toml` if present.
2. Loads `~/.roko/plugins/manifest.toml` if present.
3. Loads built-in plugins from `<roko-install>/builtin/plugins/`.
4. Scans Tier 1-3 directories (`prompts/`, `profiles/`, `tools/`) for manifest files.
5. Registers every Cell / Trigger / Graph / Rack / Prompt / Profile / Tool declared by any manifest.

```toml
# manifest.toml
schema_version = 1

[[plugin]]
name      = "@my-org/markdown-classify"
version   = "1.2.3"
kind      = "block"
tier      = "wasm"
declares  = ".roko/cells/markdown-classify.toml"
checksum  = "blake3:..."
installed_at = "2026-04-20T12:00:00Z"
```

---

## 10. Validation (12 Checks at Load Time)

At load time the engine runs:

1. **TOML parsing** -- strict mode; unknown keys error.
2. **Schema validation** -- JSON-schema validation of input/output TypeSchema declarations.
3. **Tier validation** -- verify implementation tier matches declared tier, check sandboxing constraints.
4. **Reference resolution** -- every `block = "..."` reference resolved against registry; unresolved references error.
5. **Capability check** -- for every Cell, check Space grants cover declared capabilities; missing capabilities prompt user grant or error.
6. **Type checking** -- every edge's source output -> target input compatibility verified; missing adapters error.
7. **Cycle detection** -- Graph cycles outside `loop` nodes error.
8. **Slot completeness** -- every required Slot has a filling (default or user-provided).
9. **Macro coverage** -- every Macro binding points to a real node + param.
10. **Cost estimation** -- sum of estimated costs vs budget; warns if estimate > budget.
11. **Trigger validation** -- source-kind specifics validated (cron syntax, regex compilability, webhook auth presence).
12. **CaMeL capability validation** -- verify Extension capability declarations are consistent with CaMeL IFC rules ([doc-17](17-SECURITY-MODEL.md)).

`roko graph validate <name>` runs these checks ahead of execution. The dashboard validation panel shows the same checks with green/yellow/red indicators.

---

## 11. Versioning

Semver everywhere. Graphs, Cells, Triggers, Racks, Prompts, Profiles, and Tools all carry `version`. References use semver requirements.

```toml
block       = "markdown-classify"
version_req = "^1.0"          # >=1.0.0, <2.0.0
```

At Graph load, the engine resolves the highest version satisfying the requirement, pins the resolved version onto the Flow snapshot, and uses that for the duration of the run. Re-running the same Graph at the same `(graph@version, input)` resolves to the same Cells. Reproducibility by default.

When a Graph is published to the marketplace ([doc-15](15-MARKETPLACE-AND-SHARING.md)), all Cell references are pinned to exact versions (`=1.2.3`) automatically. Forks may relax pins.

### Lockfile

`<workspace>/.roko/marketplace.lock` pins exact versions and checksums for installed marketplace artifacts. Ensures reproducible installs across machines.

---

## 12. Configuration Hierarchy

Three layers, deep-merged:

1. **Space**: `<workspace>/workspace.toml` -- top precedence
2. **User**: `~/.roko/config.toml` -- middle
3. **Built-in defaults** -- bottom

CLI flags override config. Environment variables (`ROKO_*`) override config but are overridden by flags.

User-level `~/.roko/config.toml`:

```toml
[ui]
default_view  = "tui"
color         = "auto"
density       = "comfortable"

[runs]
default_detach   = false
default_confirm  = "interactive"
recent_history_n = 50

[notifications]
on_completion = ["desktop"]
on_failure    = ["desktop", "slack"]
on_human_input = ["desktop", "slack"]
slack_channel = "#roko-runs"
```

---

## 13. Acceptance Criteria

| Criterion | Verification |
|---|---|
| Graph TOML round-trips through parser -> model -> serialization with identity preserved | Unit test on representative Graph |
| Schema validation rejects malformed input/output at load time, not run time | Negative test cases |
| Reference resolution finds Cells across workspace > user > builtin in order | Integration test with shadowing |
| Capability intersection enforced at runtime; Cell run rejected if capability denied | Cell attempting `Net` without capability fails closed |
| Edge type-checking catches unwirable connections at validate time | Negative test: wire `String` to `i32` with no adapter |
| Loop bound enforced; engine errors before unbounded execution | Synthetic infinite-loop Cell terminates after `max_iterations` |
| Slot defaults applied; required Slots without defaults error at load time | Two test cases per |
| Macro bindings: setting one Macro updates multiple internal node params | Multi-binding Macro test |
| Tier 1 prompt loads and injects into system prompt context | Load .md prompt, verify in composed context |
| Tier 2 config profile deep-merges over base profile | Profile with `base` extends parent correctly |
| Tier 3 declarative tool invokes subprocess with sandboxed capabilities | Tool tries disallowed path -> fails; allowed command -> succeeds |
| Tier 4 WASM plugin loads, runs, sandboxed (no fs without capability) | WASM Cell attempts file write without capability -> fails |
| Tier 5 Rust Cell compiles and runs with full access | Built-in Cell executes normally |
| Domain profile configures all dimensions simultaneously | Set `profile = "security-audit"`, verify clock + extensions + gates + models all match |
| Profile inheritance: child overrides parent cleanly | Child profile with `base`, verify deep merge |
| Multi-workspace daemon isolates capabilities per workspace | Agent in workspace A cannot access workspace B filesystem |
| Cross-workspace knowledge sharing respects scope and filter | Share Heuristic from A, verify in B with correct provenance |
| CaMeL capability tags validated at load time | Extension declaring caps beyond Space grants fails validation |
| 12 validation checks all run on `roko graph validate` | One test per check |
