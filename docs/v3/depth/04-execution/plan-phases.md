# Plan Phases

> Depth file for [04-EXECUTION.md](../../04-EXECUTION.md) section 7.
> Preserves and updates content from v1 `01-orchestration/04-plan-phases.md`.

---

## Overview

Every plan progresses through a defined sequence of phases. The phase
lifecycle encodes the entire plan execution workflow: enrichment,
implementation, quality gating, verification, code review, documentation,
and merge. It includes bounded retry loops for gate failures and review
rejections.

**Source:** `crates/roko-core/src/plan_phase.rs`

---

## Phase Definitions

### Active phases

| Phase | Description | Agent role |
|---|---|---|
| `Queued` | Waiting in the execution queue | -- |
| `Enriching` | Context enrichment (knowledge, episodes, playbooks) | Strategist |
| `Implementing` | Agents executing tasks | Implementer |
| `Gating` | Quality gates running (compile, test, clippy) | -- |
| `AutoFixing` | Fixing gate failures | AutoFixer |
| `Verifying` | Task-level verification commands running | -- |
| `RegeneratingVerify` | Regenerating failed verification code | AutoFixer |
| `Reviewing` | Code review | Auditor |
| `DocRevision` | Documentation update | Scribe |
| `Merging` | Worktree branch being merged | -- |

### Terminal phases

| Phase | Description |
|---|---|
| `Complete` | Plan merged successfully |
| `Failed { reason }` | Plan failed terminally (with `FailureKind`) |
| `Skipped` | Plan skipped by operator |

Terminal phases are absorbing: once reached, no further transitions occur
within a single execution run.

---

## Phase Transition Diagram

```
                                         +------------------+
                                         |     Queued        |
                                         +--------+---------+
                                                  | Start
                                                  v
                                         +------------------+
                                         |    Enriching      |
                                         +--------+---------+
                                                  | EnrichmentDone
                                                  v
                                +-----------------------------------+
                        +------>|          Implementing              |<------+
                        |       +----------+------------------------+       |
                        |                  | ImplementationDone              |
                        |                  v                                |
                        |       +------------------+                        |
                        |       |     Gating        |<--------+             |
                        |       +--+----------+----+          |             |
                        |          |          |               |             |
                        |     GatePassed  GateFailed          |             |
                        |          |          |               |             |
                        |          |          v               |             |
                        |          |  +--------------+        |             |
                        |          |  |  AutoFixing   |       |             |
                        |          |  +------+-------+        |             |
                        |          |         | AutoFixDone    |             |
                        |          |         +----------------+             |
                        |          v                                        |
                        |  +------------------+                             |
                        |  |    Verifying      |<---------+                 |
                        |  +--+----------+----+           |                 |
                        |     |          |                |                 |
                        | VerifyPassed VerifyFailed       |                 |
                        |     |          |                |                 |
                        |     |          v                |                 |
                        |     |  +------------------+    |                  |
                        |     |  |RegeneratingVerify|    |                  |
                        |     |  +------+-----------+    |                  |
                        |     |         | VerifyRegenDone |                 |
                        |     |         +----------------+                  |
                        |     v                                             |
                        |  +------------------+                             |
                        |  |    Reviewing      |---- ReviewRejected --------+
                        |  +--------+---------+
                        |           | ReviewApproved
                        |           v
                        |  +------------------+
                        |  |   DocRevision     |
                        |  +--------+---------+
                        |           | DocRevisionDone
                        |           v
                        |  +------------------+
                        |  |     Merging       |
                        |  +--+----------+----+
                        |     |          |
                        |  Succeeded   Failed
                        |     |          |
                        |     v          v
                        |  +--------+  +--------+
                        |  |Complete|  | Failed  |
                        |  +--------+  +--------+
                        |
                        |  (Skip from any non-terminal -> Skipped)
                        |  (Fatal from any non-terminal -> Failed)
                        +--------------------------------------
```

---

## Transition Rules

| From | Event | To | Notes |
|---|---|---|---|
| Queued | Start | Enriching | Plan begins |
| Queued | Skip | Skipped | Operator skip |
| Enriching | EnrichmentDone | Implementing | Context ready |
| Implementing | ImplementationDone | Gating | All tasks done |
| Gating | GatePassed | Verifying | All gates passed |
| Gating | GateFailed (iter < 5) | AutoFixing | Retry available |
| Gating | GateFailed (iter >= 5) | Failed(AutoFixExhausted) | Max retries |
| AutoFixing | AutoFixDone | Gating | Re-run gates |
| Verifying | VerifyPassed | Reviewing | Verification passed |
| Verifying | VerifyFailed | RegeneratingVerify | Verification failed |
| RegeneratingVerify | VerifyRegenDone | Verifying | Re-verify |
| Reviewing | ReviewApproved | DocRevision | Auditor approved |
| Reviewing | ReviewRejected | Implementing | Re-implement |
| DocRevision | DocRevisionDone | Merging | Docs updated |
| Merging | MergeSucceeded | Complete | Success |
| Merging | MergeFailed (< 3 attempts) | Failed | Merge conflict |
| Merging | MergeFailed (>= 3) | Failed(Deadlock) | Deadlock |
| Any non-terminal | Skip | Skipped | Operator skip |
| Any non-terminal | Fatal(reason) | Failed(Other(reason)) | Crash |

Illegal transitions (e.g., trying to gate-pass from Queued) produce a
`TransitionError` with the source phase, target phase, and a diagnostic
message. This indicates a bug in the controller, not a normal failure.

---

## Bounded Retry Loops

Two retry loops have explicit compile-time bounds:

### Auto-fix loop

`Gating -> AutoFixing -> Gating`, maximum 5 iterations
(`MAX_AUTO_FIX_ITERATIONS`).

When a gate fails (compilation error, test failure, clippy warning), an
auto-fixer agent receives the gate failure output and attempts to fix
the issues. The fixed code is re-gated. After 5 failed cycles, the plan
transitions to `Failed { reason: AutoFixExhausted }`.

The auto-fixer agent receives progressively more context with each
iteration: the original task description, all prior gate failure outputs,
and the diff of changes made so far. The cascade router may escalate to
more capable models on higher iterations.

### Merge retry

`Merging -> Failed`, maximum 3 attempts (`MAX_MERGE_ATTEMPTS`).

When a merge fails (file conflict), the request is re-enqueued in the
merge queue with reduced priority. After 3 failed merges, the plan
transitions to `Failed { reason: Deadlock }`.

These bounds prevent infinite loops. The values are compile-time constants
-- they represent hard safety limits, not tunable parameters.

---

## Failure Types

```rust
pub enum FailureKind {
    AutoFixExhausted,   // 5 gate-fix cycles without passing
    Deadlock,           // 3 merge attempts without success
    Other(String),      // arbitrary failure reason
}
```

`FailureKind` is serialized into snapshots and event logs for post-mortem
analysis. The `roko diagnose <plan-id>` command reads these to produce
structured failure reports.

---

## Agent Roles per Phase

| Role | Phase | Purpose | Model tier |
|---|---|---|---|
| Strategist | Enriching | Context gathering, plan validation | Standard |
| Implementer | Implementing | Execute tasks from the plan | Task-dependent |
| AutoFixer | AutoFixing | Fix compilation/test failures | Escalating |
| AutoFixer | RegeneratingVerify | Regenerate verification code | Standard |
| Auditor | Reviewing | Review implementation correctness | Standard |
| Scribe | DocRevision | Update documentation | Fast |

Each role receives a different system prompt, tool set, and model tier
through `RoleSystemPromptSpec` and the cascade router.

---

## Phase-to-Graph Mapping

In the Graph engine, plan phases correspond to groups of graph nodes in
the production topology:

| Phase | Graph nodes | Execution class |
|---|---|---|
| Enriching | context, knowledge, episodes, playbook, modulation, safety, experiment | Workflow |
| Implementing | compose, executor | Activity |
| Gating | gate | Activity |
| Verifying | (controller-side) | -- |
| Reviewing | (controller-side) | -- |
| DocRevision | (controller-side) | -- |
| Merging | (controller-side) | -- |

The first three phases are expressed as graph nodes. The remaining phases
(verification, review, documentation, merge) are controller-side actions
that run outside the graph. This split exists because the later phases
operate on the execution context (worktrees, merge queue) rather than on
task-level data.

---

## Mapping from Mori Pipeline

The original Mori orchestrator defined:

```
Preflight -> Strategist -> Implementer -> Gates -> Review -> Verdict
```

The current lifecycle is more granular:

| Mori Phase | Current Phase | Notes |
|---|---|---|
| Preflight | Enriching | Context gathering |
| Strategist | Enriching | Merged into enrichment |
| Implementer | Implementing | Agent task execution |
| Gates | Gating + Verifying | Split into gate ladder and verification |
| Review | Reviewing | Auditor review |
| Verdict | DocRevision + Merging | Split into doc update and merge |

The split of Gates into Gating + Verifying separates the generic quality
gates (compile, test, clippy) from task-specific verification commands
declared in `tasks.toml`. The split of Verdict into DocRevision + Merging
adds an explicit documentation update step before integration.
