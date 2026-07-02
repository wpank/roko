# How Roko Generates Plans From Documentation

## The loop: docs → plans → execution → verification

```
roko-progress docs / PRD docs / implementation-plans
        ↓
   Plan Generator (Claude agent, Strategist role)
        ↓
   plans/<num>-<slug>/
   ├── plan.md          (frontmatter + description)
   ├── tasks.toml       (task definitions with acceptance criteria)
   ├── brief.md         (context for agents)
   └── decomposition.md (how the plan was broken down)
        ↓
   roko plan run ./plans/
        ↓
   orchestrator reads plans → DAG → dispatch agents → gates → persist
```

## What already exists

| Component | Status |
|-----------|--------|
| Plan discovery (`roko-orchestrator::plan_discovery`) | ✅ Reads `plans/<num>/plan.md`, parses frontmatter |
| DAG construction (`roko-orchestrator::dag`) | ✅ Builds task dependency graph |
| Executor state machine (`roko-orchestrator::executor`) | ✅ 14-phase, emits SpawnAgent/RunGate actions |
| Orchestration harness (`roko-cli::orchestrate`) | ⚠ Exists but uses ExecAgent (wiring needed) |
| Plan generation | ❌ Does not exist yet |

## What a plan looks like (mori format, which roko reads)

### `plans/W01-wire-system-prompts/plan.md`

```markdown
---
plan: W01-wire-system-prompts
depends_on: []
parallel_with: []
crates_touched: [roko-cli, roko-compose]
estimated_tasks: 3
estimated_parallel_width: 1
---

# Wire SystemPromptBuilder into orchestrate.rs

Replace inline 1-sentence role prompts with the existing 6-layer
SystemPromptBuilder and 9 role templates from roko-compose.
```

### `plans/W01-wire-system-prompts/tasks.toml`

```toml
[meta]
plan = "W01-wire-system-prompts"
iteration = 1
total = 3
done = 0
status = "ready"
max_parallel = 1

[[task]]
id = "T1"
title = "Import SystemPromptBuilder in orchestrate.rs"
status = "ready"
files = [
    "crates/roko-cli/src/orchestrate.rs",
    "crates/roko-cli/Cargo.toml",
]
acceptance = [
    "grep SystemPromptBuilder crates/roko-cli/src/orchestrate.rs returns matches",
    "cargo check -p roko-cli passes",
]
depends_on = []

[[task]]
id = "T2"
title = "Replace role_system_prompt() with SystemPromptBuilder calls"
status = "ready"
files = [
    "crates/roko-cli/src/orchestrate.rs",
]
acceptance = [
    "role_system_prompt function is removed or calls SystemPromptBuilder",
    "each AgentRole variant produces a multi-section prompt (not 1 sentence)",
    "cargo test -p roko-cli passes",
]
depends_on = ["T1"]

[[task]]
id = "T3"
title = "Pass composed prompt to agent via --append-system-prompt"
status = "ready"
files = [
    "crates/roko-cli/src/orchestrate.rs",
]
acceptance = [
    "dispatch_agent() passes system prompt to the agent",
    "cargo test -p roko-cli passes",
]
depends_on = ["T2"]
```

## How to generate plans from roko-progress docs

### Step 1: Create a plan generator script

This is a Claude agent with the Strategist role. It reads a checklist section
and produces plan directories.

```bash
# Generate plans from implementation-plans (the focused wiring work)
roko run --role strategist \
  --prompt "Read /Users/will/dev/nunchi/roko/roko/tmp/implementation-plans/01-agent-wiring.md. \
    For each phase (A, B, C, D), generate a plan directory under ./plans/ with plan.md and \
    tasks.toml in the format shown in /Users/will/dev/uniswap/bardo/.mori/plans/71-mvp-gate/tasks.toml. \
    Each checklist item becomes a task. Set depends_on based on the ordering. \
    Set acceptance criteria that can be verified by grep or cargo test."
```

### Step 2: Or generate from MORI-PARITY-CHECKLIST sections

```bash
# For a specific section of the parity checklist
roko run --role strategist \
  --prompt "Read section §7.1 (Claude backend) from \
    /Users/will/dev/nunchi/roko/bardo-backup/tmp/roko-progress/MORI-PARITY-CHECKLIST.md. \
    Generate a plan directory ./plans/P07-claude-backend/ with plan.md and tasks.toml. \
    Each [ ] item becomes a task. Each [x] item is already done (skip). \
    Cross-reference with /Users/will/dev/nunchi/roko/roko/tmp/implementation-plans/01-agent-wiring.md \
    for the implementation approach."
```

### Step 3: Batch-generate all plans

```bash
# Generate plans for all P0 implementation-plans
for plan in 01 02 03 04; do
  roko run --role strategist \
    --prompt "Read /Users/will/dev/nunchi/roko/roko/tmp/implementation-plans/${plan}-*.md \
      and generate a plan directory under ./plans/ with plan.md and tasks.toml."
done
```

## The self-hosting bootstrap sequence

```
TODAY (manual):
  1. Complete the 7 wiring tasks (PROMPT-WIRING.md)
  2. roko plan run works end-to-end

NEXT (semi-automated):
  3. Manually create 1 plan directory from implementation-plans/01
  4. Run: roko plan run ./plans/
  5. Verify it works — roko executes the plan against itself

THEN (automated):
  6. Use roko to generate plans from implementation-plans/02-10
  7. Run: roko plan run ./plans/  (roko builds itself)

FINALLY (self-improving):
  8. Use roko to generate plans from MORI-PARITY-CHECKLIST.md
  9. Use roko to generate plans from PRD migration docs
  10. roko continuously reads its own docs and generates work
```

## Converting implementation-plans to roko plans

Each implementation-plan maps to 1-4 roko plan directories:

| Implementation plan | → Roko plans |
|---|---|
| `01-agent-wiring.md` (34 items) | `W01-claude-agent-system/`, `W02-claude-cli-agent/`, `W03-cli-backend-routing/`, `W04-other-backends/` |
| `02-system-prompt-integration.md` (15 items) | `W05-system-prompt-wiring/`, `W06-orchestrator-prompts/` |
| `03-safety-hooks.md` (17 items) | `W07-cli-settings-hooks/`, `W08-dispatcher-safety/` |
| `04-orchestrator-pipeline.md` (18 items) | `W09-runtime-harness/`, `W10-agent-pool/`, `W11-gate-pipeline/` |

The "W" prefix = Wiring plans (P0 priority).

## Bootstrapping: the first plan to create manually

Create this ONE plan directory by hand. Once roko can execute it, everything else follows.

```bash
mkdir -p plans/W01-wire-system-prompts
```

Then write `plan.md` and `tasks.toml` as shown above, run `roko plan run ./plans/`,
and verify roko can wire its own SystemPromptBuilder.

That's the bootstrap moment — roko modifying its own source code via its own orchestrator.
