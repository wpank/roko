Status: stub · budget 700 words · owner gap-4161ea

# 3 Architecture and control stack

[[TODO: Write this section to gap-4161ea's plan, in about 700 words. It contains "Figure 1", and every crate path and identifier exists at HEAD or is marked (designed). Keep line 1 and set it to `draft`, then replace everything below the heading, this list of claims included. Conventions and canonical numbers: `README.md` in this directory.]]

## Claims

| Id | Kind | Claim | Writer's source | Cite in the text as |
|---|---|---|---|---|
| AR1 | design | A plan is a `tasks.toml`: tasks, their dependencies, the files each may touch, the context to load, and the commands that verify them. `plan_to_graph` compiles it into a Graph (a DAG of Cells), and the Graph engine has been the only plan executor since Runner-v2 was deleted. | tldr/02 "In one paragraph", "The building blocks"; A1 | `crates/roko-graph/src/convert.rs`; `crates/roko-graph/src/engine.rs`; commit `6b5da8616`; a WIRED tag |
| AR2 | design | Per-task dispatch: context pack, model choice, the agent run (a CLI such as Claude Code or Codex, or an API model), the task's verify commands, and a retry with the failure output. | tldr/02; tldr/03 "Execution" | `crates/roko-cli/src/graph_task_dispatch.rs`; `crates/roko-cli/src/dispatch/`; `crates/roko-agent/`; tags from the appendix |
| AR3 | status | Nodes start as soon as their own dependencies settle, and a failed task blocks only its dependants. This moved after the tldr was written. | README "Rows that moved" | Commits `445a60d0d`, `3e7552acd`, `bbf6517fc`; the tag from the appendix |
| AR4 | status | The default topology is one executor node per task. An 11-node per-task subgraph exists behind `--rich-topology`, and its enrichers pass through. | tldr/02 "Plan topology" | `crates/roko-graph/src/convert.rs`; the tag from the appendix |
| AR5 | design | The surfaces: the CLI, the TUI, an HTTP API with SSE on :6677, the portal and ACP for editors. | tldr/02; CLAUDE.md "Components" | `crates/roko-serve/`; `apps/portal/`; `crates/roko-acp/`; WIRED tags |
| AR6 | design | The control stack in six layers (feedforward, sensors, comparators, fast regulators, slow regulators, second-order audits), each with its target and today's tag. | tldr/02 control-stack table; draft §4.1; W9 PW06 | Tags from the appendix, re-derived from the code |
| AR7 | status | The closed loops today are the provider-health circuit breaker and the plan budget. | tldr/02 "Slow regulators"; tldr/03 "Routing and cost" | Tags from the appendix |
| AR8 | status | What Roko records under `.roko/`, with only the caveats that still hold: checkpoints (used for resume), episodes (verdicts missing), cost records (no cost source), router state, and gate thresholds, which now set retry budgets. | tldr/02 "What Roko records" (its gate-thresholds row is stale) | `crates/roko-cli/src/runtime_feedback/`; commit `99adacd6d`; spec-b7303f (E4) |
| AR9 | status | Two docs/v3 headline claims are not code: "a Graph of Graphs is just a Graph" and "every Cell is a learner". Leave them out or tag them DOCS-ONLY. | tldr/02; tldr/05 §5; A1 | A DOCS-ONLY tag |
| AR10 | figure | Figure 1: the architecture and the control stack. The section carries its caption and a fenced text diagram; gap-d1d92c draws the SVG. | tldr/02 flow diagram | `figures/fig1-architecture.svg` |

## Sources

Writer inputs; the paths under `tmp/` are gitignored and are never cited in the text.

- tldr/02: `tmp/cybernetic-harness/tldr/02-HOW-IT-WORKS.md`. Architecture, the control stack, what `.roko/` records.
- tldr/03: `tmp/cybernetic-harness/tldr/03-MECHANISMS.md`. Every mechanism with basis, status and verdict.
- tldr/05: `tmp/cybernetic-harness/tldr/05-GAPS-AND-PROPOSALS.md`. Scorecard V1-V10, proposals P0-P3, parking, doc corrections, decisions.
- A1: `tmp/cybernetic-harness/tldr/research/A1-core-architecture.md`. Core architecture against docs/v3.
- draft §N: `tmp/cybernetic-harness/paper/sections/`. The research draft's sections; outline in `paper/OUTLINE.md`, conventions in `paper/00-README.md`.
- W9: `tmp/cybernetic-harness/workstreams/assessment/W9-paper-workstream.md`. Rules for honest ideal-state writing; the phantom identifiers.
- matrix: `docs/whitepaper/appendix-status-matrix.md`. The status matrix (gap-35a614): every tag at one pinned commit.
