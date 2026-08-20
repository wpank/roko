# 99 — Examples: Add Core Workflow and Tasks.toml Authoring Examples

**Priority**: P3 — developer experience; new contributors cannot learn the core self-hosting workflow by example
**Size**: M (2-3 days)
**Crates**: None (documentation and config files only)
**Depends on**: None

---

## Background

The `examples/` directory exists at `/Users/will/dev/nunchi/roko/roko/examples/` and contains useful but narrow content: provider configuration files (`roko-ollama.toml`, `roko-openrouter.toml`, etc.), markdown guides for adding a provider or custom tools, and graph TOML examples for the Cell-based graph engine. What is missing is any example of the primary self-hosting workflow: PRD → plan → execute → gate → resume.

A new contributor who wants to use roko to develop a feature has no worked example showing how `tasks.toml` is structured, what fields are required, how verify steps work, how to resume an interrupted run, or how to configure the minimum `roko.toml` for a plan run. The CLI commands exist and work, but the first-time user experience requires reading source code or production plans to understand the format.

The existing production plans under `plans/` (e.g. `plans/demo-hello/tasks.toml`, `plans/P08-search-command-fix/tasks.toml`) demonstrate the full task format but are embedded in production context, not written as a learning artifact. The `examples/graphs/` directory shows how to write graph TOML but those are a different subsystem from the runner-v2 plan executor used by `roko plan run`.

## Current State

1. **What exists in `examples/`.** Files present:
   - `/Users/will/dev/nunchi/roko/roko/examples/adding-a-provider.md` — step-by-step guide for zero-code provider addition via `roko.toml`
   - `/Users/will/dev/nunchi/roko/roko/examples/adding-a-custom-protocol.md` — guide for Rust-level provider protocol changes
   - `/Users/will/dev/nunchi/roko/roko/examples/adding-custom-tools.md` — MCP tool integration guide
   - `/Users/will/dev/nunchi/roko/roko/examples/roko-*.toml` — provider configuration examples (Gemini, GLM, Kimi, LM Studio, multi-provider, Ollama, OpenRouter, Perplexity)
   - `/Users/will/dev/nunchi/roko/roko/examples/graphs/` — 8 graph TOML files (linear-gates, parallel-gates, conditional-branch, cognitive-loop, etc.)

2. **What does not exist in `examples/`.** No file covering: a minimal `tasks.toml` with explanation, a minimal `roko.toml` for plan execution, the PRD-to-plan CLI workflow, plan resume, gate configuration.

3. **Reference tasks.toml format.** From production plans, the canonical fields are:
   - `[meta]` section: `plan`, `total`, `done`, `status`, `max_parallel`, `estimated_total_minutes`, optional `skip_enrichment`
   - `[[task]]` entries: `id`, `title`, `description`, `status`, `tier` (mechanical/focused/integrative/architectural), `max_loc`, `files`, `role`, `depends_on`
   - Optional task fields: `model_hint`, `max_retries`, `timeout_secs`, `[task.context]` with `read_files` and `symbols`, `anti_patterns`
   - `[[task.verify]]` entries: `phase` (structural/compile/test), `command`, `fail_msg`, optional `timeout_ms`

4. **Reference roko.toml minimum for plan execution.** A plan run requires at minimum: an `[agent]` section specifying a provider, and a `[providers.*]` section. The `roko init` command generates a starter config.

5. **CLI commands for the core workflow.** All commands exist today:
   - `roko prd idea "<text>"` — capture idea
   - `roko prd draft new "<slug>"` — draft a PRD
   - `roko prd plan <slug>` — generate a plan from a PRD
   - `roko plan validate <dir>` — lint tasks.toml
   - `roko plan run <dir> --engine runner-v2` — execute
   - `roko plan run <dir> --engine runner-v2 --resume-plan` — resume after interrupt
   - `roko status` — check signal count and episode summary

## Implementation Plan

### Step 1: Create `examples/plan-execution/` directory with annotated tasks.toml

Create `/Users/will/dev/nunchi/roko/roko/examples/plan-execution/tasks.toml` — a minimal 2-task plan with all common fields annotated:

```toml
# Example: roko plan execution with two tasks and a dependency.
#
# Run this plan with:
#   roko plan validate examples/plan-execution/
#   roko plan run examples/plan-execution/ --engine runner-v2
#
# Resume after interruption with:
#   roko plan run examples/plan-execution/ --engine runner-v2 --resume-plan

[meta]
# plan must match the directory name.
plan = "plan-execution"
total = 2
done = 0
status = "ready"
# How many tasks may run concurrently.
max_parallel = 1
# Approximate expected runtime in minutes (informational only).
estimated_total_minutes = 5

# ── Task 1: No dependencies ───────────────────────────────────────────────────

[[task]]
# Unique ID within the plan. Conventionally: PLAN-T01, T01, etc.
id = "T01"
title = "Create a hello world file"
description = """
Create `examples/plan-execution/hello.txt` containing exactly:
  Hello from roko!

Do not modify any other file.
"""
# Task status. Must be "ready" for the runner to pick it up.
status = "ready"
# Tier controls model selection. Options: mechanical, focused, integrative, architectural.
tier = "mechanical"
# Max lines of code the agent is allowed to write. Set low to constrain scope.
max_loc = 3
# Files the agent is expected to modify or create.
files = ["examples/plan-execution/hello.txt"]
# Role controls which system prompt template is used.
role = "implementer"
# No dependencies — this task can start immediately.
depends_on = []

# Verify steps run after the agent finishes and gates each task.
# phase: "structural" (file/content checks), "compile" (build checks), "test" (test suite)
[[task.verify]]
phase = "structural"
command = "grep -q 'Hello from roko!' examples/plan-execution/hello.txt"
fail_msg = "examples/plan-execution/hello.txt must contain 'Hello from roko!'"

# ── Task 2: Depends on T01 ────────────────────────────────────────────────────

[[task]]
id = "T02"
title = "Append a timestamp comment to the hello file"
description = """
Append a second line to `examples/plan-execution/hello.txt`:
  # Written by roko plan runner

The file should now have exactly two lines.
"""
status = "ready"
tier = "mechanical"
max_loc = 1
files = ["examples/plan-execution/hello.txt"]
role = "implementer"
# This task waits for T01 to complete before starting.
depends_on = ["T01"]

[[task.verify]]
phase = "structural"
command = "grep -q '# Written by roko plan runner' examples/plan-execution/hello.txt"
fail_msg = "The second line must be '# Written by roko plan runner'"

[[task.verify]]
phase = "structural"
command = "test $(wc -l < examples/plan-execution/hello.txt) -eq 2"
fail_msg = "hello.txt must contain exactly 2 lines"
```

### Step 2: Create `examples/plan-execution/README.md`

Write a narrative walkthrough covering:
- Prerequisites (roko installed, API key set)
- How to validate the plan: `roko plan validate examples/plan-execution/`
- How to run: `roko plan run examples/plan-execution/ --engine runner-v2`
- What to expect (agent output, gate results, snapshot written to `.roko/state/`)
- How to resume after Ctrl-C: `roko plan run examples/plan-execution/ --engine runner-v2 --resume-plan`
- How to check status: `roko status`
- Where to find logs: `.roko/roko.log`, `.roko/episodes.jsonl`
- How to create your own plan (annotated field reference pointing to the tasks.toml above)

### Step 3: Create `examples/prd-workflow/README.md`

Document the PRD → plan → run loop as a shell walkthrough:

```bash
# 1. Capture an idea
roko prd idea "Add a greeting command that prints hello"

# 2. Draft a PRD (agent-driven, requires API key)
roko prd draft new "greeting-command"

# 3. Generate a plan from the PRD (agent-driven)
roko prd plan greeting-command

# 4. Review the generated plan
cat plans/greeting-command/tasks.toml

# 5. Validate before running
roko plan validate plans/greeting-command/

# 6. Execute
roko plan run plans/greeting-command/ --engine runner-v2
```

Include notes on what each step produces (files created, where to look), and how the `prd.auto_plan = true` config key triggers plan generation automatically on `prd publish`.

### Step 4: Annotate `examples/graphs/linear-gates.toml` with a note distinguishing graph execution from plan execution

Add a comment block to the top of the existing file clarifying that graph TOML runs via `roko graph run` and is distinct from the runner-v2 plan executor (`roko plan run`). One sentence is sufficient.

### Step 5: Verify the examples are referenced from a central index

Check whether `plans/INDEX.md` or any top-level index mentions `examples/`. If not, add a brief entry to `plans/INDEX.md` pointing to the new example directories.

## Acceptance Criteria

1. `/Users/will/dev/nunchi/roko/roko/examples/plan-execution/tasks.toml` exists and passes `roko plan validate examples/plan-execution/`.
2. `/Users/will/dev/nunchi/roko/roko/examples/plan-execution/README.md` exists with prerequisites, run command, resume command, and log locations.
3. `/Users/will/dev/nunchi/roko/roko/examples/prd-workflow/README.md` exists with the 6-step CLI sequence.
4. Every CLI command referenced in the new examples is a real command that exists today (no hypothetical APIs).
5. The tasks.toml example uses only fields documented by the existing production plans (no invented fields).
6. `roko plan validate examples/plan-execution/` exits 0.

### Not in Scope

- Video tutorials or screencasts
- Web-based interactive documentation
- Examples for chain, DeFi, or marketplace features (Phase 2+)
- Examples requiring a live GitHub token or deployed server

## Verification Checklist

- [ ] `roko plan validate examples/plan-execution/` exits 0
- [ ] `grep -r 'cargo run\|hypothetical' examples/plan-execution/` returns no results (no fake commands)
- [ ] `examples/plan-execution/tasks.toml` fields match fields in `plans/demo-hello/tasks.toml`
- [ ] `examples/prd-workflow/README.md` CLI commands exist in `roko --help` output

## Files to Modify

| File | Change |
|---|---|
| `/Users/will/dev/nunchi/roko/roko/examples/plan-execution/tasks.toml` | Create: annotated 2-task example plan |
| `/Users/will/dev/nunchi/roko/roko/examples/plan-execution/README.md` | Create: narrative walkthrough with run/resume/log instructions |
| `/Users/will/dev/nunchi/roko/roko/examples/prd-workflow/README.md` | Create: PRD-to-plan CLI workflow as a shell script walkthrough |
| `/Users/will/dev/nunchi/roko/roko/examples/graphs/linear-gates.toml` | Add 1-line comment distinguishing graph execution from plan execution |
| `/Users/will/dev/nunchi/roko/roko/plans/INDEX.md` | Add reference to examples/ directory |
