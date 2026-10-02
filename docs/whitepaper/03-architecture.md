Status: reviewed · budget 700 words · owner gap-4161ea

# 3 Architecture and control stack

Roko has two parts: an execution path that turns a plan into verified, recorded results, and a control stack
designed to regulate that path from its records (Figure 1). Every tag here comes from the appendix's status matrix,
pinned at `a43288b5f`, and names its row, such as EX3.

## 3.1 From plan to recorded result

A plan is a `tasks.toml`: tasks, their dependencies, the files each may touch, the context to load, and the shell
commands that verify each task. `plan_to_graph` (`crates/roko-graph/src/convert.rs`) compiles it into a Graph, a DAG
of Cells, which the Graph engine (`crates/roko-graph/src/engine.rs`) runs and checkpoints, so an interrupted run
resumes where it stopped (WIRED@a43288b5f: EX1, EX2). The engine has been the only plan executor since Runner-v2 was
deleted in `6b5da8616`. Each task is one executor node; the 11-node per-task subgraph behind `--rich-topology` adds
enrichers that pass their input through unchanged.

A task starts once its own dependencies settle, a failure blocks only its dependants (`445a60d0d`, `3e7552acd`), and
tasks whose declared files overlap never run together. Since `626e182a9` a plan that omits `max_parallel` runs as wide
as its graph when its tasks declare their files (WIRED@a43288b5f, EX3).

For each task, `GraphTaskDispatcher` (`crates/roko-cli/src/graph_task_dispatch.rs`) renders the declared files and
symbols into the prompt (`render_declared_context`), picks a model rung by the task's role and tier, runs an agent (a
CLI such as Claude Code or Codex, or an API model) in the task's own git worktree, screens its diff for tampering,
runs the task's verify commands, and retries a failure with the parsed gate output, one rung up after two failures
blamed on the agent (WIRED@a43288b5f: AU4, IS1, QA4, QA1, QA2, EX6, EX7). Passed attempts land on plan branches that
merge into a batch branch under a whole-plan check (WIRED@a43288b5f: IS2, IS3). Three parts are PARTIAL@a43288b5f: the
learned `CascadeRouter` picks only where no ladder rung can run (RC2); the OpenAI-compatible adapter keeps one field
per stream chunk, so GLM-4.7's answers arrived blank in the live run (RC1, gap-625195);[^3-live] and provider CLIs and
MCP servers lose only the key names roko knows (IS5, gap-843aef).

Operators watch runs through a TUI, an HTTP API with SSE on port 6677 (`crates/roko-serve/`) and the portal
(`apps/portal/`), all WIRED@a43288b5f (SS4); the CLI starts runs, and ACP (`crates/roko-acp/`) serves editors. Pause
does nothing: BROKEN@a43288b5f (SS5, gap-198c9c).

## 3.2 The control stack

The stack is designed to sense each attempt, compare it with what should have happened, act to close the gap, and
audit its own corrections, keeping the essential variables of §2 in bounds.

| Layer | Target design | Today |
|---|---|---|
| Feedforward | A spec-quality gate before dispatch; recurring failures become plan lints | Plan lints WIRED@a43288b5f (AU2); the spec-quality lint PARTIAL@a43288b5f, opt-in and not yet a gate (AU3, spec-e57870) |
| Sensors | One settled record per attempt: verdict, executed model, cost with its source, routing decision | Telemetry WIRED@a43288b5f (RG1); attempt records and costs PARTIAL@a43288b5f (LM1, RC6): the record has no cost amounts, an unpriced call counts as zero, and there is no decision log (gap-f548c1) |
| Comparators | The visible verify, plus checks the agent cannot see or edit: hidden tests, a tamper and scope diff | The visible verify WIRED@a43288b5f (QA1), with pinned acceptance tests (AU7) and a tamper and scope diff (QA4); hidden tests MISSING@a43288b5f (QA5; spec-6ac537) |
| Fast regulators, within a run | Retry with feedback, escalate one model rung, then split or replan | Retry, escalation and a stall watchdog WIRED@a43288b5f (EX6, EX7, EX9); split or replan BUILT-UNWIRED@a43288b5f (EX8) |
| Slow regulators, across runs | Routing learned from verified outcomes; keyed failure memory; a bounded controller (M1) | The loops below; failure memory PARTIAL@a43288b5f (LM4); M1 MISSING@a43288b5f (RG3); spec-6ac537 |
| Second-order audits | Random deep audits of green results (M4); per-loop exposure, influence and benefit (M2); guarded commit with rollback | MISSING@a43288b5f (QA5, RG4, RG6; spec-6ac537) |

Playbook credit closes its loop (WIRED@a43288b5f, LM2). Three more act, but each misfired in the live run, so they are
PARTIAL@a43288b5f: the provider-health circuit breaker counted output-screening denials and a login failure as
provider failures (RC3); the plan budget, which since `abc1f4b27` starts no task once spent, saw OpenAI spend as zero
(RC5); and the ladder overrode the retry budgets set from each gate rung's pass rate (QA7).[^3-live] §5.2 tags the
other loops. No loop has a measured benefit: the audit that would measure one is MISSING@a43288b5f (RG4). §5 describes
the mechanisms designed for the missing layers, M1 to M4.

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

**Figure 1:** The architecture and the control stack. The marks follow the status matrix at `a43288b5f`; §3.2 names
each row.

[^3-live]: The live run of 2026-10-02, frozen as `evidence/2026-10-02-live-cheap-model-run.md` (sha256
    `813172c96b88`), "What broke" 1, 2, 7 and 8, and its root causes in
    `evidence/2026-10-02-live-defect-root-causes.md` (sha256 `5df7d5221539`), §1, §2 (the ladder raising retries to
    five) and §4: a binary built at `a43288b5f`, two five-task plans in a test repository.
