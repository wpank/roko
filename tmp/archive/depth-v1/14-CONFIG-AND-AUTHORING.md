# 14 — Config and Authoring

> TOML is the lingua franca. Every Graph, Rack, Trigger, Space, and Agent is declared in TOML. Implementations may be Rust, WASM, script, or pure composition.

**Source**: wf-04 (Configuration & Authoring), updated to unified vocabulary.

---

## 1. Where Things Live

```
<workspace>/
├── workspace.toml                    # Space config: capabilities, models, deploy targets
└── .roko/
    ├── graphs/                       # Graph definitions (TOML)
    │   ├── doc-ingest.toml
    │   ├── prd-draft.toml
    │   └── deploy.toml
    ├── blocks/                       # Block definitions (non-built-in)
    │   ├── markdown-classify.toml
    │   ├── perplexity-search.toml
    │   └── citation-grounder.toml
    ├── triggers/                     # Trigger bindings
    │   └── ingest-on-new-doc.toml
    ├── racks/                        # Parameterized Graphs (Racks)
    │   └── visual-quality.toml
    └── plugins/                      # WASM / script / native plugin manifests
        ├── my-org.markdown-classify-1.2.3.wasm
        └── manifest.toml

~/.roko/
├── graphs/                           # User-level (across Spaces)
├── blocks/
├── triggers/
├── racks/
└── plugins/

<roko-install>/
└── builtin/
    ├── graphs/                       # Ships with roko
    ├── blocks/
    └── racks/
```

Resolution order at load time: **workspace > user > builtin**. By name and semver requirement.

---

## 2. Block TOML Schema

A Block TOML declares a Block and points at its implementation.

```toml
# .roko/blocks/markdown-classify.toml

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

[block.capabilities]
required = ["llm"]                      # only LLM access; no fs / net / shell

[block.estimate_cost]
usd_per_unit    = 0.002
seconds_per_unit = 1.5

# Implementation tier — exactly one of: rust / wasm / script / composition
[block.impl]
tier   = "rust"
crate  = "roko-builtin-doc"
type   = "MarkdownClassifyBlock"
```

---

## 3. Graph TOML Schema

A Graph TOML declares the composition of Blocks into a node/edge graph with Macros, Slots, and policy.

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
- `block` — execute a Block
- `sub-graph` — recursively execute another Graph
- `branch` — conditional fan-out (evaluate condition, walk matching edges)
- `fan-out` — parallel fan-out (iterate expression, spawn one child per element)
- `fan-in` — merge parallel branches (strategies: concat, vote, first, reduce)
- `loop` — repeat body until predicate or max iterations
- `human-input` — pause for user input
- `wait` — pause for a duration or external signal

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
from      = "audit"
to        = "refine-loop"
condition = "audit.findings.severity_max >= 'high' AND macros.enable_audit == true"

[[graph.edge]]
from = "plan"
to   = "persist"
```

Edges carry:
- **Maps**: field-level routing from source output to target input
- **Conditions**: Expr-language predicates controlling edge traversal
- **Adapters**: optional adapter Block reference for type conversion

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

A Rack is a Graph with exposed Macros (knobs) and Slots (jacks). Rack-specific fields extend the Graph schema.

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

A single Macro can fan out across multiple internal Blocks. Setting `macro.strictness = "high"` might bind to `auditor.threshold = 0.9`, `synthesizer.temperature = 0.3`, and `reviewer.iterations = 3` simultaneously.

Macro kinds:
- `boolean` — toggle
- `enum { variants }` — segmented control
- `integer { min, max, step }` — stepper
- `float { min, max, step }` — rotary knob
- `text { pattern }` — text input
- `money { currency, max }` — budget slider
- `model-ref` — searchable model dropdown
- `agent-ref` — agent picker
- `slot-ref` — the Macro IS the Slot's filling

### 4.2 Slots (jacks)

```toml
[[graph.slot]]
name           = "researcher"
label          = "Web Researcher"
description    = "Block that performs web search; defaults to Perplexity"
accepts        = "any-block-with-tag"
tag            = "web-research"
input_schema   = { type = "object", required = ["query"] }
output_schema  = { type = "object", required = ["citations", "summary"] }
required       = false
default_filling = { block = "perplexity-search", version = "^1" }
```

Slots accept:
- `any-block` — any Block whose types match
- `any-graph` — any Graph whose types match
- `any-block-with-tag { tag }` — Block matching a tag
- `specific-capability { capability }` — Block with a specific capability

Slots are the composability hinge. A `research-pipeline` Rack has slots for "Researcher" and "Verifier" — consumers plug in any Block whose types match, without forking the parent.

---

## 5. Trigger TOML Schema

Trigger TOMLs live in `<workspace>/.roko/triggers/` and `~/.roko/triggers/`.

```toml
# .roko/triggers/ingest-on-new-doc.toml

[trigger]
name    = "ingest-on-new-doc"
enabled = true

[trigger.source]
block = "file-watch-trigger@^1"
[trigger.source.params]
path       = "docs/"
patterns   = ["**/*.md"]
debounce_ms = 500

[trigger.binding]
graph = "doc-ingest@^1"
[trigger.binding.input]
source_dir  = "docs/"
incremental = true
[trigger.binding.macros]
enable_web_research = false

[trigger.policy]
concurrency = "queue"
max_depth   = 16
```

See [doc-06 (Trigger System)](06-TRIGGER-SYSTEM.md) for the full Trigger model.

---

## 6. Space Configuration

Spaces (workspaces) are configured in `workspace.toml` at the workspace root.

```toml
[space]
name           = "nunchi-dashboard"
schema_version = 1
extends        = "~/.roko/templates/web-app"

# ── Capability grants ──────────────────────────────────────
[space.capabilities]
fs_read       = true
fs_write      = true
net           = { domains = ["*"] }
llm           = true
shell         = { commands = ["cargo", "git", "npm"] }
chain_write   = false
secrets       = { keys = ["anthropic_key", "openai_key", "railway_token"] }

# ── Model routing ──────────────────────────────────────────
[space.models]
strategist = "claude-opus-4-7"
researcher = "claude-sonnet-4-6"
scribe     = "claude-haiku-4-5"
default    = "claude-sonnet-4-6"

# ── Deploy targets ─────────────────────────────────────────
[[space.deploy]]
name    = "railway"
default = true
[space.deploy.params]
service = "nunchi-dashboard"

[[space.deploy]]
name = "fly-staging"
[space.deploy.params]
app = "nunchi-dashboard-staging"

# ── Knowledge sharing ──────────────────────────────────────
[space.knowledge]
share_with  = ["tag:nunchi"]
import_from = ["roko"]

# ── Graph confirmation policy ──────────────────────────────
[space.confirm]
"doc-ingest" = "never"
"deploy"     = "always"
"*"          = "interactive"
```

---

## 7. Agent Configuration

Agents are configured in `roko.toml` or in per-agent TOML files.

```toml
# roko.toml — agent section

[[agent]]
name    = "coder"
profile = "coding"
mode    = "ephemeral"

[agent.extensions]
enabled = ["safety", "budget", "episode-logger"]

[agent.connectors]
enabled = ["mcp-code"]

[agent.mcp_config]
path = ".roko/mcp-config.json"

[agent.models]
primary   = "claude-opus-4-6"
fallback  = "claude-sonnet-4-6"
reflexive = "claude-haiku-4-5"

[agent.budget]
max_usd      = 10.0
max_tokens   = 100000
max_duration = "30m"

[agent.clock]
gamma_ms = 500
theta_ms = 4000
delta_ms = 120000
regime   = "normal"
```

Agent modes:
- `ephemeral` — runs until task completes, then stops
- `persistent` — runs tick loop indefinitely
- `reactive` — sleeps until Trigger fires, works, sleeps

---

## 8. Authoring Tiers

Blocks can be implemented at four tiers. The tier determines sandboxing and distribution.

### 8.1 Tier 1: Pure Composition (TOML)

A Block whose implementation is a Graph of other Blocks. Lowest friction, no execution sandboxing needed.

```toml
[block.impl]
tier  = "composition"
graph = ".roko/graphs/citation-and-grounding.toml"
```

Use case: combining built-in primitives without writing code. The visual editor emits this tier exclusively when the user wires Blocks together.

### 8.2 Tier 2: Scripts (Bash / Python / Node)

Block input is delivered on stdin (JSON); output expected on stdout (JSON); stderr is captured as logs.

```toml
[block.impl]
tier        = "script"
interpreter = "python3"
path        = ".roko/plugins/markdown-classify.py"
stdin       = "json"
stdout      = "json"
timeout_seconds = 60

[block.impl.env]
PYTHONUNBUFFERED = "1"
```

Script sandboxing:
- `FsRead` / `FsWrite`: restricted via path patterns. The engine prepares a temp directory, exposes only declared paths.
- `Net`: restricted via HTTP proxy that allowlists declared domains.
- `Shell`: not transitively granted. Scripts cannot spawn subprocesses unless `Shell` is explicitly declared.

Script manifest (`manifest.toml` alongside the script):

```toml
[block]
name        = "custom-classifier"
version     = "1.0.0"
description = "Custom text classification"
tags        = ["classify", "nlp"]

[block.capabilities]
required = ["llm"]

[block.impl]
tier        = "script"
interpreter = "python3"
path        = "classify.py"
```

### 8.3 Tier 3: WASM

WebAssembly modules using a roko-defined ABI (`wit-bindgen` interfaces). Sandboxed by default; capability-gated.

```toml
[block.impl]
tier      = "wasm"
path      = ".roko/plugins/markdown-classify-1.2.3.wasm"
checksum  = "blake3:abc123..."
memory_mb = 64
fuel      = 100_000_000              # WASM execution-fuel cap
```

WASM is the recommended tier for marketplace artifacts: deterministic builds, sandboxing for free, capability declarations enforced by the host.

### 8.4 Tier 4: Native Rust

Compiled into roko or into a plugin crate dynamically loaded at startup. Highest performance, no sandboxing.

```toml
[block.impl]
tier  = "rust"
crate = "roko-builtin-doc"
type  = "MarkdownClassifyBlock"
```

Used by built-in Blocks and trusted in-tree plugins. Marketplace artifacts may NOT use this tier directly; they must compile to WASM.

### 8.5 Tier Selection Guidance

| Use case | Tier | Why |
|---|---|---|
| In-tree built-in (gates, routers, composers) | Rust | Performance, full access |
| User wires existing Blocks into new Block | Composition | No execution risk |
| Glue around existing CLI tools | Script | Easy authoring, sandboxed |
| Marketplace artifact, possibly untrusted | WASM | Sandboxed, deterministic, portable |
| Performance-sensitive analytics | Rust (in-tree plugin) | No sandbox overhead |

The visual editor only writes Composition. Other tiers are written by hand or by build tools.

---

## 9. Capability Declarations

Capabilities are declared on Blocks and Triggers, granted at the Space, and intersected at runtime.

### 9.1 Block declarations

```toml
[block.capabilities]
required = [
  { "FsRead"  = { paths = ["docs/**", "src/**"] } },
  { "FsWrite" = { paths = [".roko/artifacts/**"] } },
  { "Shell"   = { commands = ["cargo", "rustc", "git"] } },
  { "Net"     = { domains = ["api.openai.com", "api.anthropic.com"] } },
  { "Secrets" = { keys = ["openai_key", "anthropic_key"] } },
]
```

### 9.2 Space grants

```toml
[space.capabilities]
fs_read       = true
fs_write      = true
net           = { domains = ["*"] }
llm           = true
shell         = false
chain_write   = false
```

### 9.3 Three-layer intersection

A Block may run only when all three layers permit:

```
Block declaration  ∩  Graph allow-list  ∩  Space grant  =  effective capabilities
```

Missing at any layer = denied. The system fails closed.

### 9.4 CLI disclosure on install

```
$ roko block install @my-org/markdown-classify
Block: @my-org/markdown-classify@1.2.3
Capabilities required:
  - Llm                   (any provider)
  - FsRead                (any path)
  - Net                   (api.openai.com, api.perplexity.ai)
This Space currently grants: Llm, FsRead, Net (api.* allowed).
All capabilities granted. Continue? [Y/n]
```

---

## 10. Plugin Discovery and Loading

At Space open, the engine:

1. Loads `<workspace>/.roko/plugins/manifest.toml` if present.
2. Loads `~/.roko/plugins/manifest.toml` if present.
3. Loads built-in plugins from `<roko-install>/builtin/plugins/`.
4. Registers every Block / Trigger / Graph / Rack declared by any plugin manifest.

```toml
# manifest.toml
schema_version = 1

[[plugin]]
name      = "@my-org/markdown-classify"
version   = "1.2.3"
kind      = "block"
declares  = ".roko/blocks/markdown-classify.toml"
checksum  = "blake3:..."
installed_at = "2026-04-20T12:00:00Z"

[[plugin]]
name      = "@my-org/perplexity-search"
version   = "0.5.1"
kind      = "block"
declares  = ".roko/blocks/perplexity-search.toml"
checksum  = "blake3:..."
```

Plugins may declare multiple Blocks / Graphs / Triggers in a single manifest entry by listing additional `declares`.

---

## 11. Validation

At load time the engine runs:

1. **TOML parsing** — strict mode; unknown keys error.
2. **Schema validation** — JSON-schema validation of input/output TypeSchema declarations.
3. **Reference resolution** — every `block = "..."` reference resolved against the registry; unresolved references error.
4. **Capability check** — for every Block, check Space grants cover declared capabilities; missing capabilities prompt user grant or error.
5. **Type checking** — every edge's source output -> target input compatibility verified; missing adapters error.
6. **Cycle detection** — Graph cycles outside `loop` nodes error.
7. **Slot completeness** — every required Slot has a filling (default or user-provided).
8. **Macro coverage** — every Macro binding points to a real node + param.
9. **Cost estimation** — sum of estimated costs vs. budget; warns if estimate > budget.
10. **Trigger validation** — source-kind specifics validated (cron syntax, regex compilability, webhook auth presence).

`roko graph validate <name>` runs these checks ahead of execution. The dashboard validation panel shows the same checks with green/yellow/red indicators.

---

## 12. Versioning

Semver everywhere. Graphs, Blocks, Triggers, Racks all carry `version`. References use semver requirements.

```toml
block       = "markdown-classify"
version_req = "^1.0"          # >=1.0.0, <2.0.0
```

At Graph load, the engine resolves the highest version satisfying the requirement, pins the resolved version onto the Flow snapshot, and uses that for the duration of the run. Re-running the same Graph at the same `(graph@version, input)` resolves to the same Blocks. Reproducibility by default.

When a Graph is published to the marketplace, all Block references are pinned to exact versions (`=1.2.3`) automatically. Forks may relax pins.

---

## 13. Configuration Hierarchy

Three layers, deep-merged:

1. **Space**: `<workspace>/workspace.toml` — top precedence
2. **User**: `~/.roko/config.toml` — middle
3. **Built-in defaults** — bottom

CLI flags override config. Environment variables (`ROKO_*`) override config but are overridden by flags.

User-level `~/.roko/config.toml`:

```toml
[ui]
default_view  = "tui"           # "cli" | "tui" | "dashboard"
color         = "auto"
density       = "comfortable"   # "compact" | "comfortable" | "spacious"

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

## 14. Acceptance Criteria

| Criterion | Verification |
|---|---|
| Graph TOML round-trips through parser -> model -> serialization with identity preserved | Unit test on a representative Graph |
| Schema validation rejects malformed input/output at load time, not run time | Negative test cases |
| Reference resolution finds Blocks across workspace > user > builtin in order | Integration test with shadowing at each level |
| Capability intersection enforced at runtime; Block run rejected if capability denied | Block attempting `Net` without capability fails closed |
| Edge type-checking catches unwirable connections at validate time | Negative test: wire `String` to `i32` with no adapter |
| Loop bound enforced; engine errors before unbounded execution | Synthetic infinite-loop Block; Loop terminates after `max_iterations` |
| Slot defaults applied; required Slots without defaults error at load time | Two test cases per |
| Macro bindings: setting one Macro updates multiple internal node params | Multi-binding Macro test |
| WASM plugin loads, runs, and is sandboxed (no fs without capability) | WASM Block attempts file write without capability -> fails |
| Script plugin executes, JSON I/O round-trips, capabilities enforced | Script tries disallowed path -> fails |
