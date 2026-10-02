+++
id = "gap-2339e2"
kind = "gap"
title = "PK78 Park and clean up: Five next-step hints tell users to run the removed `roko develop` (+11 more)"
status = "open"
triage = "verified"
severity = "p3"
goal = "tooling"
rank = 78
size = "L"
subsystem = ["docs"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/backlog/2026-10-02-complete-and-wire PK78"
anchors = ["CLAUDE.md", "crates/roko-cli/src/commands/backlog.rs", "crates/roko-cli/src/commands/develop.rs", "crates/roko-cli/src/commands/mod.rs", "crates/roko-cli/src/commands/prd.rs", "crates/roko-cli/src/commands/setup.rs", "crates/roko-cli/src/commands/util.rs", "crates/roko-cli/src/main.rs", "crates/roko-cli/src/main_tests.rs", "crates/roko-cli/src/orchestrator/merge_queue.rs", "crates/roko-cli/src/orchestrator/mod.rs", "crates/roko-cli/src/orchestrator/runtime_snapshot.rs", "crates/roko-cli/src/resolved_overrides.rs", "crates/roko-cli/src/runner/gate_dispatch.rs", "crates/roko-cli/src/runner/merge.rs", "crates/roko-graph/src/cells/plan_compose.rs", "crates/roko-graph/src/cells/stubs.rs", "crates/roko-graph/src/delivery.rs", "crates/roko-graph/src/engine.rs", "crates/roko-graph/src/lib.rs", "crates/roko-graph/src/topology.rs", "crates/roko-learn/src/cascade/helpers.rs", "crates/roko-learn/src/cascade/tests.rs", "crates/roko-learn/src/cascade_router.rs", "crates/roko-mcp-code/README.md", "crates/roko-serve/src/integrations.rs", "docs/v3/00-INDEX.md", "docs/v3/12-SAFETY.md", "docs/v3/20-GATEWAY.md", "docs/v3/31-SELF-HOSTING.md", "docs/v3/35-ARCHITECTURE.md", "docs/v3/39-ROADMAP.md", "docs/v3/REFERENCES.md", "docs/v3/depth/00-architecture/crate-map-and-dependencies.md", "docs/v3/depth/08-learning/self-improvement-frameworks.md", "docs/v3/depth/19-tools/service-integrations.md", "docs/v3/depth/35-architecture/crate-dependency-graph.md", "docs/v3/depth/39-references/17-process-reward-models.md", "docs/v3/depth/39-references/24-additions-2025-2026.md", "docs/v3/explorer/architecture.md", "docs/v3/explorer/crate-map.md"]
lane = "rust-hot"
parent = "spec-0b3a32"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -q 'roko develop' crates/roko-cli/src/commands/backlog.rs crates/roko-cli/src/commands/util.rs crates/roko-cli/src/commands/prd.rs crates/roko-cli/src/commands/setup.rs"

[[verify]]
command = "! grep -q 'Develop {' crates/roko-cli/src/main.rs && test ! -e crates/roko-cli/src/commands/develop.rs && ! grep -q 'struct DevelopInput' crates/roko-cli/src/resolved_overrides.rs"

[[verify]]
command = "! grep -rqw 'MergeEnqueuer' crates/roko-graph/src && ! grep -q 'fn with_merge_queue' crates/roko-graph/src/engine.rs"

[[verify]]
command = "! grep -rqw 'pub struct MergeQueue' crates/roko-cli/src && ! grep -rqw 'RUNG_MERGE' crates/roko-cli/src && test ! -e crates/roko-cli/src/orchestrator/runtime_snapshot.rs"

[[verify]]
command = "! grep -q 'ENRICHER_SUFFIXES' crates/roko-graph/src/topology.rs && grep -rqw 'fn rich_topology_task_subgraph_has_no_passthrough_enrichers' crates/roko-graph/ && cargo test -p roko-graph rich_topology_task_subgraph_has_no_passthrough_enrichers"

[[verify]]
command = "! grep -q '\"gemini-2.5-flash-lite\", \"claude-haiku-4-5\"' crates/roko-learn/src/cascade_router.rs crates/roko-learn/src/cascade/helpers.rs && grep -rqw 'fn static_route_never_names_an_unconfigured_model' crates/roko-learn/ && cargo test -p roko-learn static_route_never_names_an_unconfigured_model"

[[verify]]
command = "! grep -qE 'roko-mcp-(slack|scripts)' crates/roko-serve/src/integrations.rs && grep -rqw 'fn builtin_integrations_name_only_shipped_mcp_servers' crates/roko-serve/ && cargo test -p roko-serve builtin_integrations_name_only_shipped_mcp_servers"

[[verify]]
command = "! grep -q 'roko-mcp-slack' docs/v3/00-INDEX.md docs/v3/35-ARCHITECTURE.md docs/v3/depth/00-architecture/crate-map-and-dependencies.md docs/v3/depth/35-architecture/crate-dependency-graph.md docs/v3/depth/19-tools/service-integrations.md docs/v3/explorer/architecture.md docs/v3/explorer/crate-map.md"

[[verify]]
command = "! grep -q 'always dominates' docs/v3/12-SAFETY.md"

[[verify]]
command = "! grep -rqE '48/48 accepted|45% of otherwise-discarded|Partial-Pass Scoring' docs/v3 --include='*.md' && ! grep -q '| Accepted epics | 48/48 |' docs/v3/depth/00-architecture/crate-map-and-dependencies.md && ! grep -qE '\\| Workspace members \\| 3[79] \\|' docs/v3/35-ARCHITECTURE.md"

[[verify]]
command = "! grep -q '### 3.1 Five Replan Strategies' docs/v3/31-SELF-HOSTING.md && ! grep -q 'COMPLETE (E26 12/12)' docs/v3/20-GATEWAY.md"

[[verify]]
command = "! grep -q 'prompt_experiment: None' CLAUDE.md && ! grep -q '12 watchers' CLAUDE.md"
+++

## Problem

This package delivers 12 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK78, slice 92xx, phase 9), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 9202 | S | p3 | Five next-step hints tell users to run the removed `roko develop` | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9202-fix-hints-that-point-at-removed-roko-develop.md` |
| 2 | 9203 | S | p3 | Delete the hidden `roko develop` stub and its dead `cmd_develop` module | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9203-delete-hidden-roko-develop-stub-and-dead-module.md` |
| 3 | 9204 | S | p3 | Remove the Graph engine's unused merge-queue hook (`MergeEnqueuer`, `with_merge_queue`) | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9204-remove-dead-merge-enqueuer-from-graph-engine.md` |
| 4 | 9205 | S | p3 | Delete roko-cli's unused `MergeQueue`, `RuntimeSnapshot` and `RUNG_MERGE` | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9205-delete-unused-mergequeue-and-runtime-snapshot.md` |
| 5 | 9206 | M | p3 | Drop the six passthrough enricher cells from the `--rich-topology` subgraph | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9206-drop-rich-topology-passthrough-enricher-cells.md` |
| 6 | 9207 | M | p3 | Static routing picks from the configured tier map, not hard-coded Gemini and Claude slugs | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9207-static-routing-uses-tier-map-not-builtin-slugs.md` |
| 7 | 9208 | S | p3 | Serve's built-in integrations advertise MCP servers that do not exist (`roko-mcp-slack`, `roko-mcp-scripts`) | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9208-integrations-name-only-shipped-mcp-servers.md` |
| 8 | 9209 | S | p3 | The v3 docs list a `roko-mcp-slack` crate that does not exist | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9209-remove-roko-mcp-slack-from-v3-docs.md` |
| 9 | 9210 | S | p3 | 12-SAFETY claims deference to the operator always dominates | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9210-safety-doc-overstates-operator-deference.md` |
| 10 | 9211 | S | p3 | Residual doc claims survive gap-cdf3fc: 48/48 accepted, the 45% hindsight figure, the Messier citation | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9211-fix-residual-48-48-hindsight-and-messier-claims.md` |
| 11 | 9212 | S | p3 | Two corrected docs keep a misleading heading and status line (replan strategies, gateway COMPLETE) | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9212-fix-replan-heading-and-gateway-complete-residues.md` |
| 12 | 9213 | S | p3 | CLAUDE.md says Graph dispatch passes `prompt_experiment: None` and the conductor has 12 watchers | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9213-claude-md-stale-prompt-experiment-and-watchers.md` |

## Why it matters

Phase 9: domains, assistant, held and parked work, cleanup, showcase and deploy. The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9200-held-parked-and-cleanup.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `CLAUDE.md`, `crates/roko-cli/src/commands/backlog.rs`, `crates/roko-cli/src/commands/develop.rs`, `crates/roko-cli/src/commands/mod.rs`, `crates/roko-cli/src/commands/prd.rs`, `crates/roko-cli/src/commands/setup.rs`, `crates/roko-cli/src/commands/util.rs`, `crates/roko-cli/src/graph_execution/plan_runner.rs`, `crates/roko-cli/src/main.rs`, `crates/roko-cli/src/main_tests.rs`, `crates/roko-cli/src/orchestrator/merge_queue.rs`, `crates/roko-cli/src/orchestrator/mod.rs`, `crates/roko-cli/src/orchestrator/runtime_snapshot.rs`, `crates/roko-cli/src/resolved_overrides.rs`, `crates/roko-cli/src/runner/gate_dispatch.rs`, `crates/roko-cli/src/runner/merge.rs`, `crates/roko-graph/src/cells/plan_compose.rs`, `crates/roko-graph/src/cells/stubs.rs`, `crates/roko-graph/src/delivery.rs`, `crates/roko-graph/src/engine.rs`, `crates/roko-graph/src/lib.rs`, `crates/roko-graph/src/topology.rs`, `crates/roko-learn/src/cascade/helpers.rs`, `crates/roko-learn/src/cascade/tests.rs`, `crates/roko-learn/src/cascade_router.rs`, `crates/roko-mcp-code/README.md`, `crates/roko-serve/src/integrations.rs`, `docs/v3/00-INDEX.md`, `docs/v3/12-SAFETY.md`, `docs/v3/20-GATEWAY.md`, `docs/v3/31-SELF-HOSTING.md`, `docs/v3/35-ARCHITECTURE.md`, `docs/v3/39-ROADMAP.md`, `docs/v3/REFERENCES.md`, `docs/v3/depth/00-architecture/crate-map-and-dependencies.md`, `docs/v3/depth/08-learning/self-improvement-frameworks.md`, `docs/v3/depth/19-tools/service-integrations.md`, `docs/v3/depth/35-architecture/crate-dependency-graph.md`, `docs/v3/depth/39-references/17-process-reward-models.md`, `docs/v3/depth/39-references/24-additions-2025-2026.md`, `docs/v3/explorer/architecture.md`, `docs/v3/explorer/crate-map.md`.

It also edits the hot file(s) `crates/roko-cli/src/graph_execution/plan_runner.rs`, which are left out of this item's anchors so that two hot packages can run at once; the coordinator resolves any merge conflict.

## Current state

The tasks were checked against `2c3ea9f73` on 2026-10-02. Re-check each task's anchors and premise at your base commit before implementing it, and report a task that is already done instead of redoing it.

## Plan

1. Work through the tasks in the order above. For each: read its file, implement its Plan, write the test it names, and make one commit per task whose message ends with `Backlog-Task: <task id>`, `Work-Item: <this item's id>` and `Executor: claude-agent`.
2. Follow `BUILD-RULES.md` in the backlog folder. Workers run no cargo: Rust is checked by the coordinator's batched gate. Python and doc checks you may run.
3. If a task cannot be done (a premise is false, a decision is missing, or its verify cannot pass), stop at that task, keep the earlier commits, and report it; do not skip ahead to tasks that depend on it.

## Done when

- Every task's verify command passes (this item's `[[verify]]` list, one entry per task), after the coordinator's batched gate.
- Each task's own "Done when" holds (see its file).

## Notes

- Waits on: nothing.
- Suggested model: sonnet.
