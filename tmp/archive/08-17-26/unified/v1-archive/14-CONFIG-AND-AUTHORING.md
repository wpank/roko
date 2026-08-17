# 14 — Config and Authoring

> TOML is the lingua franca. Every Graph, Rack, Trigger, Space, and Agent is declared in TOML. The 5-tier SPI enables contributions from pure Markdown prompts to compiled Rust, with progressive capability and progressive isolation.

**Source**: wf-04 (Configuration & Authoring), updated to unified vocabulary. Major additions: 5-tier package SPI, domain profiles as complete cognitive postures, workspace scoping.

---

## 1. Where Things Live

```
<workspace>/
+-- workspace.toml                    # Space config: capabilities, models, deploy targets
+-- .roko/
    +-- graphs/                       # Graph definitions (TOML)
    |   +-- doc-ingest.toml
    |   +-- prd-draft.toml
    |   +-- deploy.toml
    +-- blocks/                       # Block definitions (non-built-in)
    |   +-- markdown-classify.toml
    |   +-- perplexity-search.toml
    |   +-- citation-grounder.toml
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
+-- graphs/                           # User-level (across Spaces)
+-- blocks/
+-- triggers/
+-- racks/
+-- prompts/
+-- profiles/
+-- tools/
+-- plugins/

<roko-install>/
+-- builtin/
    +-- graphs/                       # Ships with roko
    +-- blocks/
    +-- racks/
    +-- prompts/
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

# Implementation tier -- exactly one of the 5 tiers
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

## 6. The 5-Tier Package SPI

The Service Provider Interface defines five tiers for authoring Blocks, Extensions, Graphs, and agent capabilities. Each tier balances expressiveness against isolation. Progressive capability with progressive trust.

### Tier 1: Prompts (pure Markdown/TOML front-matter, no execution)

The lowest-friction tier. A prompt package is a Markdown file with TOML front-matter that declares metadata. No code executes — the content is injected into the system prompt or context assembly.

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
**Distribution**: Marketplace. Anyone can publish prompts.
**Use cases**: System prompt customization, review protocols, domain-specific instructions, research methodologies.

### Tier 2: Config Profiles (TOML bundles layering onto roko.toml)

A config profile is a TOML bundle that layers onto `roko.toml`, customizing agent behavior without writing code. Profiles configure existing capabilities — they do not add new ones.

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
**Use cases**: Domain specialization, team conventions, environment-specific tuning.

### Tier 3: Declarative Tools (TOML manifests for subprocess/HTTP/MCP, sandboxed)

A declarative tool is a TOML manifest that wraps a subprocess, HTTP endpoint, or MCP server as a Block. The manifest declares I/O schemas, capability requirements, and invocation details. The runtime handles sandboxing.

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

**Sandboxing**: OS-level process isolation. Subprocess inherits only declared capabilities. Network access restricted to declared domains. Filesystem access restricted to declared paths.
**Distribution**: Verified publishers. Declarative tools can invoke subprocesses, so they require publisher verification.
**Use cases**: Wrapping CLI tools, calling HTTP APIs, integrating MCP servers, building tool chains without writing code.

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

**Sandboxing**: WASM sandbox. Memory isolated. Fuel-metered (execution bounded). Capability-gated — the host only exposes declared capabilities via the ABI.
**Distribution**: Marketplace (recommended default). WASM modules are portable, deterministic, and safe to run.
**Use cases**: Custom Blocks, Extensions, scoring functions, analysis pipelines, anything that needs code execution.

### Tier 5: Native Rust (compiled, full trust, in-tree only)

Compiled Rust code implementing `impl Block for MyBlock` or `impl Extension for MyExt`. Highest performance, no sandboxing. Reserved for built-in components and trusted in-tree plugins.

```toml
[block.impl]
tier  = "rust"
crate = "roko-builtin-doc"
type  = "MarkdownClassifyBlock"
```

**Sandboxing**: Process-level only. Full trust.
**Distribution**: Compiled into the binary. Marketplace artifacts may NOT use this tier directly — they must compile to WASM.
**Use cases**: Built-in gates, routers, composers, core Extensions, performance-critical analytics.

### Tier Selection Guidance

| Use case | Tier | Why |
|---|---|---|
| System prompt customization | 1. Prompts | No execution risk, instant iteration |
| Domain specialization (clock, models, gates) | 2. Config | Configuration, not code |
| Wrapping existing CLI tools or APIs | 3. Declarative tools | No code needed, sandboxed |
| Marketplace artifact, custom logic | 4. WASM | Sandboxed, deterministic, portable |
| In-tree built-in (gates, routers, composers) | 5. Rust | Performance, full access |

The visual editor (see [doc-16](16-SURFACES.md)) only writes TOML (Tiers 1-3). WASM and Rust tiers require build tools.

---

## 7. Domain Profiles as Cognitive Postures

A domain profile is a **complete cognitive posture** — not just a string label like `"coding"`. It bundles clock configuration, extensions, wakeup events, context weights, gate configuration, and infrastructure settings into a coherent whole.

### Profile schema

```toml
# Full domain profile specification

[profile]
name        = "security-audit"
version     = "1.0.0"
description = "Complete cognitive posture for security auditing"

# -- Clock configuration (adaptive tick rates) -----------------
[profile.clock]
gamma_ms = 300            # perception tick
theta_ms = 5000           # inference tick
delta_ms = 300000         # reflection tick (5 min)
regime   = "normal"       # initial regime: Calm / Normal / Volatile / Crisis

# -- Extensions loaded for this profile -----------------------
[profile.extensions]
enabled  = ["safety", "vuln-scanner", "dependency-audit", "cost-tracker", "circuit-breaker", "camel-monitor"]
disabled = []

# -- Wakeup events (what triggers the Agent to start a tick) ---
[profile.wakeup]
events = [
  "signal:kind:Code",             # new code Signal in Store
  "pulse:gate.verdict.emitted",   # gate verdict on Bus
  "trigger:cron:0 */4 * * *",    # every 4 hours
  "trigger:webhook:/hooks/audit", # webhook
]

# -- Context weights for Compose protocol ---------------------
[profile.context_weights]
neuro     = 0.15           # knowledge store context
task      = 0.20           # task-specific context
research  = 0.30           # research findings
heuristic = 0.10           # learned heuristics
episode   = 0.10           # past episodes
pheromone = 0.05           # stigmergic signals
affect    = 0.05           # somatic markers
system    = 0.05           # system instructions

# -- Gate configuration (which gates run, thresholds) ----------
[profile.gates]
compile   = true
test      = true
clippy    = true
diff      = true
vuln_scan = { enabled = true, severity = "medium" }
llm_judge = { enabled = true, model = "claude-sonnet-4-6" }
consensus = { enabled = false }         # disabled for solo audits

# -- Infrastructure ------------------------------------------
[profile.infrastructure]
execution  = "in-process"              # or "isolated" for untrusted
budget_usd = 25.0
max_tokens = 200000
max_duration = "2h"

# -- Model routing -------------------------------------------
[profile.models]
primary   = "claude-opus-4-6"          # complex analysis
fallback  = "claude-sonnet-4-6"        # standard tasks
reflexive = "claude-haiku-4-5"         # quick checks
```

### Why "cognitive posture" matters

A `profile = "coding"` Agent and a `profile = "security-audit"` Agent differ in every dimension:

| Dimension | Coding | Security Audit |
|---|---|---|
| Clock | Fast gamma (200ms), moderate theta | Moderate gamma (300ms), slow theta (5s) |
| Extensions | git, compiler, test-runner | vuln-scanner, dependency-audit |
| Wakeup events | Code changes, PR events | Scheduled scans, vulnerability feeds |
| Context weights | High task (0.4), low research (0.1) | High research (0.3), moderate task (0.2) |
| Gates | compile, test, clippy | vuln_scan, llm_judge, diff |
| Infrastructure | ephemeral, $10 budget | persistent, $25 budget |

Setting `profile = "security-audit"` configures all of these simultaneously. The profile IS the Agent's cognitive posture — its complete behavioral configuration.

### Profile inheritance

Profiles can extend other profiles:

```toml
[profile]
name = "defi-security-audit"
base = "security-audit"           # inherits all settings

# Override specific dimensions
[profile.extensions]
enabled = ["chain-reader", "slither-analyzer"]   # added on top of base

[profile.gates]
vuln_scan = { severity = "low" }                 # stricter threshold
```

Deep merge: arrays are concatenated (Extensions), objects are merged (gates config), scalars are overridden (clock values).

---

## 8. Workspace Scoping

Roko supports multi-workspace operation. A single daemon can serve multiple workspaces, each with its own capability grants, knowledge scope, and resource limits.

### workspace.toml

```toml
# <workspace>/workspace.toml

[space]
name           = "nunchi-dashboard"
schema_version = 1
extends        = "~/.roko/templates/web-app"

# -- Capability grants ------------------------------------------------
[space.capabilities]
fs_read       = true
fs_write      = true
net           = { domains = ["*"] }
llm           = true
shell         = { commands = ["cargo", "git", "npm"] }
chain_write   = false
secrets       = { keys = ["anthropic_key", "openai_key", "railway_token"] }

# -- Model routing ----------------------------------------------------
[space.models]
strategist = "claude-opus-4-7"
researcher = "claude-sonnet-4-6"
scribe     = "claude-haiku-4-5"
default    = "claude-sonnet-4-6"

# -- Deploy targets ---------------------------------------------------
[[space.deploy]]
name    = "railway"
default = true
[space.deploy.params]
service = "nunchi-dashboard"

[[space.deploy]]
name = "fly-staging"
[space.deploy.params]
app = "nunchi-dashboard-staging"

# -- Knowledge scoping ------------------------------------------------
[space.knowledge]
scope       = "workspace"             # workspace | user | global
share_with  = ["tag:nunchi"]          # share knowledge with these Spaces
import_from = ["roko"]               # import knowledge from these Spaces

# -- Graph confirmation policy ----------------------------------------
[space.confirm]
"doc-ingest" = "never"
"deploy"     = "always"
"*"          = "interactive"
```

### Multi-workspace daemon

When `roko serve` runs as a daemon, it can manage multiple workspaces:

```toml
# ~/.roko/daemon.toml

[daemon]
port = 6677
workspaces = [
  { path = "/Users/will/dev/nunchi/roko/roko",   name = "roko" },
  { path = "/Users/will/dev/nunchi/dashboard",    name = "dashboard" },
  { path = "/Users/will/dev/nunchi/relay",        name = "relay" },
]

# Per-workspace resource limits
[daemon.limits]
max_agents_per_workspace = 20
max_total_agents = 50
max_budget_per_workspace_usd = 100.0
```

### Per-workspace capability grants

Each workspace has its own capability surface. An Agent in the `roko` workspace cannot access the `dashboard` workspace's filesystem unless the daemon grants cross-workspace access:

```toml
# workspace.toml in roko workspace
[space.capabilities]
fs_read  = true
fs_write = { paths = [".roko/**", "crates/**", "tmp/**"] }
shell    = { commands = ["cargo", "git"] }

# workspace.toml in dashboard workspace
[space.capabilities]
fs_read  = true
fs_write = { paths = [".roko/**", "src/**", "public/**"] }
shell    = { commands = ["npm", "git"] }
net      = { domains = ["api.vercel.com"] }
```

Three-layer capability intersection applies per workspace: Block declaration (intersection) Graph allow-list (intersection) Space grant = effective capabilities. See [doc-02](02-BLOCK.md) section 5 and [doc-17](17-SECURITY-MODEL.md).

### Cross-workspace knowledge sharing

Knowledge Signals (see [doc-11](11-MEMORY-AND-KNOWLEDGE.md)) are scoped to their workspace by default. Cross-workspace sharing is explicit:

```toml
# roko workspace shares coding heuristics
[space.knowledge]
share_with = ["tag:nunchi"]           # all nunchi workspaces receive
share_kinds = ["Heuristic", "Insight"] # only these Signal kinds

# dashboard workspace imports from roko
[space.knowledge]
import_from = ["roko"]
import_filter = { min_tier = "Consolidated" }  # only high-confidence Signals
```

Shared Signals carry their origin workspace tag in CaMeL provenance (see [doc-08](08-EXTENSION-SYSTEM.md) section 2 and [doc-17](17-SECURITY-MODEL.md)). The receiving workspace can query but not modify the original.

---

## 9. Agent Configuration

Agents are configured in `roko.toml` or in per-agent TOML files.

```toml
# roko.toml -- agent section

[[agent]]
name    = "coder"
profile = "coding"            # full cognitive posture (see section 7)
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

## 10. Capability Declarations

Capabilities are declared on Blocks and Triggers, granted at the Space, and intersected at runtime.

### 10.1 Block declarations

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

### 10.2 Space grants

```toml
[space.capabilities]
fs_read       = true
fs_write      = true
net           = { domains = ["*"] }
llm           = true
shell         = false
chain_write   = false
```

### 10.3 Three-layer intersection

A Block may run only when all three layers permit:

```
Block declaration  (intersection)  Graph allow-list  (intersection)  Space grant  =  effective capabilities
```

Missing at any layer = denied. The system fails closed. CaMeL IFC (see [doc-17](17-SECURITY-MODEL.md)) tags capability provenance through the execution chain. See [doc-08](08-EXTENSION-SYSTEM.md) section 2 for how Extensions propagate these tags.

### 10.4 CLI disclosure on install

```
$ roko block install @my-org/markdown-classify
Block: @my-org/markdown-classify@1.2.3
Tier: WASM (sandboxed, fuel-metered)
Capabilities required:
  - Llm                   (any provider)
  - FsRead                (any path)
  - Net                   (api.openai.com, api.perplexity.ai)
This Space currently grants: Llm, FsRead, Net (api.* allowed).
All capabilities granted. Continue? [Y/n]
```

---

## 11. Plugin Discovery and Loading

At Space open, the engine:

1. Loads `<workspace>/.roko/plugins/manifest.toml` if present.
2. Loads `~/.roko/plugins/manifest.toml` if present.
3. Loads built-in plugins from `<roko-install>/builtin/plugins/`.
4. Scans Tier 1-3 directories (`prompts/`, `profiles/`, `tools/`) for manifest files.
5. Registers every Block / Trigger / Graph / Rack / Prompt / Profile / Tool declared by any manifest.

```toml
# manifest.toml
schema_version = 1

[[plugin]]
name      = "@my-org/markdown-classify"
version   = "1.2.3"
kind      = "block"
tier      = "wasm"
declares  = ".roko/blocks/markdown-classify.toml"
checksum  = "blake3:..."
installed_at = "2026-04-20T12:00:00Z"

[[plugin]]
name      = "@my-org/perplexity-search"
version   = "0.5.1"
kind      = "block"
tier      = "declarative"
declares  = ".roko/tools/perplexity-search.toml"
checksum  = "blake3:..."
```

Plugins may declare multiple Blocks / Graphs / Triggers in a single manifest entry by listing additional `declares`.

---

## 12. Validation

At load time the engine runs:

1. **TOML parsing** — strict mode; unknown keys error.
2. **Schema validation** — JSON-schema validation of input/output TypeSchema declarations.
3. **Tier validation** — verify implementation tier matches declared tier, check sandboxing constraints.
4. **Reference resolution** — every `block = "..."` reference resolved against the registry; unresolved references error.
5. **Capability check** — for every Block, check Space grants cover declared capabilities; missing capabilities prompt user grant or error.
6. **Type checking** — every edge's source output -> target input compatibility verified; missing adapters error.
7. **Cycle detection** — Graph cycles outside `loop` nodes error.
8. **Slot completeness** — every required Slot has a filling (default or user-provided).
9. **Macro coverage** — every Macro binding points to a real node + param.
10. **Cost estimation** — sum of estimated costs vs. budget; warns if estimate > budget.
11. **Trigger validation** — source-kind specifics validated (cron syntax, regex compilability, webhook auth presence).
12. **CaMeL capability validation** — verify Extension capability declarations are consistent with CaMeL IFC rules (see [doc-17](17-SECURITY-MODEL.md)).

`roko graph validate <name>` runs these checks ahead of execution. The dashboard validation panel shows the same checks with green/yellow/red indicators.

---

## 13. Versioning

Semver everywhere. Graphs, Blocks, Triggers, Racks, Prompts, Profiles, and Tools all carry `version`. References use semver requirements.

```toml
block       = "markdown-classify"
version_req = "^1.0"          # >=1.0.0, <2.0.0
```

At Graph load, the engine resolves the highest version satisfying the requirement, pins the resolved version onto the Flow snapshot, and uses that for the duration of the run. Re-running the same Graph at the same `(graph@version, input)` resolves to the same Blocks. Reproducibility by default.

When a Graph is published to the marketplace, all Block references are pinned to exact versions (`=1.2.3`) automatically. Forks may relax pins.

---

## 14. Configuration Hierarchy

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

## 15. Acceptance Criteria

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
| Tier 1 prompt loads and injects into system prompt context | Load a .md prompt, verify it appears in composed context |
| Tier 2 config profile deep-merges over base profile | Profile with `base` extends parent, overrides scalar, concatenates arrays |
| Tier 3 declarative tool invokes subprocess with sandboxed capabilities | Tool tries disallowed path -> fails; allowed command -> succeeds |
| Tier 4 WASM plugin loads, runs, and is sandboxed (no fs without capability) | WASM Block attempts file write without capability -> fails |
| Tier 5 Rust Block compiles and runs with full access | Built-in Block executes normally |
| Domain profile configures all dimensions simultaneously | Set `profile = "security-audit"`, verify clock + extensions + gates + models all match |
| Profile inheritance: child overrides parent cleanly | Child profile with `base`, verify deep merge |
| Multi-workspace daemon isolates capabilities per workspace | Agent in workspace A cannot access workspace B filesystem |
| Cross-workspace knowledge sharing respects scope and filter | Share Heuristic from workspace A, verify it appears in workspace B with correct provenance tags |
| CaMeL capability tags validated at load time | Extension declaring capabilities beyond Space grants fails validation |
