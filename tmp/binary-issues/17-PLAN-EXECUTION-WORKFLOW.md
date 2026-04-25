# 17 — Plan & Execution Workflow Audit

**Status**: open (critical)
**Scope**: `crates/roko-cli/src/run.rs`, `crates/roko-cli/src/orchestrate.rs`, `crates/roko-cli/src/runner/`, `crates/roko-cli/src/plan_generate.rs`, `crates/roko-cli/src/prd.rs`

## What This Document Covers

When a user wants roko to actually *do work* — build things, implement features, fix bugs
— they use `roko run "prompt"`, `roko plan generate`, or `roko plan run`. This doc audits
every step of that flow: one-shot execution, plan generation, plan execution, gate
validation, and state persistence.

---

## 1. One-Shot Mode (`roko run "prompt"`)

### How it works

User runs `roko run "fix the bug"`. The v2 engine (default) calls `WorkflowEngine::run`.
The agent gets:
- A proper system prompt via `build_role_system_prompt_validated` (9-layer builder)
- Tools via `claude_tool_allowlist()` scoped by role
- MCP config passthrough if set in `roko.toml`

### Issues

**R1. No codebase context in one-shot mode** (`run.rs:1124`)

```rust
let workspace = "Single-shot execution through `roko run`.";
```

The workspace description is a static string. The agent gets zero information about the
project — no file structure, no language, no crate layout, no key files. Compare with
`plan run` which injects code-intelligence context, knowledge store entries, playbook
rules, and anti-patterns.

A user running `roko run "fix the failing test in roko-gate"` expects the agent to
understand the project. Instead, the agent is blind and must rediscover everything
through tool calls.

**R2. Hardcoded model display string** (`run.rs:612`)

```rust
output_format::step("model", "claude-sonnet-4-20250514");
```

Prints `claude-sonnet-4-20250514` regardless of what model is actually configured. The
real model is resolved from config, but the display line is hardcoded. The user sees a
lie.

**R3. No streaming output in v2 engine** (`run.rs:583-678`)

Prints "starting workflow...", then goes silent until completion. No real-time progress.
No visibility into what the agent is doing turn-by-turn. For a task that takes 5 minutes,
the user stares at nothing.

**R4. Output truncated with no way to see full result** (`run_inline.rs:102-113`)

```rust
if md_lines.len() > 30 {
    term.push_lines_revealed(&md_lines[..25], Duration::from_millis(20))?;
    // ... +N more lines
}
```

Long agent outputs show only the first 25 lines. No pager, no flag to show full output.
The rest is only visible in `.roko/engrams.jsonl`.

**R5. Completion summary lacks actionable info** (`run_inline.rs:117-153`)

Shows: pass/fail, elapsed time, episode ID. Does NOT show: what files were changed, what
the agent actually did, cost/tokens, or what to do next if it failed.

---

## 2. Plan Generation (`roko plan generate`, `roko prd plan`)

### How it works

Both commands spawn a Claude agent with a comprehensive system prompt
(`PLAN_GENERATOR_SYSTEM_PROMPT` in `plan_generate.rs:164-267`) that includes CLAUDE.md
content, naming glossary, and source material. The agent is told to search the codebase
before generating tasks. Output is `tasks.toml`.

### Issues

**PG1. No validation that generated tasks.toml is well-formed** (`commands/plan.rs:456-472`)

After `plan generate` completes, there is NO post-generation validation that:
- A `tasks.toml` file was actually created
- The TOML parses correctly
- File paths referenced in `[task.context]` exist
- Dependencies form a valid DAG

Contrast with `plan regenerate` (lines 570-610) which DOES validate and rolls back on
parse failure. The `generate` path has no rollback.

**PG2. Agent not structurally forced to search codebase** (`plan_generate.rs:238-249`)

The system prompt says "Before generating tasks, you MUST search the codebase..." — but
this is a prompt instruction, not enforced. The agent can generate plans without reading
any files. No validation ensures the agent actually searched.

**PG3. Plan generation failure gives bare-bones error** (`prd.rs:841-844`)

```rust
if exit_code != 0 {
    return Err(anyhow!("plan generation agent failed with exit code {exit_code}"));
}
```

One-line error with just the exit code. No agent output, no diagnostic info, no suggestion.

---

## 3. Plan Execution (`roko plan run`)

### Two execution engines exist

This is a source of confusion itself:

1. **Runner v2** (`crates/roko-cli/src/runner/`): The active path used by `plan run`.
   Uses `tokio::select!` event loop with agent event channels and gate dispatch.

2. **Legacy PlanRunner** (`orchestrate.rs`): 21K+ line orchestration engine. Still
   reachable through some code paths. Much richer feature set (playbooks, dreams, daimon,
   anti-patterns, HDC fingerprinting).

### Issues

**PE1. `dangerously_skip_permissions: true` hardcoded** (`commands/plan.rs:290`)

```rust
dangerously_skip_permissions: true,
```

Every `roko plan run` gives agents full unrestricted permissions. No opt-in to safe mode.
No visibility or control over this. The flag name itself tells you it should not be the
default.

**PE2. Task execution hardcoded to sequential** (`runner/event_loop.rs:115`)

```rust
max_concurrent_tasks: 1,
```

Despite the executor supporting parallel dispatch (the DAG respects `depends_on` and
`parallel_groups`), the runner v2 hardcodes concurrency to 1. Users with independent tasks
wait serially.

**PE3. No real-time feedback during plan execution** (`runner/event_loop.rs:338-345`)

The event loop processes `AgentEvent` variants but only publishes them to the TUI bridge.
A user running `roko plan run` without the TUI sees near-zero feedback between "starting"
and "done" for each task.

**PE4. Two plan execution engines = maintenance burden**

The runner v2 and legacy PlanRunner have different feature sets:
- Legacy has: playbooks, dreams, daimon, anti-patterns, HDC, curriculum ordering, worktrees
- Runner v2 has: cleaner async, better state management, TUI bridge

They can diverge in behavior and neither is clearly deprecated.

---

## 4. Agent Dispatch During Plan Execution

### What agents get (good)

In the legacy PlanRunner, each task's agent gets:
- Role-specific system prompt (9-layer builder)
- Task-specific context from `tasks.toml` `[task.context]`
- Code intelligence (file reads with line ranges)
- Anti-patterns from knowledge store
- Playbook matches from prior successful tasks
- Daimon affect modulation
- Gate failure feedback (if retrying)

### Issues

**AD1. Agents cannot see each other's work in parallel mode**

In sequential execution (the default per PE2), this doesn't matter. But if parallel
execution were enabled, agents would share a directory without coordination. The legacy
PlanRunner mitigates with worktrees; the runner v2 does not.

**AD2. `cat` default agent command not caught in all paths** (`agent_exec.rs:98-110` vs `run.rs`)

The `agent_exec` path catches the `"cat"` default and fails fast. But the `run.rs`
one-shot path does NOT have this guard. If `roko.toml` has `command = "cat"` (the default
when no config exists), `roko run` invokes `cat` as the agent, which echoes the prompt
back as "output" and reports success. This produces garbage that looks like it worked.

---

## 5. Gate Pipeline

### What gates run

After each task, a "rung ladder": Compile → Lint → Test → Symbol → GeneratedTest →
PropertyTest → Integration. Default `max_gate_rung` is 2 (Compile + Lint + Test).

### What happens on gate failure (legacy PlanRunner)

1. Failure context stored in task tracker
2. `FailureTrace` emitted for observability
3. Executor transitions to AutoFix phase
4. If `replan_on_gate_failure` enabled, builds a revision prompt

### Issues

**G1. Gate failure messages in one-shot mode are minimal** (`run.rs:1024`)

Gate results are stored as `(gate_name, bool)` pairs. The user sees:
```
gates:
  [PASS] compile
  [FAIL] test
```

No output from the failing gate. No error message. No compiler output. No test failure.
No hint about what to fix.

**G2. `replan_on_gate_failure` requires three separate flags** (`orchestrate.rs:4712-4714`)

```rust
fn gate_failure_replan_enabled(&self) -> bool {
    self.learning_config.replan_on_gate_failure
        && !self.no_replan
        && self.executor.config().auto_replan
}
```

Three flags must all be true. If any is missing from the user's config, automatic
replanning silently doesn't happen.

---

## 6. State Persistence and Resume

### What persists
- Legacy: `.roko/state/executor.json` (full snapshot), events, task trackers, daimon state
- Runner v2: `.roko/state/run-state.json` with task fingerprints
- Both use atomic write (temp file + rename)

### Resume works well

- Auto-saves every 5 actions
- Validates task fingerprints on resume
- Handles crash recovery (JSONL trailing-garbage detection)
- Ctrl+C triggers graceful drain with 3-second timeout

### Issues

**SP1. Stale snapshot can interfere with new runs**

The runner v2 always resumes from `.roko/state/executor.json` if it exists. A snapshot
from a previous plan could interfere with a new run. Fingerprint validation catches drift,
but the error message may be opaque.

---

## 7. The Self-Hosting Flow (`idea → draft → plan → execute → validate`)

### The flow works end-to-end

1. `roko prd idea "text"` → appends to `.roko/prd/ideas.md`
2. `roko prd draft new "slug"` → agent drafts PRD in `.roko/prd/drafts/slug.md`
3. `roko prd plan slug` → agent generates `plans/<slug>/tasks.toml`
4. `roko plan run plans/` → executes tasks through agents with gate validation
5. `roko plan run plans/ --resume` → resumes from checkpoint

### Where it breaks down

**SH1. No validation between plan generation and execution**

`prd plan` can generate tasks.toml that `plan run` cannot execute — invalid TOML, circular
dependencies, nonexistent file references. `plan validate` exists but is not run
automatically between generation and execution.

**SH2. No automated progression**

Each step is fully manual. `prd.auto_plan` triggers plan generation on PRD publish, but
there is no auto-execute after generation. The user must manually run each command.

**SH3. PRD draft quality is unvalidated**

The draft agent may produce garbage. No structural validation that the PRD has proper
requirements, acceptance criteria, or is even parseable.

---

## Anti-Patterns

1. **Blind agents**: One-shot mode gives agents zero codebase context. They must
   rediscover everything through tool calls, wasting tokens and time.

2. **Cosmetic lies**: Hardcoded model display string, hardcoded permissions, truncated
   output — the UI shows things that don't match reality.

3. **Parallel infrastructure unused**: DAG executor supports parallel tasks, but
   concurrency is hardcoded to 1. The feature is built but neutered.

4. **Fail-open defaults**: `dangerously_skip_permissions: true` is the default for all
   plan runs. Three separate flags must be explicitly set for gate replanning.

5. **Invisible progress**: Both `roko run` and `roko plan run` (without TUI) show minimal
   feedback during execution. No streaming, no per-turn updates, no progress bars.

6. **Two engines, neither deprecated**: Runner v2 and legacy PlanRunner coexist with
   different feature sets. No clear migration path.

---

## Root Cause Fix

1. **Workspace context for all dispatch paths** — one-shot mode should inject the same
   codebase context (at minimum: project structure, language, key files) that the
   orchestrator provides.

2. **Stream everything** — agent turns should be streamed to the terminal in real-time,
   not buffered. The user needs to see what's happening.

3. **Validation gates between steps** — plan generation should auto-validate the output.
   The flow should be: generate → validate → (fix if invalid) → present to user.

4. **Parallel execution by default** — the DAG should be respected. Independent tasks
   should run in parallel with configurable concurrency.

5. **Permission escalation, not skip** — agents should start with restricted permissions
   and escalate with user approval, not start with everything unlocked.

---

## Checklist

### One-Shot (`roko run`)
- [ ] Inject codebase context (project structure, language, key files)
- [ ] Fix hardcoded model display string
- [ ] Add streaming output
- [ ] Show full output (pager or expandable)
- [ ] Show files changed, cost, tokens in completion summary
- [ ] Catch `cat` default in run.rs path

### Plan Generation
- [ ] Auto-validate generated tasks.toml
- [ ] Roll back on parse failure (like `regenerate` does)
- [ ] Show agent output on generation failure
- [ ] Auto-run `plan validate` after generation

### Plan Execution
- [ ] Remove hardcoded `dangerously_skip_permissions: true`
- [ ] Make `max_concurrent_tasks` configurable (default > 1 for independent tasks)
- [ ] Show real-time agent progress without requiring TUI
- [ ] Consolidate runner v2 and legacy PlanRunner (or clearly deprecate one)

### Gates
- [ ] Show gate failure output (compiler errors, test failures)
- [ ] Simplify gate replanning config (one flag, not three)

### Self-Hosting Flow
- [ ] Auto-validate between `prd plan` and `plan run`
- [ ] Optional auto-execute after plan generation
- [ ] Structural validation for PRD drafts
