# Roko Configuration & Plan Format

## Main Config: `roko.toml`

```toml
[agent]
command = "claude"
default_model = "glm-5.1"
default_backend = "zhipu"
default_effort = "medium"
temperament = "balanced"
context_limit_k = 200
bare_mode = true
mode = "ephemeral"

[agent.roles]
# Per-role overrides (model, effort, tools, etc.)

# --- 7 Providers configured ---

[providers.anthropic]
kind = "claude_cli"
command = "claude"
default_model = "claude-sonnet-4-6"
models = ["claude-haiku-4-5", "claude-sonnet-4-6", "claude-opus-4-6"]

[providers.openai]
kind = "openai_compat"
api_key_env = "OPENAI_API_KEY"
models = ["gpt-4.1", "gpt-4.1-mini", "o3", "o4-mini", "codex-mini-latest"]

[providers.perplexity]
kind = "perplexity_api"
api_key_env = "PERPLEXITY_API_KEY"
default_model = "sonar-deep-research"

[providers.moonshot]
kind = "openai_compat"
# Kimi models

[providers.zhipu]
kind = "openai_compat"
# GLM models

[providers.gemini]
kind = "openai_compat"
# Gemini models via OpenAI-compat

[providers.ollama]
kind = "openai_compat"
# Local models

# --- 18 Model aliases ---

[models.haiku]
provider = "anthropic"
slug = "claude-haiku-4-5"

[models.sonnet]
provider = "anthropic"
slug = "claude-sonnet-4-6"

[models.opus]
provider = "anthropic"
slug = "claude-opus-4-6"

# ... 15 more aliases

# --- Routing ---

[routing]
mode = "auto_override"
algorithm = "linucb"
fast_task_model = "claude-haiku-4-5"
standard_task_model = "claude-sonnet-4-6"
complex_task_model = "claude-opus-4-6"

[routing.weights]
quality = 0.5
cost = 0.3
latency = 0.2

# --- Pipeline tiers (complexity-dependent) ---

[pipeline.mechanical]
strategist = false
reviewers = false
max_iterations = 1

[pipeline.focused]
strategist = false
reviewers = false
max_iterations = 2

[pipeline.integrative]
strategist = true
reviewers = true
reviewer_mode = "quick"

[pipeline.architectural]
strategist = true
reviewers = true
reviewer_mode = "full"
max_iterations = 3

# --- Conductor ---

[conductor]
max_agents = 8
max_parallel_plans = 1
parallel_enabled = false
warm_implementers_per_plan = 1

[conductor.enabled_roles]
architect = true
auditor = true
scribe = true
critic = true

# --- Budget ---

[budget]
max_plan_usd = 25.0
max_turn_usd = 3.0
prompt_token_budget = 10000

# --- Learning ---

[learning]
replan_on_gate_failure = true
replan_max_per_plan = 2
replan_gate_attempts = 3
auto_playbook_refresh = true
knowledge_file_intel = true
```

## tasks.toml Format

```toml
[meta]
plan = "09-chain-layer"
iteration = 1
total = 5
done = 0
status = "pending"

[[task]]
id = "T1"
title = "Implement chain witness primitives"
description = "..."
role = "implementer"                    # Maps to AgentRole
status = "pending"                      # pending | done
tier = "integrative"                    # mechanical | focused | integrative | architectural
model_hint = "opus"                     # Explicit model override
frequency = "medium"                    # OperatingFrequency
replan_strategy = "decompose"           # ReplanStrategy
max_loc = 500                           # Max lines of change
files = ["crates/roko-chain/src/witness.rs"]
allowed_tools = ["Read", "Write", "Bash"]
denied_tools = ["git push"]
mcp_servers = ["roko-mcp-code"]
depends_on = []                         # Intra-plan: ["T1"]
depends_on_plan = ["08-safety-layer"]   # Cross-plan deps
split_into = ["T1.1", "T1.2"]          # Decomposition subtasks
timeout_secs = 600
max_retries = 2

[task.context]                          # Surgical context specification
files = ["crates/roko-chain/src/lib.rs"]
symbols = ["ChainWitness", "AnchorPoint"]
search_patterns = ["fn anchor"]

[[task.verify]]                         # Per-task verification pipeline
command = "cargo check -p roko-chain"
expect_exit = 0

[[task.verify]]
command = "cargo test -p roko-chain -- witness"
expect_exit = 0
```

## Key Differences from Mori tasks.toml

| Field | Mori | Roko |
|---|---|---|
| `role` | Not in tasks.toml (hardcoded pipeline) | Explicit per-task |
| `tier` | `complexity_band` (fast/standard/complex) | tier (mechanical/focused/integrative/architectural) |
| `model_hint` | `preferred_model` | `model_hint` |
| `category` | scaffolding/implementation/etc. | Not present (tier serves same purpose) |
| `reasoning_level` | low/medium/high | Not present |
| `speed_priority` | latency/balanced/accuracy | Not present |
| `quality_profile` | pragmatic/balanced/hardened | Not present |
| `context_weight` | slim/standard/deep | Not present |
| `parallel_group` | A/B/C groups | Uses `depends_on` DAG instead |
| `exclusive_files` | boolean | Uses DAG `infer_file_overlap` config |
| `verify` | Not per-task | Per-task verification pipeline |
| `context` | `context_files` list | Structured `TaskContext` with files, symbols, search_patterns |
| `split_into` | Not present | Decomposition subtasks |
| `replan_strategy` | Not present | decompose/retry/escalate |
| `mcp_servers` | Not per-task | Per-task MCP server list |
| `allowed_tools` / `denied_tools` | Not per-task | Per-task tool scoping |

## Plan Directory Structure

Roko plans are simpler than mori:

```
plans/
  09-chain-layer/
    plan.md          - Plan spec with YAML frontmatter
    tasks.toml       - Task checklist
```

vs mori's 15+ files per plan directory. Roko generates artifacts dynamically rather than pre-generating them.
