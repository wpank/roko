# Mori/Bardo Workflow Parity Audit — Queue, DAG, Parallelism, Worktrees, Merge, Recovery

**Audit date:** 2026-09-01  
**Reference implementation:** `/Users/will/dev/uniswap/bardo/apps/mori/`  
**Roko baseline:** current `main`; Runner-v2 is the active plan runner, while GraphEngine remains a separate execution path.

## Why this audit exists

The important Mori UX was not merely a TUI appearance. Mori made a large body of work feel like
one safe, resumable workflow: edit a queue manifest, let the scheduler compute runnable DAG work,
run independent work in parallel, isolate every agent in a worktree, serialize integration through
a merge queue, preserve evidence and state across interruption, and advance to the next milestone
without asking the operator to construct a long sequence of shell commands.

Roko currently has several similarly named modules. This audit therefore distinguishes three
different states:

* **Implemented:** production code exists on a named path.
* **Partial:** some pieces exist, but a Mori workflow contract is missing or only one engine/path is wired.
* **Unproven/missing:** the behavior is not on the execution path or lacks the required end-to-end proof.

Compilation, a parser, a TUI widget, or a checked backlog box is not sufficient evidence of workflow parity.

## Mori workflow contract observed in source

The reference behavior is distributed across these Mori modules:

| Workflow concern | Mori source | Operator-visible contract |
|---|---|---|
| Queue and milestones | `orchestrator/queue.rs`, `orchestrator/dag.rs` | `.mori/queue.toml` names ordered milestones, plan groups, dependencies, maintenance plans, and run overrides. With no plan arguments, Mori selects the current incomplete milestone; `--milestone` selects one explicitly. |
| DAG scheduling | `orchestrator/unified_dag.rs`, `orchestrator/executor.rs`, `app/sequential.rs`, `app/parallel.rs` | Dependency-ready work is scheduled automatically; independent plans/tasks run concurrently; completion unlocks downstream work. |
| Parallel agent pool | `app/parallel.rs`, agent pool modules | Multiple plans and multiple agents can run at once with bounded concurrency and warm reusable connections. |
| Workspace isolation | `git/worktree.rs`, `app/sequential.rs`, `app/parallel.rs` | Each attempt has a distinct worktree/branch; agents do not write into the shared checkout. |
| Integration safety | `git/merge_queue.rs`, `git/mod.rs`, `app/*` | Completed branches enter a serialized merge path with conflict handling and post-merge verification. |
| Recovery/idempotency | `state/persistence.rs`, `orchestrator/paths.rs`, `app/sequential.rs` | Runtime state, task status, completion tags, and artifacts survive restart; completed work is skipped or recovered rather than blindly repeated. |
| Operator checkpoints | `orchestrator/batch.rs`, TUI actions, `app/*` | `batch_size` pauses after N completed plans; the operator inspects results and resumes without rebuilding the run. |
| Context/artifacts | `orchestrator/inject.rs`, `orchestrator/context.rs`, `orchestrator/memory.rs` | Plan/task/dependency/review/verification artifacts are injected into isolated attempts and shared context is kept coherent across plans. |
| Failure handling | `orchestrator/autofix.rs`, `orchestrator/review.rs`, `orchestrator/gates.rs`, `orchestrator/reflection.rs` | Gate failures are classified, auto-fixed/retried when safe, and retained as evidence when not. |

The resulting UX is effectively: **queue → eligible DAG work → isolated parallel attempts → gates →
serialized merge → durable completion → next queue milestone**.

## Roko parity assessment

| Mori capability | Roko evidence | Status | Backlog/proof owner |
|---|---|---|---|
| Persistent queue manifest drives execution | `runner/queue_manifest.rs`; `roko plan queue show/validate/init` | **Partial** — parsing and inspection exist, but `plan run` does not consume `.roko/queue.toml`, select the current milestone, or apply its run/model overrides. | [#116](../backlog/116-queue-manifest.md), acceptance criteria 1–2 and 6–7 |
| Milestone sequencing and maintenance batches | Queue type has milestones, but no runner milestone unlock/maintenance execution path | **Missing/unproven** | [#116](../backlog/116-queue-manifest.md) |
| Plan-level DAG waves | `runner/plan_dag.rs`, plan list wave support, wave-oriented TUI | **Partial** — wave computation/visualization exists, but it is not the same as queue milestones and does not establish the full Mori queue contract. | [#117](../backlog/117-plan-wave-computation.md), #125 |
| Automatic runnable-work scheduling | Runner-v2 has task capacity and internal pending queues | **Partial** — within-run scheduling exists; queue-driven selection across a persistent batch is absent. | #116, #272 |
| Parallel plans/agents | Runner-v2 has bounded task concurrency; Graph has a separate plan-queue design | **Partial** — concurrency exists on the active runner, but cross-run Graph queue isolation and parallel-plan proof are blocked. | [#272](../backlog/272-parallel-plan-queues.md) |
| Per-attempt worktrees | Runner-v2 has `orchestrator/worktree.rs` and attempt ownership; Graph path is explicitly shared-root today | **Partial by engine** — Runner-v2 isolation exists but needs live proof; Graph isolation is a blocked implementation item. | [#249](../backlog/249-graph-worktree-attempt-lifecycle.md), #274 |
| No git races or lost work under parallelism | `runner/merge.rs` has `PlanMerger`/`MergeQueue` | **Unproven** — no deterministic end-to-end success, conflict, regression, and recovery proof. | [#140](../backlog/140-merge-success-conflict-proof.md), #254 |
| Serialized merge and post-merge regression | Runner-v2 merge path and regression gate are present | **Partial** — source exists, but the acceptance evidence is absent and Graph/HTTP paths have separate integration concerns. | #140, #256 |
| Crash/resume without duplicate side effects | Runner snapshots, resume, ownership and ledgers exist | **Partial/unproven** — the required process-level kill-point matrix is still blocked. | [#138](../backlog/138-crash-resume-proof-matrix.md), #284, #326 |
| Pause after N plans | `--batch-size` and event types have since landed | **Partial** — the parity audit still needs live pause/resume behavior, snapshot, and TUI/headless evidence. | [#179](../backlog/179-batch-controller.md) |
| Queue-level model/provider override | `RunOverrides.model` is parsed in `queue_manifest.rs` | **Unproven** — no execution call site applies the value; CLI force-model is separate and works only per invocation. | #116, #262 |
| Context/artifact injection into attempts | Roko prompt/context assembly and worktree injection paths exist | **Partial** — parity needs a cross-plan artifact/context fixture proving the same inputs are available in every isolated attempt. | #3, #67, #242 |
| Failure classification, retry, auto-fix, evidence | Runner gate dispatch and replan/autofix paths exist | **Partial** — agent timeout can terminate before gates, and live proof of safe retry/auto-fix convergence remains incomplete. | #15, #166, #204, #218, #286 |
| Operator workflow/TUI truthfulness | Queue modal, plan waves, status, logs, and controls exist | **Partial** — queue modal derives from execution waves rather than `queue.toml`; several controls and live evidence remain incomplete. | #216, #233, #234, #327 |

## Findings that must not be collapsed into one “queue” item

1. **Queue manifest is a selection/orchestration contract.** It answers which plans are eligible,
   in what milestone, with which run settings. A task queue or merge queue cannot substitute for it.
2. **DAG waves are not milestones.** Waves derive ordering from dependency edges; milestones encode
   operator intent, batch boundaries, maintenance associations, and a resumable roadmap.
3. **Parallel execution is not safe integration.** Agent concurrency must be paired with distinct
   worktrees, attempt identity, file-overlap handling, serialized merge/publish, and regression gates.
4. **Runner-v2 and GraphEngine must be assessed separately.** Runner-v2 has more of the Mori-shaped
   lifecycle; Graph has explicit blocked work for workspaces, run queues, control, and crash proof.
5. **“Source complete” is not “workflow proven.”** The minimum evidence is a multi-plan fixture with
   independent and dependent work, overlapping files, a merge conflict, cancellation/restart, and a
   second invocation that advances the queue rather than repeating completed work.

## Required parity evidence

The following fixture should be added to the audit/evidence lane before declaring Mori workflow parity:

1. A `.roko/queue.toml` with two milestones, at least two independent plans in milestone one, and a
   dependent plan in milestone two.
2. A Runner-v2 run using the queue manifest and a forced provider/model override; assert only the
   eligible milestone starts and the override reaches dispatch.
3. Two independent plans execute concurrently in distinct attempt worktrees; a third waits on their
   dependency. Assert no shared-checkout writes.
4. Two plans with disjoint files merge successfully; two plans with an intentional same-line conflict
   produce structured conflict evidence and leave the integration branch recoverable.
5. Kill/restart at provider, gate, merge, and completion boundaries; assert no duplicate committed
   effects and that the next invocation advances from durable completion state.
6. Pause after a small batch, inspect status/TUI, resume, and assert the pause is durable and counted.

These are acceptance/proof requirements, not claims that the current implementation already passes them.

## Backlog coverage conclusion

The core gaps are already represented by #116 (queue execution), #117 (plan waves), #179 (batch
pause), #140 (merge proof), #138/#284/#326 (crash/resume), #249 (Graph workspaces), and #272
(parallel plan queues). The audit adds the missing cross-cutting requirement: these items must be
verified together as one operator workflow, with Runner-v2/Graph distinctions preserved. No new
feature should be marked complete solely because `queue_manifest.rs`, a queue modal, or an internal
merge queue exists.

