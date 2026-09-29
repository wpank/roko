Status: reviewed · budget 700 words · owner gap-4161ea

# 3 Architecture and control stack

Roko has two parts: an execution path that turns a plan into verified, recorded results, and a control stack
designed to regulate that path from its records (Figure 1). Every tag here comes from the appendix's status matrix,
pinned at `ed0c33bd5`, and names its row, such as EX3.

## 3.1 From plan to recorded result

A plan is a `tasks.toml`: tasks, their dependencies, the files each may touch, the context to load, and the shell
commands that verify each task. `plan_to_graph` (`crates/roko-graph/src/convert.rs`) compiles it into a Graph, a DAG
of Cells, which the Graph engine (`crates/roko-graph/src/engine.rs`) runs and checkpoints, so an interrupted run
resumes where it stopped (WIRED@ed0c33bd5: EX1, EX2). The engine has been the only plan executor since Runner-v2 was
deleted in `6b5da8616`. Each task is one executor node; the 11-node per-task subgraph behind `--rich-topology` adds
enrichers that pass their input through unchanged.

A task starts once its own dependencies settle, a failure blocks only its dependants (`445a60d0d`, `3e7552acd`), and
since `1697fea53` tasks whose declared files overlap never run together. Scheduling is still PARTIAL@ed0c33bd5 (EX3):
`max_parallel` defaults to 1 (gap-272448).

For each task, `GraphTaskDispatcher` (`crates/roko-cli/src/graph_task_dispatch.rs`) renders the declared files and
symbols into the prompt (`render_declared_context`), picks a model, runs an agent (a CLI such as Claude Code or
Codex, or an API model), runs the task's verify commands, and retries a failure with the parsed gate output
(WIRED@ed0c33bd5: AU4, RC1, QA1, EX6). Four parts are PARTIAL@ed0c33bd5: `CascadeRouter` learns from the provider's
success flag, before gates run (RC2, bug-c34782); an `Unverified` verdict still counts as a pass for learning and the
dashboard (QA2, spec-e9d7ec); tasks share the
operator's checkout unless per-task worktrees are requested, and nothing merges those back (IS1, gap-4ec59f); and
since `1d923e377` verify commands start from an allowlisted environment and provider CLIs drop the key variables they
recognise, but agent tool shells and MCP servers still inherit provider keys (IS5, spec-ba7bea).

Operators watch runs through a TUI, an HTTP API with SSE on port 6677 (`crates/roko-serve/`) and the portal
(`apps/portal/`), all WIRED@ed0c33bd5 (SS4); the CLI starts runs, and ACP (`crates/roko-acp/`) serves editors. Pause,
retry and skip are BROKEN@ed0c33bd5 (SS5, bug-8208a6).

## 3.2 The control stack

The stack is designed to sense each attempt, compare it with what should have happened, act to close the gap, and
audit its own corrections, keeping the essential variables of §2 in bounds.

| Layer | Target design | Today |
|---|---|---|
| Feedforward | A spec-quality gate before dispatch; recurring failures become plan lints | Plan lints WIRED@ed0c33bd5 (AU2); the spec-quality lint PARTIAL@ed0c33bd5, opt-in and not yet a gate (AU3, spec-e57870) |
| Sensors | One settled record per attempt: verdict, executed model, cost with its source, routing decision | Telemetry WIRED@ed0c33bd5 (RG1); attempt records and costs PARTIAL@ed0c33bd5 (LM1, RC6): the record has no cost, cost rows no source, and there is no decision log (spec-b7303f) |
| Comparators | The visible verify, plus checks the agent cannot see or edit: hidden tests, a tamper and scope diff | The visible verify WIRED@ed0c33bd5 (QA1), with pinned acceptance tests (AU7); the rest MISSING@ed0c33bd5 (QA4, QA5; spec-9230a9, spec-6ac537) |
| Fast regulators, within a run | Retry with feedback, escalate one model rung, then split or replan | Retry WIRED@ed0c33bd5 (EX6); escalation ORPHANED@ed0c33bd5 (EX7); split or replan BUILT-UNWIRED@ed0c33bd5 (EX8); spec-98f76d |
| Slow regulators, across runs | Routing learned from verified outcomes; keyed failure memory; a bounded controller (M1) | The loops below; failure memory ORPHANED@ed0c33bd5 (LM4); M1 MISSING@ed0c33bd5 (RG3); spec-6ac537 |
| Second-order audits | Random deep audits of green results (M4); per-loop exposure, influence and benefit (M2); guarded commit with rollback | MISSING@ed0c33bd5 (QA5, RG4, RG6; spec-6ac537) |

Four slow regulators close their loops and are WIRED@ed0c33bd5: the provider-health circuit breaker (RC3); the plan
budget, which since `abc1f4b27` starts no task once spent (RC5); retry budgets set from each gate rung's pass rate
since `99adacd6d` (QA7); and playbook credit (LM2). §5.2 tags the other loops; knowledge retrieval among them is
BROKEN@ed0c33bd5 (LM3, bug-86117a). No loop has a measured benefit: the audit that would measure one is
MISSING@ed0c33bd5 (RG4). §5 describes the mechanisms designed for the missing layers, M1 to M4.

## 3.3 What Roko records

The Graph host and `crates/roko-cli/src/runtime_feedback/` write these records:

| Record | Contents | Caveat that still holds |
|---|---|---|
| `.roko/state/graph/<plan>/` | Checkpoint, activity log and spend | Reliable; resume reads it |
| `.roko/episodes.jsonl` | One record per agent turn | Only a failed verify writes a verdict (since `33e107da1`) |
| `.roko/runs/<run>/attempts.jsonl` | One verdict record per attempt, naming the model that ran (since `42349d8ee`) | No tokens or cost yet |
| `.roko/learn/efficiency.jsonl` | Tokens, cost and gate events | No cost source; since `d4be4e872` a timed-out attempt keeps its streamed usage |
| `.roko/learn/cascade-router.json` | Router state | It learns from the provider's success flag, before gates run (bug-c34782) |
| `.roko/learn/gate-thresholds.json` | Pass-rate averages per gate rung | Sets retry budgets since `99adacd6d` |

This paper does not repeat two headline claims of `docs/v3/00-INDEX.md`: "a Graph of Graphs is just a Graph"
(nothing implements `Cell` for `Graph`; gap-cdf3fc) and "every Cell is a learner" (the plan-task Cell keeps the
default `predict`, which returns nothing).

![Figure 1: architecture diagram](figures/fig1-architecture.svg)

**Figure 1:** The architecture and the control stack. The marks follow the status matrix at `ed0c33bd5`; §3.2 names
each row.
