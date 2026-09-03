# Plan Execution Example

This guide walks through executing a plan with roko's runner-v2 engine, from
writing a `tasks.toml` to interpreting gate results.

## Prerequisites

```bash
# Build roko (requires Rust 1.91+)
cargo build -p roko-cli

# Initialize a workspace if you haven't already
cargo run -p roko-cli -- init

# Verify your setup
cargo run -p roko-cli -- doctor
```

You need at least one LLM provider configured in `roko.toml`. Run
`cargo run -p roko-cli -- config providers list` to check.

## 1. The tasks.toml Format

Every plan lives in a directory under `plans/` and must contain a `tasks.toml`.
Here is a minimal example:

```toml
[meta]
plan = "my-first-plan"
total = 2
done = 0
status = "fixture"
max_parallel = 1
skip_enrichment = true
estimated_total_minutes = 5

[[task]]
id = "T01"
title = "Create a greeting module"
description = """Create `src/greeting.rs` with a public function
`pub fn hello(name: &str) -> String` that returns "Hello, {name}!"."""
status = "ready"
tier = "mechanical"
max_loc = 10
files = ["src/greeting.rs"]
role = "implementer"
depends_on = []

[[task.verify]]
phase = "structural"
command = "test -f src/greeting.rs"
fail_msg = "src/greeting.rs was not created"

[[task.verify]]
phase = "compile"
command = "cargo check --quiet"
fail_msg = "Code does not compile"

[[task]]
id = "T02"
title = "Add a test for the greeting module"
description = """Add a test in `src/greeting.rs` or `tests/greeting.rs` that
calls `hello("world")` and asserts the result equals "Hello, world!"."""
status = "ready"
tier = "mechanical"
max_loc = 15
files = ["tests/greeting.rs"]
role = "implementer"
depends_on = ["T01"]

[[task.verify]]
phase = "test"
command = "cargo test --quiet -- greeting"
fail_msg = "Greeting test does not pass"
```

### Key fields

| Field | Required | Description |
|---|---|---|
| `[meta].plan` | Yes | Plan identifier (used in logs and state) |
| `[meta].total` | Yes | Total task count |
| `[meta].max_parallel` | No | Maximum concurrent tasks (default 1) |
| `[meta].skip_enrichment` | No | Skip prompt enrichment for simple plans |
| `[[task]].id` | Yes | Unique task identifier within the plan |
| `[[task]].title` | Yes | Human-readable task title |
| `[[task]].description` | Yes | Full instructions for the agent |
| `[[task]].status` | Yes | Must be `"ready"` for executable tasks |
| `[[task]].tier` | Yes | Complexity: `mechanical`, `focused`, or `integrative` |
| `[[task]].max_loc` | No | Lines-of-code budget hint for the agent |
| `[[task]].files` | No | Files the task is expected to create or modify |
| `[[task]].role` | Yes | Agent role: `implementer`, `scribe`, `quick-reviewer`, etc. |
| `[[task]].depends_on` | Yes | List of task IDs that must complete first |
| `[[task.verify]]` | Yes | One or more verification commands |

### Verification phases

Each task should have at least one `[[task.verify]]` block. Common phases:

- `structural` -- file existence and structure checks
- `compile` -- compilation / syntax validation
- `test` -- test suite execution
- `evidence` -- cross-file traceability checks
- `acceptance` -- final acceptance criteria

### Optional context block

For tasks that need specific file context, add a `[task.context]` section:

```toml
[task.context]
read_files = [
    { path = "src/lib.rs", lines = "1-50", why = "Understand the module structure." },
]
symbols = [
    "pub fn hello — the function to implement",
]
anti_patterns = [
    "Do not modify any existing files.",
]
```

## 2. Validate the Plan

Before executing, validate the plan structure:

```bash
cargo run -p roko-cli -- plan validate plans/my-first-plan
```

Expected output:

```
Validating plan: plans/my-first-plan/tasks.toml
  Tasks: 2
  Dependencies: T02 -> T01
  Cycles: none
Plan is valid.
```

## 3. Execute the Plan

Run the plan with the runner-v2 engine:

```bash
cargo run -p roko-cli -- plan run plans/my-first-plan --engine runner-v2
```

Expected output progression:

```
[runner-v2] Loading plan: plans/my-first-plan/tasks.toml
[runner-v2] Plan "my-first-plan" — 2 tasks, max_parallel=1
[runner-v2] ── Phase: dispatch ──────────────────────────────
[runner-v2] T01: dispatching agent (role=implementer, tier=mechanical)
[runner-v2] T01: agent completed (tokens=1,240, cost=$0.02)
[runner-v2] ── Phase: gate ──────────────────────────────────
[runner-v2] T01: gate rung 1 (structural) — PASS
[runner-v2] T01: gate rung 2 (compile) — PASS
[runner-v2] T01: completed
[runner-v2] T02: dispatching agent (role=implementer, tier=mechanical)
[runner-v2] T02: agent completed (tokens=980, cost=$0.01)
[runner-v2] T02: gate rung 3 (test) — PASS
[runner-v2] T02: completed
[runner-v2] Plan "my-first-plan" completed — 2/2 tasks passed
```

### What happens during execution

1. The runner loads and validates the task DAG
2. Tasks with no unsatisfied dependencies are dispatched first
3. For each task, the runner:
   a. Assembles a system prompt using the 9-layer SystemPromptBuilder
   b. Dispatches an agent with the configured LLM provider
   c. Runs the 7-rung gate pipeline on the agent's output
   d. Records an efficiency event and episode
   e. On gate failure, optionally replans (controlled by `learning_config.replan_on_gate_failure`)
4. State is persisted to `.roko/state/state-snapshot.json` after each task

## 4. Resume an Interrupted Plan

If execution is interrupted (Ctrl+C, crash, network failure), resume from the
last checkpoint:

```bash
cargo run -p roko-cli -- plan run plans/my-first-plan --engine runner-v2 --resume-plan
```

The runner reads `.roko/state/state-snapshot.json` and skips already-completed
tasks.

## 5. Monitor Execution

### Interactive TUI

```bash
cargo run -p roko-cli -- dashboard
```

Use F1-F10 to switch between tabs. The Plans tab (F2) shows task progress,
gate results, and cost tracking in real time.

### CLI status

```bash
cargo run -p roko-cli -- status
```

### HTTP API (if `roko serve` is running)

```bash
# Plan status
curl -s http://localhost:6677/api/plans/my-first-plan/status | jq .

# Gate results
curl -s http://localhost:6677/api/plans/my-first-plan/gates | jq .

# Real-time SSE event stream
curl -N http://localhost:6677/api/events
```

## 6. Inspect Results

After execution, check the learning data:

```bash
# Efficiency metrics
cargo run -p roko-cli -- learn efficiency

# Episode history
cargo run -p roko-cli -- learn episodes

# Cascade router state (model routing decisions)
cargo run -p roko-cli -- learn router
```

## 7. Common Troubleshooting

### "No provider configured"

```
Error: no provider available for dispatch
```

Add a provider to `roko.toml`:

```toml
[providers.anthropic]
kind = "anthropic_api"
api_key_env = "ANTHROPIC_API_KEY"

[models.sonnet]
provider = "anthropic"
slug = "claude-sonnet-4-5"
```

Then set the environment variable: `export ANTHROPIC_API_KEY=sk-ant-...`

### Gate failure on a task

The runner retries failed tasks based on adaptive thresholds. If a task
repeatedly fails:

1. Check the gate output: the runner prints the failing `fail_msg`
2. Review the verify command -- run it manually to diagnose
3. Check if `replan_on_gate_failure` is enabled in your config
4. Inspect `.roko/learn/gate-thresholds.json` for adaptive threshold state

### Stale state after code changes

If you changed task definitions but the runner skips tasks:

```bash
# Delete the snapshot to force a fresh run
rm .roko/state/state-snapshot.json
cargo run -p roko-cli -- plan run plans/my-first-plan --engine runner-v2
```

### Disk space issues

The runner checks disk space before starting. If it refuses to run:

```bash
cargo run -p roko-cli -- doctor disk
```

Clean up stale build artifacts:

```bash
cargo clean
```

## 8. Reference Plans

The repository includes several example plans you can study:

| Plan | Description |
|---|---|
| `plans/demo-hello/` | Single-task smoke test (simplest possible plan) |
| `plans/demo-multistage/` | 5-task pipeline: discovery, evidence, decision, validation, review |
| `plans/demo-parallel-integration/` | Demonstrates parallel task execution |
| `plans/demo-resume-recovery/` | Tests the resume-from-checkpoint workflow |
