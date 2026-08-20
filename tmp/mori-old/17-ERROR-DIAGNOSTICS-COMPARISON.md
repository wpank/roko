# Error Diagnostics Comparison: Mori vs Roko

## 1. Mori's Error Display (F1:dash Errors sub-tab)

### Architecture

The error digest widget (`tui/widgets/error_digest.rs`) aggregates four error
sources into a single scrollable panel:

1. **Gate output errors** -- Parsed from `cargo check`/`cargo test` raw output.
   Groups errors by file, shows line numbers, deduplicates identical messages
   with `(x3)` counts, and sorts errors before warnings.

2. **Pipeline errors** -- A global `state.error: Option<String>` set by the
   orchestrator when a fatal condition is hit (e.g. spawn failure, worktree
   corruption).

3. **Preflight warnings** -- From `orchestrator/preflight.rs`: checks for fast
   linker (mold/lld), sccache, and cargo-nextest at startup. These are
   actionable DX hints, not blockers.

4. **Runtime issues** -- Last 6 log entries at Warn/Error level, deduplicated
   by `source:message` key. Colorized by keyword detection: "exited", "error",
   "failed" get EMBER; "drift", "warning", "partial" get WARNING.

### How gate output is parsed

```rust
struct ErrorEntry {
    file: String,
    line_num: Option<u32>,
    message: String,
    severity: Severity, // Error | Warning
}
```

Lines starting with `error[E` or `error:` become Error entries. Lines with
`-->` extract file:line. Lines with `warning` become Warning entries. Entries
are grouped by file (`ErrorGroup`) for visual hierarchy.

### Visual treatment

- Error count shown in the panel title: `Errors (3)`.
- Border turns EMBER (red) when errors exist.
- Tree-style connectors (`+--`, `L--`) for entries within a file group.
- Auto-scrolls to bottom (most recent).
- Empty space below errors filled with ambient "activity ripples" whose
  intensity scales with error count.


## 2. Agent Failure Reporting (Mori)

### Structured error digest for agents

`gates.rs::extract_error_digest()` pulls unique `error[E...]` blocks with
file:line references, caps at 10 unique errors, and formats as:

```
3 unique error(s):

error[E0308]: mismatched types
  --> src/foo.rs:12:5
  |

error[E0432]: unresolved import
  --> src/bar.rs:3:1
```

This digest goes into `GateResult.error_digest` and is injected into the
agent's next turn prompt, giving agents targeted signal instead of raw
compiler output pages.

### Discovered patterns

`gates.rs::append_discovered_pattern()` persists error signatures to
`.mori/runs/discovered-patterns.json` (last 20 entries). Parallel agents can
read these to avoid re-discovering the same errors. Injected into context as:

```markdown
## Discovered Patterns (from parallel agents)
- **03-feature**: error[E0308] mismatched types at src/foo.rs
```

### Agent crash classification

Mori does not have a formalized agent crash classifier -- it relies on the
orchestrator's generic error handling. Roko is ahead here (see section 12).


## 3. Diagnostic System (z:diag in F2:plans)

Pressing `z` in the Plans tab triggers `DiagnosePlan(plan_name)`, which pops
a confirmation modal. On confirm, in **parallel mode** the executor runs a
diagnostic analysis that:

1. Inspects the plan's current phase and failure state.
2. Examines the worktree's git state, branch divergence, and diff.
3. Collects the most recent gate output and error digest.
4. Writes a structured recovery report to the plan's context directory.

In **sequential mode**, pressing `z` logs a warning: "Plan recovery actions
are only available in parallel mode."

The diagnostic is focused on the operator -- it answers "why did this plan
get stuck?" with a machine-readable report that a human or recovery agent
can act on.


## 4. Retry System (s/r in F2:plans)

Pressing `s` or `r` triggers `SoftRetryPlan(plan_name)`:

- In **parallel mode**: `retry_failed_plan_from_canonical_state()` resets
  the plan back to its last good checkpoint while preserving completed work.
  It reloads the canonical task ledger, determines which tasks still need
  doing, and re-enters the scheduling loop. A rebase from the latest batch
  branch is optionally applied to pick up code that other plans merged since
  the failure.

- In **sequential mode**: "Soft-retry not available in sequential mode."

The key design: retry preserves work already done. If 8/10 tasks were
complete and the 9th failed, a retry only re-runs tasks 9 and 10.


## 5. Repair System (S/R in F2:plans)

Two repair variants:

- `S` = **RepairPlanPreserve**: Repairs the plan infrastructure (worktree,
  branch, task ledger) while preserving implementation work. Used when the
  worktree is corrupted but the code is still good.

- `R` = **RepairPlanClean**: Clean-slate repair. Deletes the worktree and
  branch, clears all state, and reschedules from scratch. Used when
  corruption is unrecoverable.

Both are guarded by confirmation modals and only available in parallel mode.
The worktree recovery engine classifies issues as:

```rust
enum RecoveryDecision {
    Healthy,
    NeedsResync,
    NeedsRebase,
    ParseRepair,        // Task ledger schema issue
    QuarantineAndRecreate,
    ManualAttention,
}
```


## 6. Reverify System (c in F2:plans)

Pressing `c` triggers `ReverifyPlan(plan_name)`:

```rust
pub fn reverify_plan(&mut self, plan: &str) -> Vec<ExecutorAction> {
    state.phase = PlanPhase::Gating;
    // Remove from merge queue, clear review flags
    vec![
        ExecutorAction::EnsureWorktree { plan },
        ExecutorAction::RunPlanGates { plan },
    ]
}
```

This moves the plan back to the Gating phase **without** re-running any
implementation tasks. Only gates (compile, test, clippy, diff) and reviews
re-run. Used when external changes (merged dependency fixes from another
plan) should fix a gate failure.


## 7. Preflight Checks (Mori)

`orchestrator/preflight.rs::preflight_dx_checks()` runs at startup and checks:

1. **Fast linker**: Is mold or lld configured in `.cargo/config.toml`?
2. **sccache**: Installed and configured (env var or config)?
3. **cargo-nextest**: Installed?
4. (Skipped: `cargo fmt` -- formatting is only checked in worktrees.)

These are **warnings**, not blockers. They appear in the Errors sub-tab
under the "Preflight" section.

### What Mori does NOT preflight (that matters)

- API key validity
- Disk space
- Git state health
- Configuration schema
- Provider reachability


## 8. Gate Failure Communication (Mori)

### To the operator (TUI)

1. `OrchestratorEvent::GateResult { gate, passed, output }` is received.
2. Gate output is stored in `state.last_gate_output`.
3. Error digest widget parses it (see section 1).
4. Phase bar shows the plan in `compile-gate` or `test-gate` phase.
5. Log entry: `[gate] compile: FAIL` with Error level.
6. If iteration > 1, the phase bar shows the iteration number.

### To the agent (prompt injection)

1. `extract_error_digest()` creates a focused error summary.
2. `append_discovered_pattern()` persists the pattern for other agents.
3. The reflection system generates a verbal analysis (see section 8a).

### 8a. Reflection System

`orchestrator/reflection.rs::spawn_reflection()` fires a background Haiku
call to analyze gate failures:

**Prompt structure:**
```
## What failed
<one sentence>

## Why it failed
<root cause analysis, 2-3 sentences>

## What to try differently
<concrete action items, 2-3 bullets>

## Files/functions to focus on
<specific file:function references>
```

Reflections are stored in `IterationMemory` per-plan, deduped by first error
line, and injected into the agent's next-attempt prompt as:

```markdown
# Prior Iteration Reflections
These are analyses of previous failed attempts...

## Iteration 1 Reflection
[haiku's analysis]
Files changed in that attempt: src/lib.rs, src/config.rs
```

Older reflections are compressed to one-liners; only the last 3 get full
detail.


## 9. Express Mode Auto-Fix

When a plan in **express mode** fails a gate:

1. `PlanPhase::AutoFixing` is entered.
2. `ExecutorAction::AutoFixErrors { plan, errors }` is emitted.
3. An `AutoFixer` agent role is spawned with the structured error digest.
4. The auto-fixer attempts `cargo fix --allow-dirty --workspace` plus `cargo fmt`.
5. If the fix compiles, gates re-run.
6. After N failures: `FailureKind::AutoFixExhausted`.

Additionally, `autofix.rs` contains:

- **`CompileErrorClass`**: Structured classification of compile errors:
  ImportNotFound, TypeMismatch, MissingField, TraitNotImplemented, Other.
- **`parse_cargo_json_errors()`**: Parses `cargo check --message-format=json`
  into classified errors.
- **`collect_rustc_suggestions()`**: Extracts machine-applicable fix suggestions.
- **`apply_rustc_fixes()`**: Applies rustc suggestions + cargo fmt.
- **`generate_compile_fix_plan()`**: Creates a TOML fix plan from errors.
- **`generate_issue_plan()`**: Creates a structured issue document when an
  agent is stuck after multiple attempts (feeds into research pipeline).
- **`classify_invariant_failures()`**: Distinguishes CodeBug (fix code) from
  SpecIssue (review spec) from MissingTest (write test).


## 10. Log Format and Filtering (F5:logs)

`tui/views/logs.rs` renders structured log entries:

```
HH:MM:SS [icon][event_icon] [source] message
```

- **Time**: `%H:%M:%S` format.
- **Level icons**: Error = cross, Warn = warning triangle, Info = dot, Debug = dots.
- **Event icons**: completed/APPROVE = checkmark, phase transition = arrow,
  retry/iteration = loop, gate/cargo = gear.
- **Source coloring**: Mapped to agent role accents (Conductor, Strategist,
  Implementer, etc.) for quick visual identification.
- **Message coloring**: By severity level.
- **Scrollable**: Supports scroll-back with scrollbar widget.
- **Total count**: Shown in title bar: `Logs (247)`.

No text filtering (no `/search`). No export. No level filtering.


## 11. Mori's Overall Error Philosophy

Mori treats errors as **first-class data** in multiple layers:

| Layer | Purpose | Format |
|-------|---------|--------|
| TUI error digest | Operator sees grouped errors | Visual tree |
| GateResult.error_digest | Agent gets focused errors | Text |
| Discovered patterns | Cross-agent learning | JSON |
| Iteration memory | Per-plan failure history | JSON |
| Reflections | LLM-generated analysis | Markdown |
| AutoFix | Machine-applicable fixes | cargo fix |
| Issue plans | Structured escalation docs | Markdown |
| Event log | Audit trail | JSONL |

The hierarchy: detect -> classify -> display -> analyze -> fix -> learn.


---

## 12. Roko's Current State

### What Roko has (strengths)

**Doctor diagnostics** (`doctor.rs`):
~20 named checks with Ok/Warn/Fail/Skipped status:
- workdir validation, config presence, layout basics
- Claude CLI availability
- Provider API keys (per configured provider)
- Provider usability, available providers, default model
- Rust/Node version checks
- Serve auth, serve health
- Dead conductor config detection
- State layout audit, config freshness
- Harness providers, MCP allowlist
- Orphaned tmp files, plans dir conflicts
- Disk health (free space, stale targets, worktrees, oversized JSONL)
- Target staleness

Each check has: id, status, message, optional detail, optional path/url,
optional actionable fix command. Summary: `N ok, N warn, N fail, N skipped`.

**Gate failure classification** (`roko-gate`):
- `classify_gate_failure()` with `GateFailureClassification` struct
- `CompileError` with error code, file, line, message, suggestion
- `parse_cargo_json()` structured JSON output parsing
- `records_from_classification()` for pattern store
- `render_failure_classification()` for human-readable output

**Error pattern store** (`roko-learn`):
- `ErrorPatternStore` with `GateFailureObservation` records
- Error keys (E0425::src/main.rs) for deduplication
- Integration with gate dispatch in the event loop

**Post-gate reflections** (`roko-learn`):
- `PostGateReflectionStore` with durable reflection records
- `ReflectionGateOutcome` (Passed/Failed)
- `ReflectionAdmissionStatus` (Candidate/Admissible/Admitted/Rejected)
- Playbook candidate extraction from repeated patterns
- Bounded evidence accumulation (max 10 items, 160 chars each)

**Agent crash classification** (`agent_exec.rs`):
- `AgentCrashClass` enum: AuthenticationError, RateLimited, ContextOverflow,
  ModelNotFound, NetworkError, Unknown
- Each variant has `is_retriable()` and `recovery_hint()` methods
- Well-tested with unit tests for each classification

**TUI error digest** (`tui/widgets/error_digest.rs`):
- Gate pass/fail ratio header with percentage
- Recent failed gates list (plan_id/task_id/gate)
- Recent errors list with timestamps
- All driven from `DashboardSnapshot` data

**Output sink** (`runner/output_sink.rs`):
- `RunOutputSink` trait with structured events
- `gate_result()`, `task_failed()`, `agent_error()` callbacks
- Multiple implementations: StderrSink, FormattedStderrSink, FanOutSink,
  AcpProgressSink

**Conductor diagnosis** (`roko-core`):
- `DiagnosisSummary` and `DiagnosisSeverity` types
- Dashboard event for pushing diagnoses to TUI/SSE

**Learning infrastructure**:
- Episode logger records gate verdicts per task
- Playbook store queries inject learned patterns into dispatch prompts
- Cascade router learns from provider outcomes
- Efficiency events track per-turn metrics


### What Roko is missing (gaps)

#### 12a. No user-initiated recovery actions in TUI

Mori has `s:retry`, `z:diag`, `S/R:repair`, `c:reverify` as keybindings
that the operator can invoke from the TUI during a live run. Roko's TUI
(`roko dashboard`) is read-only -- there are no interactive recovery actions.
The only way to intervene is to stop the run and restart with `--resume-plan`.

#### 12b. No preflight DX checks

Roko's `doctor` checks for provider keys, toolchain versions, and disk space,
which is better than Mori's preflight. But these checks are only available
as a separate `roko doctor` command, not integrated into the plan-run startup
path. If you run `roko plan run` with a broken config, you discover the
problem when the first agent fails, not at startup.

#### 12c. No LLM-generated reflections on gate failures

Mori spawns a background Haiku call to analyze every gate failure and produces
structured "what failed / why / what to try differently" reflections. Roko
has `PostGateReflectionStore` but it uses pattern-matching and evidence
accumulation rather than LLM analysis. The Mori approach produces more
immediately actionable guidance for retry attempts.

#### 12d. No express-mode auto-fix

Mori has a dedicated `AutoFixer` agent role and `apply_rustc_fixes()` for
machine-applicable compiler suggestions. Roko's gate failure handling triggers
a replan cycle (`build_gate_failure_plan_revision`) but does not attempt
automatic `cargo fix` application as a first-pass repair.

#### 12e. No iteration memory injected into retry prompts

Mori's `IterationMemory` stores per-plan failure history and formats it as
markdown for injection into the agent's next-attempt prompt. Roko has
`ErrorPatternStore` and `PostGateReflectionStore` but the connection from
those stores into the system prompt assembly for retry attempts is less
explicit.

#### 12f. Log filtering and search

Neither Mori nor Roko has text-based log filtering. Both show a scrollable
log view with severity icons.

#### 12g. No structured issue escalation

Mori's `generate_issue_plan()` creates a formal document when an agent
exhausts retries, feeding into a research -> fix -> implement pipeline.
Roko has gate failure replan but no structured escalation document for
stuck tasks.

#### 12h. Tracing levels not easily surfaced to operators

Roko uses `tracing` (via `error!`, `warn!`, `info!`, `debug!`) extensively
in the event loop. But these are only visible in the process's stderr/log
output, not in the TUI. The TUI's error digest reads from the
`DashboardSnapshot` which is updated via `StateHub` events, not from raw
tracing output.


---

## 13. Recommendations for Making Roko Debuggable by Claude Agents

When roko self-hosts (agents running agents), the debugging tool is another
Claude agent. These recommendations are ordered by impact:

### R1. Structured gate failure context in retry prompts (HIGH)

When a task fails a gate and is retried, the retry prompt should include:

```markdown
## Previous Attempt Failures

### Attempt 1 (compile gate FAIL)
3 unique errors:
- error[E0308]: mismatched types at crates/roko-foo/src/bar.rs:42
- error[E0432]: unresolved import at crates/roko-foo/src/lib.rs:3

Files changed: crates/roko-foo/src/bar.rs, crates/roko-foo/src/lib.rs

### What to do differently
- The import was removed in a recent refactor; use the new path
- Do NOT repeat the same approach
```

This is what Mori's IterationMemory + Reflection provides. Roko has the data
(error patterns, reflections) but needs to wire it into `dispatch_agent_with`
for retry attempts.

**Where to wire**: `crates/roko-cli/src/runner/event_loop.rs` in the retry
continuation path, before building the dispatch prompt. Read from
`ErrorPatternStore` and `PostGateReflectionStore`, format as markdown, inject
into the system prompt's gate feedback section.

### R2. Preflight check at plan-run startup (HIGH)

Run a subset of `roko doctor` checks before `roko plan run` begins:

1. Config loads without error
2. At least one provider key is set
3. The default model resolves to a known provider
4. Disk space is adequate (>1GB free)
5. Working directory is a valid git repo with clean-enough state

Report these as a startup block with actionable fix messages. Do not proceed
if any are FAIL.

**Where to wire**: At the top of `run_plan_v2()` in `event_loop.rs`, before
the executor enters its main loop.

### R3. Machine-readable error output for agent consumption (HIGH)

When `roko plan run` is invoked by a Claude agent (the self-hosting case),
the agent needs structured error output it can parse. The
`FormattedStderrSink` currently uses ANSI-colored text. Add a
`--output-format json` flag that switches to `AcpProgressSink`-style
machine-readable output:

```json
{"type":"gate_fail","plan":"p1","task":"t1","gate":"compile","errors":[
  {"code":"E0308","file":"src/bar.rs","line":42,"message":"mismatched types"}
]}
```

This is already partially wired (AcpProgressSink exists). Ensure it emits
gate failure details with structured error classification.

### R4. `roko diagnose <plan-id>` CLI command (MEDIUM)

A non-TUI equivalent of Mori's `z:diag` that an agent can invoke:

```bash
roko diagnose plans/my-plan/
```

Output: a structured report covering:
- Plan phase and failure reason
- Worktree git state (branch, divergence, uncommitted changes)
- Last gate output summary (classified errors)
- Suggested recovery action (retry, repair, reverify, skip)
- Relevant log lines from the last N minutes

This command does not exist. It would be the primary debugging tool for
a supervisor agent running `roko plan run` as a subprocess.

### R5. Error classification in state snapshot (MEDIUM)

The state snapshot (`state-snapshot.json`) should include per-task error
classification when a task fails:

```json
{
  "tasks": {
    "t1": {
      "status": "failed",
      "error_class": "compile",
      "error_code": "E0308",
      "error_file": "src/bar.rs",
      "error_line": 42,
      "error_message": "mismatched types",
      "attempts": 3,
      "last_failure_ts": "2026-08-19T12:00:00Z"
    }
  }
}
```

This lets a supervisor agent read the snapshot and decide what to do without
re-running gates.

### R6. `cargo fix` as a first-pass gate-failure handler (MEDIUM)

Before retrying with a full agent dispatch, try:

1. `cargo fix --allow-dirty --workspace` for the affected crates
2. `cargo fmt` to clean up
3. Re-run the compile gate
4. If it passes, skip the agent retry entirely

This is what Mori's express-mode auto-fix does. Many compile errors
(unused imports, missing derives, simple type mismatches) have
machine-applicable suggestions.

### R7. Gate failure event log (LOW)

Append gate failures to `.roko/gate-failures.jsonl` with:

```json
{
  "ts": "2026-08-19T12:00:00Z",
  "plan_id": "p1",
  "task_id": "t1",
  "gate": "compile",
  "attempt": 2,
  "error_count": 3,
  "error_digest": "E0308 at src/bar.rs:42, E0432 at src/lib.rs:3",
  "duration_ms": 4500
}
```

This gives a supervisor agent a single file to tail for understanding
what's going wrong across all plans.

### R8. Interactive TUI recovery keybindings (LOW)

Add Mori-equivalent keybindings to `roko dashboard`:

- `r`: Retry selected plan (resets failed tasks, preserves completed)
- `c`: Reverify selected plan (re-run gates only)
- `z`: Diagnose selected plan (write report to disk)

These are LOW priority because the self-hosting case uses CLI commands,
not TUI interaction. But they help human operators during development.

### R9. Structured error output in `roko status` (LOW)

`roko status` currently reports counts and episodes. Add a `--errors` flag
that shows the most recent gate failures with classified error summaries:

```
Recent failures:
  [compile] plan-01/task-03 (attempt 2): 3 errors
    E0308 at crates/roko-foo/src/bar.rs:42: mismatched types
    E0432 at crates/roko-foo/src/lib.rs:3: unresolved import
  [test] plan-02/task-01 (attempt 1): 2 failures
    test_foo::test_bar ... FAILED
```


---

## 14. Summary Table

| Capability | Mori | Roko | Gap |
|-----------|------|------|-----|
| Error display (TUI) | 4-source grouped digest | Snapshot-based gate/error list | Roko simpler but functional |
| Error classification | CompileErrorClass (5 variants) | CompileError + GateFailureClassification | Roko is at parity |
| Agent crash classification | None formalized | AgentCrashClass (6 variants) | Roko is ahead |
| Preflight checks | 3 DX checks (startup) | ~20 doctor checks (separate cmd) | Mori inline; Roko more thorough but not inline |
| Gate failure to agents | Error digest injection | Error pattern store + reflection store | Roko has data; wiring to prompts less explicit |
| LLM reflection on failure | Haiku-generated analysis | Pattern-matching reflections | Mori more immediately actionable |
| Iteration memory | JSON per-plan, injected to prompt | PostGateReflectionStore + ErrorPatternStore | Roko has richer infra; less direct prompt injection |
| Auto-fix (cargo fix) | Express mode AutoFixer | Not present | Gap |
| Retry (preserve work) | s/r keybinding | --resume-plan flag | Roko functional but less granular |
| Repair (worktree) | S/R keybindings | Recovery engine exists | Roko has code; not operator-accessible |
| Reverify (gates only) | c keybinding | Not exposed | Gap |
| Diagnose (report) | z keybinding | roko doctor (different scope) | Gap: no plan-specific diagnosis |
| Issue escalation | generate_issue_plan() | Gate failure replan | Roko replan is equivalent but less documented |
| Log view | Scrollable, role-colored | TUI log view exists | Near parity |
| Discovered patterns | Cross-agent JSON store | ErrorPatternStore | Comparable |
| Structured output for agents | Not designed for | AcpProgressSink exists | Roko is ahead in concept |
