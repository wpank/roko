# Plan Workflow Example

This guide covers the plan-first workflow from start to finish: an agent writes
a plan from your prompt, you review and edit it, and roko runs it.

## Overview

```
prompt  -->  plan  -->  review and edit  -->  run
```

A plan is a directory under `plans/` holding a `tasks.toml` (the task graph)
and a `plan.md` (the narrative). An agent writes the first version from your
prompt. The files are yours to read, change and commit before anything runs.
The Graph engine then runs the plan: each task starts as soon as its
dependencies finish, works in its own git worktree, and counts as done only
when its `verify` commands and the gates pass.

## Prerequisites

```bash
# Build roko
cargo build -p roko-cli

# Initialize workspace
cargo run -p roko-cli -- init

# Verify a provider is configured (needed to write and to run plans)
cargo run -p roko-cli -- config providers list
```

## 1. Write a Plan from a Prompt

```bash
cargo run -p roko-cli -- plan generate "Add a gate retry counter"
```

An agent reads the prompt and the codebase and writes the plan to
`plans/add-a-gate-retry-counter/`:

```
plans/add-a-gate-retry-counter/
├── tasks.toml   # tasks, dependencies, the files each task may touch, verify commands
└── plan.md      # what the plan does and why
```

`plan generate` runs no tasks. It validates the plan it wrote and exits
non-zero when the plan fails validation.

The directory name, the plan's slug, comes from the prompt's words. These
options shape the plan:

| Option | Effect |
|---|---|
| `--context <PATH>` | Add files, directories or globs to the planner's prompt (repeatable) |
| `--from-file <PATH>` | Read the request from a file; the file's name becomes the slug |
| `--model <KEY>` | Write the plan with this model instead of `[authoring] planner_model` |

A longer request reads better from a file:

```bash
cat > gate-retry-counter.md <<'EOF'
Add a retry counter to the gate pipeline. Record how many times each rung is
retried per task, include the counts in the efficiency events, and report the
average retries per gate in GET /api/gates/summary.
EOF

cargo run -p roko-cli -- plan generate --from-file gate-retry-counter.md \
  --context crates/roko-gate/src/gate_pipeline.rs
```

This writes `plans/gate-retry-counter/`, the plan the rest of this guide uses.

### Or in one command

`roko run --plan` writes the plan and then runs it. On a terminal it shows the
plan and asks before running it; `--yes` skips the question. With `--dry-run`
it writes the plan and stops, like `plan generate`:

```bash
cargo run -p roko-cli -- run --plan "Add a gate retry counter"
cargo run -p roko-cli -- run --plan --dry-run "Add a gate retry counter"
```

Without `--plan`, `roko run "<prompt>"` sizes the prompt itself: a small change
runs as one checked task, and a larger one gets a plan written first.

## 2. Improve the Plan with Research (Optional)

```bash
cargo run -p roko-cli -- research enhance-plan gate-retry-counter
cargo run -p roko-cli -- research enhance-tasks gate-retry-counter
```

`enhance-plan` has an agent rework `plan.md` and `tasks.toml` in place with
research-backed techniques: finer task decomposition, exact file and line
context for each task, runnable verify commands, and the cheapest suitable
model for each task. `enhance-tasks` works on `tasks.toml` alone: it splits
large tasks, adds read context with line ranges, removes dependency edges that
block parallelism, and checks that the file paths exist.

To research a topic first, `roko research topic "<topic>"` writes a report with
citations under `.roko/research/`. Pass that report to the planner with
`--context`.

## 3. Review and Edit the Plan

The plan is plain text. Change whatever you like: the task wording, the files a
task may touch, the dependencies, the verify commands.

```bash
$EDITOR plans/gate-retry-counter/tasks.toml plans/gate-retry-counter/plan.md
```

[Example plan](#example-plan) below shows the format, and the
[plan-execution](../plan-execution/README.md) example explains every field.

Then lint it:

```bash
cargo run -p roko-cli -- plan validate plans/gate-retry-counter
```

`--spec-quality` also scores each task's spec, `--dag` prints the dependency
analysis (waves, critical path, dangling references), and `--strict` fails on
warnings as well as errors.

Preview the run without executing anything:

```bash
cargo run -p roko-cli -- plan show gate-retry-counter
cargo run -p roko-cli -- plan run plans/gate-retry-counter --dry-run
```

A plan is ordinary files, so you can commit it with the rest of your change as
a record of what ran.

## 4. Run the Plan

```bash
cargo run -p roko-cli -- run plans/gate-retry-counter
```

This is the same run as `roko plan run plans/gate-retry-counter`, which takes
every plan-run option (`--max-tasks`, `--budget-override`, `--no-tui` and the
rest). Independent tasks run in parallel, each in its own git worktree, and a
failed check goes back to the agent with its errors for another attempt.
Finished work is delivered to the run's batch branch, `roko/batch/<run-id>`.
Your checkout is not changed; the run ends by printing the command that takes
the work into it.

`roko run plans/` runs every plan under `plans/`. See the
[plan-execution](../plan-execution/README.md) example for detailed execution
guidance.

## 5. Resume an Interrupted Run

Progress is checkpointed after every task under `.roko/state/graph/<plan>/`.
To pick up where a run stopped:

```bash
cargo run -p roko-cli -- plan run plans/gate-retry-counter --resume-plan
```

Passed tasks stay done. `--fresh` archives the old run state and starts over.

## 6. Watch Progress

```bash
cargo run -p roko-cli -- plan status plans/gate-retry-counter   # task states
cargo run -p roko-cli -- status                                 # signals, episodes, gate results
cargo run -p roko-cli -- dashboard                              # live TUI; F2 shows the plan
```

## Example Plan

After review, `plans/gate-retry-counter/tasks.toml` might look like this:

```toml
[meta]
plan = "gate-retry-counter"
total = 2
done = 0
status = "ready"
max_parallel = 1

[[task]]
id = "T01"
title = "Record the attempt number of each gate rung run"
description = """In `crates/roko-gate/src/gate_pipeline.rs`, record a 1-based
attempt number each time a rung runs for a task, and include the per-rung
counts in the efficiency event. New fields use `#[serde(default)]` so existing
JSONL records still parse."""
status = "ready"
tier = "focused"
role = "implementer"
files = ["crates/roko-gate/src/gate_pipeline.rs"]
depends_on = []

[[task.verify]]
phase = "test"
command = "cargo test -p roko-gate"
fail_msg = "roko-gate tests do not pass"

[[task]]
id = "T02"
title = "Report average retries per gate in the gate summary"
description = """Add `avg_retries` per gate to the `GET /api/gates/summary`
response in `crates/roko-serve/src/routes/status/gates.rs`, computed from the
attempt numbers T01 records."""
status = "ready"
tier = "focused"
role = "implementer"
files = ["crates/roko-serve/src/routes/status/gates.rs"]
depends_on = ["T01"]

[[task.verify]]
phase = "test"
command = "cargo test -p roko-serve gates"
fail_msg = "the gate summary tests do not pass"
```

And `plan.md` holds the narrative:

```markdown
# Gate retry counter

Record how many times each gate rung runs per task, and report the average
retries per gate in `GET /api/gates/summary`.

T01 records the attempt numbers in the gate pipeline. T02 reports them through
the HTTP API once T01 lands.
```

## Full Workflow Summary

```bash
# 1. Write a plan from a prompt (or from a file with --from-file)
cargo run -p roko-cli -- plan generate --from-file gate-retry-counter.md

# 2. (Optional) Improve it with research
cargo run -p roko-cli -- research enhance-plan gate-retry-counter

# 3. Review and edit plans/gate-retry-counter/, then lint it
cargo run -p roko-cli -- plan validate plans/gate-retry-counter

# 4. Run it
cargo run -p roko-cli -- run plans/gate-retry-counter

# 5. Resume if interrupted
cargo run -p roko-cli -- plan run plans/gate-retry-counter --resume-plan

# 6. Monitor progress
cargo run -p roko-cli -- dashboard
```

## In the Portal

`roko serve` prints a `portal:` link. In the portal you type a prompt and the
plan appears; you edit its `tasks.toml` as text, the server validates each
save, and Run executes it.

## HTTP API Equivalents

If `roko serve` is running on `:6677`, the same workflow is available over
HTTP. The `/api/` routes need the launch token `roko serve` writes to
`.roko/runtime/serve.token`, or an API key:

```bash
BASE=http://localhost:6677/api
KEY=$(cat .roko/runtime/serve.token)

# Write a plan from a prompt. Returns 202 {"id": "<operation id>", "plan_id": "<slug>"}
GEN=$(curl -s -X POST "$BASE/plans/generate" -H "X-Api-Key: $KEY" \
  -H 'Content-Type: application/json' \
  -d '{"prompt": "Add a gate retry counter"}')
OP=$(echo "$GEN" | jq -r .id)
PLAN=$(echo "$GEN" | jq -r .plan_id)

# Poll the operation until "status" is "completed" (or "failed", with an "error")
curl -s "$BASE/operations/$OP" -H "X-Api-Key: $KEY" | jq .

# Read the plan's tasks.toml, edit it, and save it back (the save is validated first)
curl -s "$BASE/plans/$PLAN/source" -H "X-Api-Key: $KEY" | jq -r .toml > "$PLAN.tasks.toml"
$EDITOR "$PLAN.tasks.toml"
jq -Rs '{toml: .}' "$PLAN.tasks.toml" | curl -s -X PUT "$BASE/plans/$PLAN/source" \
  -H "X-Api-Key: $KEY" -H 'Content-Type: application/json' -d @- | jq .

# (Optional) Research-backed improvements to the plan
curl -s -X POST "$BASE/research/enhance-plan/$PLAN" -H "X-Api-Key: $KEY" | jq .

# Run it, then check progress
curl -s -X POST "$BASE/plans/$PLAN/execute" -H "X-Api-Key: $KEY" | jq .
curl -s "$BASE/plans/$PLAN/status" -H "X-Api-Key: $KEY" | jq .
```

`POST /api/plans/{id}/validate` with `{"toml": "..."}` checks an edit without
saving it, and `POST /api/plans/{id}/execute` with `{"resume": true}` resumes a
run from its checkpoint.
