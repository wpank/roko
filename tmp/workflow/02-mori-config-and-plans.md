# Mori Configuration & Plan Format

## Top-Level Config: `.mori/config.toml`

```toml
codex_default_model = "claude-sonnet-4-6"
context_limit_k = 150
default_effort = "Medium"
auto_advance_batch = true
auto_merge_on_complete = true
architect_enabled = true
auditor_enabled = true
scribe_enabled = true
critic_enabled = true
skip_tests = false
max_iterations = 5
max_agents = 20
max_parallel_plans = 5
parallel_enabled = true
express_mode = true
warm_implementers_per_plan = 1

# 3-tier model routing by complexity
fast_task_model = "claude-haiku-4-5-20251001"
standard_task_model = "claude-sonnet-4-6"
complex_task_model = "claude-opus-4-6"
fast_task_provider = "claude"
standard_task_provider = "claude"
complex_task_provider = "claude"

# Per-role model overrides
[role_models]
architect = "gpt-5.4-mini"
implementer = "gpt-5.4-mini"
conductor = "gpt-5.4-mini"
# ... any of the 28 roles

[role_effort]
# Per-role effort overrides

[role_context_k]
# Per-role context limit overrides

[plan_overrides]
# Per-plan config overrides
```

## Queue / Milestone System: `.mori/queue.toml`

Plans are organized into milestones for execution ordering:

```toml
[run]
mode = "express"
max_agents = 15
max_parallel_plans = 5
optimization_profile = "balanced"
context_strategy = "hybrid"
routing_mode = "auto_override"
warm_implementers_per_plan = 2

[[milestone]]
name = "Audit Remediation"
description = "Re-implement plans that failed structural verification audit."
tags = ["audit", "remediation", "critical"]
plans = ["01", "10", "11", "12", ...]

[[milestone]]
name = "Runnable Golem"
description = "Critical path to bardo-golem doing something real."
tags = ["mvp", "golem", "critical-path"]
plans = ["02", "03", "09", "12", "13a", "10", "11", ...]
```

7 milestones total, organizing 171 plans into priority groups.

## MCP Config: `.mori/mcp-config.json`

```json
{
  "mcpServers": {
    "mori": {
      "command": "mori-mcp",
      "args": ["context-server", "--root", "."]
    }
  }
}
```

Provides code intelligence tools: `search_code`, `get_symbol_context`, `get_file_ast`, `find_similar_patterns`.

## Plan File Structure

Each plan lives in `.mori/plans/<num>-<slug>/` with these artifacts:

```
.mori/plans/06-terminal-navigation/
  plan.md                   - Full implementation specification
  brief.md                  - Strategist-generated implementation brief
  tasks.toml                - Implementation task checklist
  review-tasks.toml         - Review task checklist (Architect+Auditor)
  verify-tasks.toml         - Verification task checklist
  scribe-tasks.toml         - Documentation task checklist
  rubric.md                 - Shared review rubric
  decomposition.md          - Step-by-step breakdown
  prd-extract.md            - PRD2 excerpt for this plan
  research.md               - Research artifacts
  integration.md            - Integration memo
  dependency-manifest.toml  - Dependency manifest
  fixture-manifest.toml     - Fixture manifest
  testing-backlog.md        - Testing backlog
  reviews/                  - Review output directory
```

## Plan Frontmatter (YAML in plan.md)

```yaml
---
plan: "06-terminal-navigation"
depends_on: ["05-terminal-widgets"]
parallel_with: []
crates_touched: ["bardo-terminal"]
estimated_tasks: 24
estimated_minutes: 30
refactor_after: false
parallel_safe: true
---
```

## tasks.toml Format (Implementation Tasks)

```toml
[meta]
plan = "06-terminal-navigation"
iteration = 2
total = 24
done = 0
status = "pending"
max_parallel = 4
estimated_total_minutes = 90

[[task]]
id = "T0.1"
title = "Create src/lib.rs with module tree"
status = "pending"                      # pending | active | done
files = ["apps/bardo-terminal/src/lib.rs"]
acceptance = [
    "File exists: apps/bardo-terminal/src/lib.rs",
    "`cargo check -p bardo-terminal` exits with code 0",
]
depends_on = []                         # Intra-plan: ["T1"], Cross-plan: ["09:T3"]
parallel_group = "A"                    # Tasks in same group run concurrently
exclusive_files = true                  # No other concurrent task touches these files
context_files = [...]                   # Extra files to inject into prompt

# --- Model routing metadata ---
category = "scaffolding"                # scaffolding | implementation | integration | verification | research | refactor | infra | docs
reasoning_level = "low"                 # low | medium | high
speed_priority = "latency"             # latency | balanced | accuracy
quality_profile = "pragmatic"           # pragmatic | balanced | hardened
context_weight = "slim"                 # slim | standard | deep
complexity_band = "fast"                # fast | standard | complex (determines model tier)
tags = ["crate:golem-core", "plan:02"]
escalate_on_retry = true                # Use higher-tier model on retry
```

## review-tasks.toml Format (Reviewer Tasks)

```toml
[meta]
plan = "02-core-types"
role = "reviewer"
review_type = "architect+auditor"

[[task]]
id = "R1"
title = "Gompertz Hazard Monotonicity and Rate Bounds"
type = "invariant"                      # invariant | contract | acceptance | gate
severity = "blocking"                   # blocking | major
check = [
    "Hazard h(t) monotonically increases with age when epistemic_fitness constant",
    "test_hazard_monotonic_with_age: h(t1) < h(t2) for t1 < t2",
]
files = ["crates/golem-mortality/src/stochastic.rs"]
verdict = "pending"
notes = ""
```

## scribe-tasks.toml Format (Documentation Tasks)

```toml
[meta]
plan = "02-core-types"
role = "scribe"
crate = "golem-core"

# Module documentation
[[task]]
id = "D1"
type = "module_doc"
title = "Document GolemId: UUID wrapper and identity system"
output_file = "plans/context/docs/01-golemid-system.md"
sections = ["context", "architecture", "api", "examples", "testing"]
modules = ["src/id.rs"]

# Citation documentation
[[task]]
id = "C1"
type = "citation"
title = "Document [ADAMS-2024] and its implementation"
citation_key = "[ADAMS-2024]"
where_used = ["epistemic fitness calculation"]
code_locations = ["src/cognitive.rs epistemic_fitness"]

# Formula documentation
[[task]]
id = "F1"
type = "formula"
title = "Document Gompertz-Makeham hazard function"
formula = "h(t) = lambda + alpha * exp(beta * t) * epsilon(t)"
academic_origin = "[CAMPBELL-2025]"
code_function = "golem_mortality::stochastic::compute_hazard_rate()"
invariants = ["INV-001", "INV-002"]

# Diagram tasks
[[task]]
id = "M1"
type = "diagram"
diagram_type = "classDiagram"
concept = "32 atomic f32/u8/u32 signals, 256 bytes, 4 cache lines"

# Interaction diagram
[[task]]
id = "X1"
type = "interaction"
diagram_type = "flowchart"
components = ["src/id.rs", "CorticalState", "Event"]
```

## dependency-manifest.toml

```toml
[[dependency]]
name = "crate:golem-core"
kind = "crate"
required_for = ["implementation", "integration"]
downstream_plan_refs = ["01-workspace-scaffold", "14b-cognitive-mechanisms"]

[[dependency]]
name = "mirage-evm"
kind = "service"
required_for = ["integration", "verification"]
mock_strategy = "mirage-sidecar"
fixture_keys = ["mirage-evm"]
```

## fixture-manifest.toml

```toml
[[fixture]]
key = "mirage-evm"
kind = "mirage-evm"
entrypoint = "cargo run -p mirage-rs -- --host 127.0.0.1 --port 8545"
reusable = true
healthcheck = "TCP connect 127.0.0.1:8545 succeeds"
```

## OrchestratorConfig (Rust struct, runtime)

```rust
pub struct OrchestratorConfig {
    pub repo_root: PathBuf,
    pub plans_dir: PathBuf,
    pub no_review: bool,
    pub skip_tests: bool,
    pub max_iterations: u32,
    pub batch_size: Option<usize>,
    pub model: Option<String>,
    pub no_docs: bool,
    pub max_parallel_plans: usize,    // default 3, max 4
    pub wave_error_policy: WaveErrorPolicy,
    pub parallel: bool,
    pub pre_plan: bool,
    pub refactor_interval: usize,     // run refactorer every N plans
}
```

## Wave Error Policy

```rust
pub enum WaveErrorPolicy {
    HaltWave,         // Stop everything on first failure
    ContinueWave,     // Keep running other plans; merge successes (DEFAULT)
    SkipAndContinue,  // Skip failed plan and continue
}
```

## Brief Authority Chain

Every generated brief declares:
```
## Authority Chain (what wins when files conflict)
Quick Reference > TOML acceptance criteria > Plan prose > PRD > Brief/Decomposition.
The brief is orientation. It does not override the plan or TOML.
```
