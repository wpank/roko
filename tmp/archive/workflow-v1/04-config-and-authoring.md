# PRD-04 — Configuration & Authoring

**Status**: Draft
**Author**: Will (architect) + Claude (synthesis)
**Date**: 2026-04-25
**Crate**: `roko-workflow-toml` (new) + extensions across `roko-workflow`, `roko-cli`
**Prerequisites**: PRD-00, PRD-02, PRD-03

---

## 0. Scope

This document defines:
- The TOML schema for Workflows, Modules, Triggers, and Profiles.
- The four authoring tiers (TOML-only / scripts / WASM / Rust) and how they coexist.
- The capability declaration model.
- Validation, versioning, and discovery rules.
- The plugin loading model.

Composition is always TOML. Implementations may be in any tier. The TOML is the lingua franca everything else reads, writes, and edits.

---

## 1. Where Things Live

```
<workspace>/
├── workspace.toml
└── .roko/
    ├── workflows/                # workflow definitions (composition TOML)
    │   ├── doc-ingest.toml
    │   ├── prd-draft.toml
    │   └── deploy.toml
    ├── modules/                  # module definitions for non-built-in modules
    │   ├── markdown-classify.toml
    │   ├── perplexity-search.toml
    │   └── citation-grounder.toml
    ├── triggers/
    │   └── ingest-on-new-doc.toml
    ├── profiles/                 # visual-gate2 profiles (also workflows)
    │   └── visual-quality.toml
    └── plugins/                  # WASM / scripts / native plugin manifests
        ├── my-org.markdown-classify-1.2.3.wasm
        └── manifest.toml

~/.roko/
├── workflows/                    # user-level workflows (used across workspaces)
├── modules/
├── triggers/
├── profiles/
└── plugins/

<roko-install>/
└── builtin/
    ├── workflows/                # ships with roko
    ├── modules/
    └── profiles/
```

Resolution order at load time: workspace > user > builtin. By name and semver requirement.

---

## 2. Module TOML Schema

A Module TOML declares a Module and points at its implementation.

```toml
# .roko/modules/markdown-classify.toml

[module]
name        = "markdown-classify"
version     = "1.0.0"
description = "Classifies markdown segments by intent (context, task, spec, reference)"
publisher   = "@wpank"
license     = "CC-BY-4.0"
tags        = ["doc", "ingest", "classify"]

[module.input]
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

[module.output]
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

[module.required_evidence]
kinds = []                              # this module needs no evidence

[module.capabilities]
required = ["llm"]                      # only LLM access; no fs / net / shell

[module.estimate_cost]
usd_per_segment    = 0.002
seconds_per_segment = 1.5

# Implementation tier — exactly one of: rust / wasm / script / composition
[module.impl]
tier   = "rust"
crate  = "roko-builtin-doc"
type   = "MarkdownClassifyModule"

# Or for WASM:
# [module.impl]
# tier      = "wasm"
# path      = ".roko/plugins/my-org.markdown-classify-1.2.3.wasm"
# checksum  = "blake3:abc..."
# memory_mb = 64

# Or for script:
# [module.impl]
# tier        = "script"
# interpreter = "python3"
# path        = ".roko/plugins/markdown-classify.py"
# stdin       = "json"                  # input on stdin as JSON
# stdout      = "json"                  # output on stdout as JSON

# Or for composition (a Module that is itself a small Workflow):
# [module.impl]
# tier     = "composition"
# workflow = ".roko/workflows/markdown-classify-internal.toml"
```

---

## 3. Workflow TOML Schema

A Workflow TOML declares the composition of Modules into a state graph, with macros, slots, and policy.

```toml
# .roko/workflows/doc-ingest.toml

[workflow]
name        = "doc-ingest"
version     = "1.0.0"
description = "Ingest a directory of markdown into PRDs, plans, and tasks"
publisher   = "@wpank"
license     = "CC-BY-4.0"
tags        = ["doc", "ingest", "authoring"]
forked_from = "@nunchi/doc-ingest@0.9.0"

[workflow.input]
schema = """
type: object
required: [source_dir]
properties:
  source_dir:    { type: string }
  incremental:   { type: boolean, default: true }
  new_files:     { type: array, items: { type: string } }
"""

[workflow.output]
schema = """
type: object
properties:
  created_prds:  { type: array }
  created_plans: { type: array }
  audit_report:  { type: string }
"""

# ──────────────────────────────────────────────────
# Macros — promoted parameters
# ──────────────────────────────────────────────────

[[workflow.macro]]
name        = "enable_audit"
label       = "Audit findings"
description = "Run an audit pass after synthesis"
kind        = "boolean"
default     = true
bindings    = [{ node = "audit", param = "enabled" }]

[[workflow.macro]]
name        = "enable_web_research"
label       = "Web research"
description = "Enrich PRDs with web search and citations"
kind        = "boolean"
default     = true
bindings    = [
  { node = "enrich", param = "enabled" },
  { node = "audit",  param = "check_citations" },
]

[[workflow.macro]]
name        = "max_refine_iterations"
kind        = "integer"
min         = 1
max         = 5
default     = 2
bindings    = [{ node = "refine-loop", param = "max_iterations" }]

[[workflow.macro]]
name        = "synthesizer_model"
kind        = "model-ref"
default     = "claude-opus-4-7"
bindings    = [
  { node = "synthesize-prd", param = "model" },
  { node = "refine",         param = "model" },
]

[[workflow.macro]]
name        = "cluster_granularity"
kind        = "enum"
variants    = ["auto", "section", "file"]
default     = "auto"
bindings    = [{ node = "cluster", param = "granularity" }]

[[workflow.macro]]
name        = "budget_usd"
kind        = "money"
currency    = "USD"
max         = 50.0
default     = 5.0
bindings    = [{ workflow = ".policy.budget_usd" }]

# ──────────────────────────────────────────────────
# Slots — typed empty positions
# ──────────────────────────────────────────────────

[[workflow.slot]]
name           = "researcher"
label          = "Web Researcher"
description    = "Module that performs web search; defaults to Perplexity"
accepts        = "any-module-with-tag"
tag            = "web-research"
input_schema   = { type = "object", required = ["query"] }
output_schema  = { type = "object", required = ["citations", "summary"] }
required       = false
default_filling = { module = "perplexity-search", version = "^1" }

# ──────────────────────────────────────────────────
# State graph
# ──────────────────────────────────────────────────

[[workflow.node]]
id     = "walk"
kind   = "module"
module = "fs-walk"
[workflow.node.params]
patterns = ["**/*.md"]
ignore   = ["**/.git/**"]

[[workflow.node]]
id     = "classify"
kind   = "module"
module = "markdown-classify"

[[workflow.node]]
id     = "cluster"
kind   = "module"
module = "doc-cluster"
[workflow.node.params]
granularity = "auto"

[[workflow.node]]
id     = "synthesize-prd"
kind   = "module"
module = "prd-synthesize"
[workflow.node.params]
model       = "claude-opus-4-7"
role        = "strategist"

[[workflow.node]]
id     = "enrich"
kind   = "module"
module = "{{ slot.researcher }}"
condition = "macros.enable_web_research == true"

[[workflow.node]]
id     = "audit"
kind   = "module"
module = "prd-audit"

[[workflow.node]]
id     = "refine-loop"
kind   = "loop"
body   = "synthesize-prd"
until  = "audit.findings.severity_max < 'high'"
max_iterations = 2

[[workflow.node]]
id     = "plan"
kind   = "module"
module = "prd-plan"

[[workflow.node]]
id     = "persist"
kind   = "module"
module = "artifact-persist"

# ──────────────────────────────────────────────────
# Edges
# ──────────────────────────────────────────────────

[[workflow.edge]]
from = "walk"
to   = "classify"
[[workflow.edge.maps]]
from = "files"
to   = "segments"

[[workflow.edge]]
from = "classify"
to   = "cluster"

[[workflow.edge]]
from = "cluster"
to   = "synthesize-prd"

[[workflow.edge]]
from      = "synthesize-prd"
to        = "enrich"
condition = "macros.enable_web_research == true"

[[workflow.edge]]
from      = "synthesize-prd"
to        = "audit"
condition = "macros.enable_web_research == false"

[[workflow.edge]]
from = "enrich"
to   = "audit"

[[workflow.edge]]
from      = "audit"
to        = "refine-loop"
condition = "audit.findings.severity_max >= 'high' AND macros.enable_audit == true"

[[workflow.edge]]
from = "refine-loop"
to   = "plan"

[[workflow.edge]]
from      = "audit"
to        = "plan"
condition = "audit.findings.severity_max < 'high' OR macros.enable_audit == false"

[[workflow.edge]]
from = "plan"
to   = "persist"

# ──────────────────────────────────────────────────
# Policy
# ──────────────────────────────────────────────────

[workflow.policy]
budget_usd        = 5.0
deadline_seconds  = 1800
on_module_failure = "retry-with-escalation"   # see PRD-05
max_retries       = 2
human_input_default = "human"
```

---

## 4. Trigger TOML Schema

Specified in PRD-03 §2. Trigger TOMLs live in `<workspace>/.roko/triggers/` and `~/.roko/triggers/`.

---

## 5. Capability Declaration

Capabilities are declared on Modules and Triggers, granted at the Workspace, and intersected at runtime.

```toml
# Module declaring capabilities

[module.capabilities]
required = [
  "fs.read",
  "llm",
  { "net" = { domains = ["api.perplexity.ai", "arxiv.org"] } },
]
```

```toml
# Workspace granting capabilities (workspace.toml)

[workspace.capabilities]
fs.read       = true
fs.write      = true
net           = { domains = ["*"] }
llm           = true
shell         = false
chain.write   = false
```

Granular capability declarations:

```toml
[module.capabilities]
required = [
  { "fs.read"   = { paths = ["docs/**", "src/**"] } },
  { "fs.write"  = { paths = [".roko/artifacts/**"] } },
  { "shell"     = { commands = ["cargo", "rustc", "git"] } },
  { "net"       = { domains = ["api.openai.com", "api.anthropic.com"] } },
  { "secrets"   = { keys = ["openai_key", "anthropic_key"] } },
]
```

The CLI surfaces an aggregated capability summary at install time:

```
$ roko module install @my-org/markdown-classify
Module: @my-org/markdown-classify@1.2.3
Capabilities required:
  - llm                    (any provider)
  - fs.read                (any path)
  - net                    (api.openai.com, api.perplexity.ai)
This workspace currently grants: llm, fs.read, net (api.* allowed).
✓ All capabilities granted. Continue? [Y/n]
```

---

## 6. Authoring Tiers

### 6.1 Tier 1: Pure TOML (composition)

A Module whose implementation is a Workflow of other Modules. Lowest friction, no execution sandboxing needed.

```toml
[module.impl]
tier     = "composition"
workflow = ".roko/workflows/citation-and-grounding.toml"
```

Use case: combining built-in primitives without writing code. The visual editor (PRD-11) emits this tier exclusively when the user wires Modules together.

### 6.2 Tier 2: Scripts

Bash, Python, Node, or any interpreter. Module input is delivered on stdin (JSON); output expected on stdout (JSON); stderr is captured and surfaced as logs.

```toml
[module.impl]
tier        = "script"
interpreter = "python3"
path        = ".roko/plugins/markdown-classify.py"
stdin       = "json"
stdout      = "json"
timeout_seconds = 60

[module.impl.env]
PYTHONUNBUFFERED = "1"
```

Capability sandboxing for scripts:
- `fs.read` / `fs.write` restricted via path patterns (the engine prepares a temp directory, exposes only declared paths).
- `net` restricted via a HTTP proxy that allowlists declared domains.
- `shell` not transitively granted — scripts cannot spawn subprocesses unless `shell` is explicitly declared.

### 6.3 Tier 3: WASM

WebAssembly modules using a roko-defined ABI (`wit-bindgen` interfaces). Sandboxed by default; capability-gated.

```toml
[module.impl]
tier      = "wasm"
path      = ".roko/plugins/markdown-classify-1.2.3.wasm"
checksum  = "blake3:abc123..."
memory_mb = 64
fuel      = 100_000_000              # WASM execution-fuel cap
```

WASM is the recommended tier for marketplace artifacts: deterministic builds, sandboxing for free, capability declarations enforced by the host.

### 6.4 Tier 4: Native Rust

Compiled into roko or into a plugin crate dynamically loaded at startup. Highest performance, no sandboxing.

```toml
[module.impl]
tier  = "rust"
crate = "roko-builtin-doc"
type  = "MarkdownClassifyModule"
```

Used by built-in Modules and by trusted in-tree plugins. Marketplace artifacts may NOT use this tier directly; they must compile to WASM.

### 6.5 Tier-Selection Guidance

| Use case | Tier | Why |
|---|---|---|
| In-tree built-in (compile, lint, test gates) | Rust | Performance, full access |
| User wires existing modules into a new module | Composition | No execution risk |
| Glue around existing CLI tools | Script | Easy authoring, sandboxed by capabilities |
| Marketplace artifact, possibly untrusted | WASM | Sandboxed, deterministic, portable |
| Performance-sensitive analytics | Rust (in-tree plugin) | No sandbox overhead |

The visual editor only writes Composition. Other tiers are written by hand or by build tools.

---

## 7. Plugin Discovery & Loading

At workspace open, the engine:

1. Loads `<workspace>/.roko/plugins/manifest.toml` if present.
2. Loads `~/.roko/plugins/manifest.toml` if present.
3. Loads built-in plugins from `<roko-install>/builtin/plugins/`.
4. Registers every Module / Trigger / Workflow / Profile declared by any plugin manifest.

`manifest.toml`:

```toml
schema_version = 1

[[plugin]]
name      = "@my-org/markdown-classify"
version   = "1.2.3"
kind      = "module"
declares  = ".roko/modules/markdown-classify.toml"
checksum  = "blake3:..."
installed_at = "2026-04-20T12:00:00Z"

[[plugin]]
name      = "@my-org/perplexity-search"
version   = "0.5.1"
kind      = "module"
declares  = ".roko/modules/perplexity-search.toml"
checksum  = "blake3:..."
```

Plugins may declare multiple Modules / Workflows / Triggers in a single manifest entry by listing additional `declares`.

---

## 8. Validation

At load time the engine runs:

1. **TOML parsing** — strict; unknown keys error.
2. **Schema validation** — JSON-schema validation of input/output declarations.
3. **Reference resolution** — every `module = "..."` reference resolved against the registry; unresolved references error.
4. **Capability check** — for every Module, check workspace grants cover declared capabilities; missing capabilities prompt user grant or error.
5. **Type checking** — every edge's source.output → target.input compatibility verified; missing adapters error.
6. **Cycle detection** — state graph cycles outside `Loop` nodes error.
7. **Slot completeness** — every required slot has a filling (default or user-provided).
8. **Macro coverage** — every macro binding points to a real node + param.
9. **Cost estimation** — sum of estimated costs vs. budget; warns if estimate > budget.
10. **Trigger validation** — for trigger TOMLs, source-kind specifics validated (cron syntax, regex compilability, webhook auth presence).

`roko workflow validate <name>` runs these checks ahead of execution; the dashboard validation panel shows the same checks with green/yellow/red indicators.

---

## 9. Versioning

Semver everywhere. Workflows, Modules, Triggers all carry `version`. References use `version_req` (semver requirement).

```toml
module      = "markdown-classify"
version_req = "^1.0"          # ≥1.0.0, <2.0.0
```

At workflow load, the engine resolves the highest version satisfying the requirement, pins the resolved version onto the run snapshot, and uses that for the duration of the run. This gives reproducibility: re-running the same Workflow at the same `(workflow@version, input)` resolves to the same Modules.

When a Workflow is published to the marketplace, all Module references are pinned to exact versions (`=1.2.3`) automatically. Forks may relax pins.

---

## 10. CLI Surface

```
roko workflow list
roko workflow show <name>
roko workflow validate <name>
roko workflow new <name> [--template <name>]      # scaffold
roko workflow edit <name>                         # opens $EDITOR on TOML
roko workflow fork <source> <new-name>            # local fork
roko workflow remove <name>

roko module list
roko module show <name>
roko module install <ref>                         # marketplace install
roko module remove <name>
roko module new <name> [--tier <rust|wasm|script|composition>]

roko profile list                                 # visual-gate2 profiles
roko profile show <name>
roko profile new <name>
```

---

## 11. Acceptance Criteria

| Criterion | Verification |
|---|---|
| Workflow TOML round-trips through parser → in-memory model → serialization with byte-identity preserved (modulo whitespace). | Unit test on a representative workflow. |
| Schema validation rejects malformed input/output schemas at load time, not run time. | Negative test cases. |
| Reference resolution finds Modules across workspace > user > builtin in that order. | Integration test with a shadowing module at each level. |
| Capability intersection enforced at runtime; module run rejected if capability is denied. | Module attempting `net` without capability fails closed. |
| Edge type-checking catches unwirable connections at workflow-validate time. | Negative test: wire `String` to `i32` with no adapter → validation error. |
| Loop bound enforced; engine errors before unbounded execution. | Synthetic infinite-loop module; `Loop` node terminates after `max_iterations`. |
| Slot defaults applied when no user filling provided; required slots without defaults error at load time. | Two test cases per. |
| Macro bindings: setting one macro updates multiple internal node params consistently. | Multi-binding macro test. |
| WASM plugin loads, runs, and is sandboxed (no fs access without capability). | WASM module attempts file write without capability → fails. |
| Script plugin executes, JSON I/O round-trips, capabilities enforced via wrapping shell. | Script tries to access disallowed path → fails. |

---

## 12. Open Questions

- Should we adopt `wit-bindgen` directly for the WASM ABI, or define a roko-specific JSON-based interface? Leaning `wit-bindgen` for type safety.
- Should we allow runtime composition (build a Workflow from another Workflow at run time)? This is powerful (meta-programming) but complicates static validation. Defer to v2.
- Should there be a "module collections" concept (a manifest declaring N related modules together) or do plugin manifests already cover this? Probably the latter.
- TOML alternatives: support YAML or JSON for workflow definitions? Not in v1; one canonical format reduces tool complexity.
