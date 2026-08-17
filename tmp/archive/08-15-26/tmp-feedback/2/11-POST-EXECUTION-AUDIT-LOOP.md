# Post-Execution Audit Loop

## Idea

After `roko plan run` completes a batch of tasks, automatically launch audit agents
to review all the work that was done. Auditors find issues, generate new fix/improvement
tasks, and the runner picks those up — repeating until a clean audit pass.

```
plan run (N tasks) → audit agents review → new tasks generated → plan run (M fixes)
       ↑                                                              ↓
       └──────────── repeat until audit passes clean ←────────────────┘
```

## Behavior

### 1. Configurable audit trigger

```toml
# roko.toml
[execution.audit]
enabled = true
after = "wave"              # "wave" | "plan" | "all"
max_iterations = 3           # prevent infinite loops
auditors = ["reviewer", "integration-tester", "clippy-auditor"]
model = "claude-sonnet-4-6"  # or inherit from cascade router
scope = "changed-files"      # "changed-files" | "changed-crates" | "workspace"
```

| `after` value | When audit runs |
|---------------|-----------------|
| `"wave"` | After each parallel wave completes (before starting next wave) |
| `"plan"` | After all tasks in a plan complete |
| `"all"` | After all plans in a batch complete |

### 2. Audit agent dispatch

Each auditor gets:
- The git diff of all changes made in the completed batch
- The task definitions that were executed
- The gate results (compile, test, clippy verdicts)
- Access to read the codebase

Auditor roles and what they check:

| Auditor | Checks |
|---------|--------|
| `reviewer` | Code quality, patterns, missed edge cases, API design |
| `integration-tester` | Cross-crate consistency, public API contracts, import graph |
| `clippy-auditor` | Lint issues, idiomatic Rust, unused code |
| `doc-verifier` | Docstrings match implementation, README accuracy |
| `spec-drift-detector` | Implementation matches PRD/task spec |
| `regression-detector` | Changes that could break existing functionality |

### 3. Task generation from audit findings

Each auditor produces structured findings:

```toml
# Audit output format
[[findings]]
severity = "must-fix"        # "must-fix" | "should-fix" | "nit"
file = "crates/kora-hdc/src/vector.rs"
line = 42
description = "BundleAccumulator::add() doesn't check dimension mismatch"
suggested_fix = "Add assert_eq!(self.dim, other.dim) at entry"
task_title = "Add dimension check to BundleAccumulator::add"

[[findings]]
severity = "should-fix"
file = "crates/kora-hdc/src/constants.rs"
description = "HDC_DIM is 1024 but tests use hardcoded 512"
suggested_fix = "Use HDC_DIM constant in tests instead of magic number"
task_title = "Replace hardcoded dimension in tests with HDC_DIM constant"
```

The runner converts `must-fix` and `should-fix` findings into new `TaskDef` entries,
adds them to the plan, and continues executing. `nit` findings are logged but don't
generate tasks.

### 4. Iteration and convergence

```
Iteration 1: 8 tasks completed → audit finds 3 issues → 3 fix tasks created
Iteration 2: 3 fix tasks completed → audit finds 0 issues → done
```

Convergence rules:
- **Max iterations**: Hard cap (default 3) prevents infinite loops
- **Diminishing returns**: If iteration N finds more issues than N-1, stop (likely systemic)
- **Severity filter**: After iteration 1, only `must-fix` generates new tasks
- **Budget cap**: Audit agents count toward plan budget (`max_plan_usd`)

### 5. CLI flags

```bash
# Run with audit loop
roko plan run plans/ --audit

# Run with audit, custom config
roko plan run plans/ --audit --audit-max-iter=5

# Run audit on already-completed work (standalone)
roko audit plans/ --scope=changed-files

# Dry run — show what auditors would check, don't dispatch
roko audit plans/ --dry-run
```

## Implementation

### Where it hooks in

The audit loop sits between task completion and plan completion in the runner:

```rust
// runner/event_loop.rs — after all tasks in a wave/plan complete
if config.audit.enabled && iteration < config.audit.max_iterations {
    let findings = run_audit_agents(completed_tasks, git_diff, config).await?;
    let fix_tasks = findings_to_tasks(findings, plan_id);

    if fix_tasks.is_empty() {
        info!("audit pass clean — no issues found");
    } else {
        info!("audit found {} issues — adding fix tasks", fix_tasks.len());
        executor.add_tasks(plan_id, fix_tasks)?;
        // Runner continues with new tasks
    }
}
```

### Components needed

| Component | Exists? | What to build |
|-----------|---------|---------------|
| Audit agent dispatch | Partial | Reuse `spawn_agent()` with reviewer role + audit prompt |
| Findings parser | No | Parse structured audit output → `Vec<AuditFinding>` |
| Task generator | Partial | Convert findings → `TaskDef` (reuse `TaskDef::new()`) |
| Git diff collector | Yes | `git diff HEAD~N` already used in gate pipeline |
| Iteration controller | No | Track audit iterations, convergence, budget |
| Config schema | No | `[execution.audit]` section in roko.toml |

### Audit prompt template

```
You are auditing code changes made by other agents. Review the following:

## Changes Made
{git_diff}

## Tasks Completed
{task_summaries}

## Gate Results
{gate_verdicts}

Review for:
1. Correctness — bugs, logic errors, edge cases
2. Consistency — naming, patterns, API contracts across crates
3. Completeness — missing tests, docs, error handling
4. Spec compliance — does implementation match the task description?

Output findings as TOML (see format above). Only report real issues.
Do NOT report style preferences or minor formatting.
```

### Integration with existing subsystems

| Subsystem | How audit uses it |
|-----------|-------------------|
| CascadeRouter | Selects model for auditor agents (reviewer role → high-capability model) |
| Gate pipeline | Audit tasks go through same compile/test/clippy gates |
| EpisodeLogger | Audit agent turns recorded for learning |
| Efficiency events | Audit cost tracked separately (`audit_cost_usd` in run state) |
| Playbook store | Audit patterns saved for future runs |
| Adaptive thresholds | Audit-generated tasks affect gate EMA |

## Example: Full Execution with Audit

```
$ roko plan run plans/hdc-core-crate --audit --audit-max-iter=3

Wave 1: T01-T04 (scaffold, constants, vector, accumulator)
  T01: skip (preflight pass)
  T02: agent → implemented → gates pass ✓
  T03: agent → implemented → gates pass ✓
  T04: agent → implemented → gates pass ✓

Audit iteration 1:
  reviewer: 2 findings (dimension mismatch, missing Display impl)
  integration-tester: 1 finding (vector.rs doesn't re-export from lib.rs)
  clippy-auditor: 0 findings
  → 3 fix tasks added (T09, T10, T11)

Wave 2: T05-T08 + T09-T11 (original + audit fixes)
  T05-T08: agent → implemented → gates pass ✓
  T09: agent → fixed dimension check → gates pass ✓
  T10: agent → added Display impl → gates pass ✓
  T11: agent → fixed re-export → gates pass ✓

Audit iteration 2:
  reviewer: 0 findings
  integration-tester: 0 findings
  → Clean audit pass. Done.

Plan completed: 11 tasks, 2 audit iterations, $2.34
```

## Relation to Existing Features

| Feature | Difference from audit loop |
|---------|---------------------------|
| Gate pipeline | Gates check compile/test/lint. Audit checks semantics, design, spec compliance. |
| Gate failure replan | Replan regenerates the failed task's prompt. Audit generates NEW tasks. |
| Enrichment pipeline | Enrichment adds context BEFORE dispatch. Audit reviews AFTER completion. |
| Playbook queries | Playbooks inform how to do work. Audit checks if work was done correctly. |

The audit loop is the missing piece between "code compiles" and "code is correct."
Gates verify mechanical properties. Auditors verify semantic properties.
