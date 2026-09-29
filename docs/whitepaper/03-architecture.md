Status: draft · budget 700 words · owner gap-4161ea

# 3 Architecture and control stack

Roko has two parts: an execution path that turns a plan into verified, recorded results, and a control stack
designed to regulate that path from its records (Figure 1). Every tag here comes from the appendix's status matrix,
pinned at `a17d4dadd`, and names its row, such as EX3.

## 3.1 From plan to recorded result

A plan is a `tasks.toml`: tasks, their dependencies, the files each may touch, the context to load, and the shell
commands that verify each task. `plan_to_graph` (`crates/roko-graph/src/convert.rs`) compiles it into a Graph, a DAG
of Cells, which the Graph engine (`crates/roko-graph/src/engine.rs`) runs and checkpoints, so an interrupted run
resumes where it stopped (WIRED@a17d4dadd: EX1, EX2). The engine has been the only plan executor since Runner-v2 was
deleted in `6b5da8616`. Each task is one executor node; the 11-node per-task subgraph behind `--rich-topology` adds
enrichers that pass their input through unchanged.

A task starts once its own dependencies settle, and a failure blocks only its dependants (`445a60d0d`,
`3e7552acd`). Scheduling is still PARTIAL@a17d4dadd (EX3): `max_parallel` defaults to 1, and nothing admits parallel
tasks by their write sets (gap-272448).

For each task, `GraphTaskDispatcher` (`crates/roko-cli/src/graph_task_dispatch.rs`) renders the declared files and
symbols into the prompt (`render_declared_context`), picks a model, runs an agent (a CLI such as Claude Code or
Codex, or an API model), runs the task's verify commands, and retries a failure with the parsed gate output
(WIRED@a17d4dadd: AU4, RC1, QA1, EX6). Four parts are PARTIAL@a17d4dadd: `CascadeRouter` learns from successes only
(RC2, bug-8da8ba); an `Unverified` verdict still counts as a pass downstream (QA2, spec-e9d7ec); tasks share the
operator's checkout unless per-task worktrees are requested, and nothing merges those back (IS1, gap-4ec59f); and
since `1d923e377` verify commands and provider CLIs get an allowlisted environment, but agent tool shells and MCP
servers still inherit provider keys (IS5, spec-ba7bea).

Operators watch runs through a TUI, an HTTP API with SSE on port 6677 (`crates/roko-serve/`) and the portal
(`apps/portal/`), all WIRED@a17d4dadd (SS4); the CLI starts runs, and ACP (`crates/roko-acp/`) serves editors. Pause,
retry and skip are BROKEN@a17d4dadd (SS5, bug-8208a6).

## 3.2 The control stack

The stack is designed to sense each attempt, compare it with what should have happened, act to close the gap, and
audit its own corrections, keeping the essential variables of §2 in bounds.

| Layer | Target design | Today |
|---|---|---|
| Feedforward | A spec-quality gate before dispatch; recurring failures become plan lints | Plan lints WIRED@a17d4dadd (AU2); the spec-quality gate MISSING@a17d4dadd (AU3, spec-e57870) |
| Sensors | One settled record per attempt: verdict, executed model, cost with its source, routing decision | Telemetry WIRED@a17d4dadd (RG1); attempt records and costs PARTIAL@a17d4dadd (LM1, RC6): no cost source, no decision log (spec-b7303f) |
| Comparators | The visible verify, plus checks the agent cannot see or edit: hidden tests, a tamper and scope diff | The visible verify WIRED@a17d4dadd (QA1); the rest MISSING@a17d4dadd (QA4, QA5; spec-9230a9, spec-6ac537) |
| Fast regulators, within a run | Retry with feedback, escalate one model rung, then split or replan | Retry WIRED@a17d4dadd (EX6); escalation ORPHANED@a17d4dadd (EX7); split or replan BUILT-UNWIRED@a17d4dadd (EX8); spec-98f76d |
| Slow regulators, across runs | Routing learned from verified outcomes; keyed failure memory; a bounded controller (M1) | The loops below; failure memory ORPHANED@a17d4dadd (LM4); M1 MISSING@a17d4dadd (RG3); spec-6ac537 |
| Second-order audits | Random deep audits of green results (M4); per-loop exposure, influence and benefit (M2); guarded commit with rollback | MISSING@a17d4dadd (QA5, RG4, RG6; spec-6ac537) |

Four loops act on what they observe and are WIRED@a17d4dadd: the provider-health circuit breaker (RC3); the plan
budget, which since `abc1f4b27` starts no task once spent (RC5); retry budgets set from each gate rung's pass rate
since `99adacd6d` (QA7); and playbook credit, re-wired in `33e107da1` (LM2). Prompt experiments are assigned and
settled per attempt, but the test that picks a winner is not valid under adaptive assignment (PARTIAL@a17d4dadd:
LM5, spec-6ac537). Verified attempts are written back to the knowledge store, yet plan-run prompts retrieve nothing from it
(BROKEN@a17d4dadd: LM3, bug-86117a). No loop has a measured benefit: the audit that would measure one is
MISSING@a17d4dadd (RG4). §5 describes the mechanisms designed for the missing layers, M1 to M4.

## 3.3 What Roko records

The Graph host and `crates/roko-cli/src/runtime_feedback/` write these records:

| Record | Contents | Caveat that still holds |
|---|---|---|
| `.roko/state/graph/<plan>/` | Checkpoint, activity log and spend | Reliable; resume reads it |
| `.roko/episodes.jsonl` | One record per agent turn | Only a failed verify writes a verdict (since `33e107da1`) |
| `.roko/learn/efficiency.jsonl` | Tokens, cost and gate events | No cost source; since `d4be4e872` a timed-out attempt keeps its streamed usage |
| `.roko/learn/cascade-router.json` | Router state | Its learned stage sees no router-chosen failure (bug-8da8ba) |
| `.roko/learn/gate-thresholds.json` | Pass-rate averages per gate rung | Sets retry budgets since `99adacd6d` |

This paper does not repeat two headline claims of `docs/v3/00-INDEX.md`: "a Graph of Graphs is just a Graph"
(nothing implements `Cell` for `Graph`; gap-cdf3fc) and "every Cell is a learner" (the plan-task Cell keeps the
default `predict`, which returns nothing).

```text
EXECUTION PATH
  tasks.toml ─► validate ─► plan_to_graph ─► Graph engine: a task starts once its dependencies settle
                                                 │ each task
     ┌───────────────────────────────────────────┘
     ▼
  context pack ─► model ─► agent ─► verify ──pass──► records under .roko/
       ▲                              │ fail
       └──── retry with gate output ◄─┘
  surfaces: CLI · TUI · HTTP API + SSE · portal · ACP

CONTROL STACK (reads the records, acts on the path)     ● wired  ◐ partial  ○ not working
  6 second-order audits   ○ random audits  ○ loop audit  ○ guarded commit
  5 slow regulators       ● health  ● budgets  ● retry budgets  ● playbooks
                          ◐ router  ◐ experiments  ○ knowledge  ○ failure memory  ○ controller
  4 fast regulators       ● retry  ○ escalation  ○ split or replan
  3 comparators           ● visible verify  ○ tamper diff  ○ hidden tests
  2 sensors               ● telemetry  ◐ attempt records  ◐ costs
  1 feedforward           ● plan lints  ○ spec-quality gate
```

**Figure 1:** The architecture and the control stack. The marks follow the status matrix at `a17d4dadd`; §3.2 names
each row.
