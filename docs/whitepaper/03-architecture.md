Status: draft · budget 700 words · owner gap-4161ea

# 3 Architecture and control stack

Roko has two parts: an execution path that turns a plan into verified, recorded results, and a control stack
designed to regulate that path from its records (Figure 1). Every tag here is at `a17d4dadd`, the commit of
the appendix's status matrix.

## 3.1 From plan to recorded result

A plan is a `tasks.toml`: tasks, their dependencies, the files each may touch, the context to load, and the shell
commands that verify each task. `plan_to_graph` (`crates/roko-graph/src/convert.rs`) compiles it into a Graph, a DAG
of Cells, which the Graph engine (`crates/roko-graph/src/engine.rs`) runs and checkpoints, so an interrupted run
resumes where it stopped. The engine has been the only plan executor since Runner-v2 was deleted in `6b5da8616`
(WIRED@a17d4dadd). Each task is one executor node; the 11-node per-task subgraph behind `--rich-topology` adds
enrichers that pass their input through unchanged.

A task starts once its own dependencies settle, and a failure blocks only its dependants (`445a60d0d`,
`3e7552acd`). Scheduling is still PARTIAL@a17d4dadd: `max_parallel` defaults to 1, and nothing admits parallel tasks
by their write sets (spec-a78d57).

For each task, `GraphTaskDispatcher` (`crates/roko-cli/src/graph_task_dispatch.rs`) renders the declared files and
symbols into the prompt (`render_declared_context`), picks a model, runs an agent (a CLI such as Claude Code or
Codex, or an API model), runs the task's verify commands, and retries a failure with the parsed gate output. These
steps are WIRED@a17d4dadd. Three parts are PARTIAL@a17d4dadd: `CascadeRouter` learns from successes only
(bug-8da8ba); an `Unverified` verdict still counts as a pass downstream (spec-e9d7ec); and tasks share the
operator's checkout unless per-task worktrees are requested, whose results nothing merges back (spec-a0e40a).
Since `1d923e377`, verify commands start from an allowlisted environment and agent CLIs no longer inherit other
providers' keys, but key files stay readable (PARTIAL@a17d4dadd, spec-ba7bea).

Operators watch runs through the CLI, a TUI, an HTTP API with SSE on port 6677 (`crates/roko-serve/`), the portal
(`apps/portal/`) and ACP for editors (`crates/roko-acp/`), all WIRED@a17d4dadd. Pause, retry and skip are
BROKEN@a17d4dadd (bug-8208a6).

## 3.2 The control stack

The stack is designed to sense each attempt, compare it with what should have happened, act to close the gap, and
audit its own corrections, keeping the essential variables of §2 in bounds.

| Layer | Target design | Today |
|---|---|---|
| Feedforward | A spec-quality gate before dispatch; recurring failures become plan lints | Structural plan lints (WIRED@a17d4dadd); the spec-quality gate is MISSING@a17d4dadd (spec-e57870) |
| Sensors | One settled record per attempt: verdict, executed model, cost with its source, routing decision | PARTIAL@a17d4dadd: learning records reduce verdicts to pass or fail, cost has no source, and no decision is logged (spec-b7303f) |
| Comparators | The visible verify, plus checks the agent cannot see or edit: hidden tests, a tamper and scope diff | PARTIAL@a17d4dadd: the visible verify only (spec-e9d7ec, spec-9230a9) |
| Fast regulators, within a run | Retry with feedback, escalate one model rung, then split or replan | PARTIAL@a17d4dadd: retry on the same model; escalation is ORPHANED@a17d4dadd, and `ReplanController` has no caller (spec-98f76d) |
| Slow regulators, across runs | Routing learned from verified outcomes; keyed failure memory; a bounded controller (M1) | PARTIAL@a17d4dadd: the loops below; M1 is MISSING@a17d4dadd (spec-6ac537) |
| Second-order audits | Random deep audits of green results (M4); per-loop exposure, influence and benefit (M2); guarded commit with rollback | MISSING@a17d4dadd (spec-6ac537) |

Five loops close on plan runs, acting on what they observe: the provider-health circuit breaker; the plan budget,
which since `abc1f4b27` starts no task once spent; retry budgets set from each gate rung's pass rate (`99adacd6d`,
never below three since `41c7ffbd6`); and playbook credit and prompt-experiment assignment, re-wired in
`33e107da1`. None has a measured benefit (UNPROVEN@a17d4dadd, spec-6ac537). Knowledge write-back was
re-wired too, but plan-run prompts retrieve nothing from the store (BROKEN@a17d4dadd, bug-86117a). §5 describes the
mechanisms designed for the missing layers, M1 to M4.

## 3.3 What Roko records

The Graph host and `crates/roko-cli/src/runtime_feedback/` write these records:

| Record | Contents | Caveat that still holds |
|---|---|---|
| `.roko/state/graph/<plan>/` | Checkpoint, activity log and spend | Reliable; resume reads it |
| `.roko/episodes.jsonl` | One record per agent turn | Only a failed verify writes a verdict (since `33e107da1`) |
| `.roko/learn/efficiency.jsonl` | Tokens, cost and gate events | No cost source; since `d4be4e872` a timed-out Claude CLI attempt keeps its streamed usage |
| `.roko/learn/cascade-router.json` | Router state | Its learned stage sees no router-chosen failure (bug-8da8ba) |
| `.roko/learn/gate-thresholds.json` | Pass-rate averages per gate rung | Sets retry budgets since `99adacd6d` |

Two headline claims in `docs/v3/00-INDEX.md` are DOCS-ONLY@a17d4dadd, and this paper does not make them: "a Graph
of Graphs is just a Graph" (nothing implements `Cell` for `Graph`; gap-cdf3fc) and "every Cell is a learner" (the
plan-task Cell keeps the default `predict`, which returns nothing).

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

CONTROL STACK (reads the records, acts on the path)
  6 second-order audits   M2, M4, guarded commit    MISSING
  5 slow regulators       five loops; no M1         PARTIAL
  4 fast regulators       retry only                PARTIAL
  3 comparators           visible verify only       PARTIAL
  2 sensors               records, no cost source   PARTIAL
  1 feedforward           no spec-quality gate      MISSING
```

**Figure 1:** The architecture and the control stack, each layer with its status at `a17d4dadd`.
