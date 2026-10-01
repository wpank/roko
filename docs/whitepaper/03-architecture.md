Status: reviewed · budget 700 words · owner gap-4161ea

# 3 Architecture and control stack

Roko has two parts: an execution path that turns a plan into verified, recorded results, and a control stack
designed to regulate that path from its records (Figure 1). Every tag here comes from the appendix's status matrix,
pinned at `41228d7b2`, and names its row, such as EX3.

## 3.1 From plan to recorded result

A plan is a `tasks.toml`: tasks, their dependencies, the files each may touch, the context to load, and the shell
commands that verify each task. `plan_to_graph` (`crates/roko-graph/src/convert.rs`) compiles it into a Graph, a DAG
of Cells, which the Graph engine (`crates/roko-graph/src/engine.rs`) runs and checkpoints, so an interrupted run
resumes where it stopped (WIRED@41228d7b2: EX1, EX2). The engine has been the only plan executor since Runner-v2 was
deleted in `6b5da8616`. Each task is one executor node; the 11-node per-task subgraph behind `--rich-topology` adds
enrichers that pass their input through unchanged.

A task starts once its own dependencies settle, a failure blocks only its dependants (`445a60d0d`, `3e7552acd`), and
tasks whose declared files overlap never run together. Since `626e182a9` a plan that omits `max_parallel` runs as wide
as its graph when its tasks declare their files (WIRED@41228d7b2, EX3).

For each task, `GraphTaskDispatcher` (`crates/roko-cli/src/graph_task_dispatch.rs`) renders the declared files and
symbols into the prompt (`render_declared_context`), picks a model rung by the task's role and tier, runs an agent (a
CLI such as Claude Code or Codex, or an API model) in an environment without roko's provider keys, screens its diff
for tampering, runs the task's verify commands, and retries a failure with the parsed gate output, one rung up after
two failures blamed on the agent (WIRED@41228d7b2: AU4, RC1, IS5, QA4, QA1, EX6, EX7). Three parts are
PARTIAL@41228d7b2: the learned `CascadeRouter` picks only where no ladder rung can run (RC2); the dashboard still
counts an `Unverified` task as passed (QA2, bug-7e1b6b); and tasks share the operator's checkout unless per-task
worktrees are requested (IS1, gap-4ec59f). Under those, passed attempts land on plan branches that merge into a batch
branch under a whole-plan check (WIRED@41228d7b2: IS2, IS3).

Operators watch runs through a TUI, an HTTP API with SSE on port 6677 (`crates/roko-serve/`) and the portal
(`apps/portal/`), all WIRED@41228d7b2 (SS4); the CLI starts runs, and ACP (`crates/roko-acp/`) serves editors. Pause,
retry and skip are BROKEN@41228d7b2 (SS5, bug-8208a6).

## 3.2 The control stack

The stack is designed to sense each attempt, compare it with what should have happened, act to close the gap, and
audit its own corrections, keeping the essential variables of §2 in bounds.

| Layer | Target design | Today |
|---|---|---|
| Feedforward | A spec-quality gate before dispatch; recurring failures become plan lints | Plan lints WIRED@41228d7b2 (AU2); the spec-quality lint PARTIAL@41228d7b2, opt-in and not yet a gate (AU3, spec-e57870) |
| Sensors | One settled record per attempt: verdict, executed model, cost with its source, routing decision | Telemetry WIRED@41228d7b2 (RG1); attempt records and costs PARTIAL@41228d7b2 (LM1, RC6): the record has no cost, cost rows no source, and there is no decision log (spec-b7303f) |
| Comparators | The visible verify, plus checks the agent cannot see or edit: hidden tests, a tamper and scope diff | The visible verify WIRED@41228d7b2 (QA1), with pinned acceptance tests (AU7) and a tamper and scope diff (QA4); hidden tests MISSING@41228d7b2 (QA5; spec-6ac537) |
| Fast regulators, within a run | Retry with feedback, escalate one model rung, then split or replan | Retry, escalation and a stall watchdog WIRED@41228d7b2 (EX6, EX7, EX9); split or replan BUILT-UNWIRED@41228d7b2 (EX8) |
| Slow regulators, across runs | Routing learned from verified outcomes; keyed failure memory; a bounded controller (M1) | The loops below; failure memory PARTIAL@41228d7b2 (LM4); M1 MISSING@41228d7b2 (RG3); spec-6ac537 |
| Second-order audits | Random deep audits of green results (M4); per-loop exposure, influence and benefit (M2); guarded commit with rollback | MISSING@41228d7b2 (QA5, RG4, RG6; spec-6ac537) |

Four slow regulators close their loops and are WIRED@41228d7b2: the provider-health circuit breaker (RC3); the plan
budget, which since `abc1f4b27` starts no task once spent (RC5); retry budgets set from each gate rung's pass rate
since `99adacd6d` (QA7); and playbook credit (LM2). §5.2 tags the other loops; knowledge retrieval among them is
BROKEN@41228d7b2 (LM3, bug-86117a). No loop has a measured benefit: the audit that would measure one is
MISSING@41228d7b2 (RG4). §5 describes the mechanisms designed for the missing layers, M1 to M4.

## 3.3 What Roko records

The Graph host and `crates/roko-cli/src/runtime_feedback/` write these records:

| Record | Contents | Caveat that still holds |
|---|---|---|
| `.roko/state/graph/<plan>/` | Checkpoint, activity log and spend | Reliable; resume reads it |
| `.roko/episodes.jsonl` | One record per agent turn | Only a failed verify writes a verdict (since `33e107da1`) |
| `.roko/runs/<run>/` | A manifest with a config fingerprint, and one verdict record per attempt naming the model that served it (since `9313e49f0`, `6f8286d48`) | The records carry no tokens or cost yet |
| `.roko/learn/efficiency.jsonl` | Tokens, cost and gate events | No cost source; since `d4be4e872` a timed-out attempt keeps its streamed usage |
| `.roko/learn/cascade-router.json` | Router state | Learns from settled verdicts since `74eaf5c8f`, but the ladder places most tasks (RC2) |
| `.roko/learn/gate-thresholds.json` | Pass-rate averages per gate rung | Sets retry budgets since `99adacd6d` |

This paper does not repeat two headline claims of `docs/v3/00-INDEX.md`: "a Graph of Graphs is just a Graph"
(nothing implements `Cell` for `Graph`; gap-cdf3fc) and "every Cell is a learner" (the plan-task Cell keeps the
default `predict`, which returns nothing).

![Figure 1: architecture diagram](figures/fig1-architecture.svg)

**Figure 1:** The architecture and the control stack. The marks follow the status matrix at `41228d7b2`; §3.2 names
each row.
