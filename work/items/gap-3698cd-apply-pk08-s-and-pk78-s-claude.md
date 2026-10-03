+++
id = "gap-3698cd"
kind = "gap"
title = "Apply PK08's and PK78's CLAUDE.md rows (the records plan runs write; stale prompt_experiment and watcher count)"
status = "open"
triage = "verified"
severity = "p3"
goal = "truth"
size = "S"
hold = "waits on the uncommitted CLAUDE.md Goal-line edit in the main checkout (Will's): commit it, or say to apply the rows on top"
subsystem = ["docs"]
created = 2026-10-03
updated = 2026-10-03
last_verified = 2026-10-03
source = "tmp/backlog/2026-10-02-complete-and-wire 2121 and 9213 (held at gates 4c and 5a)"
discovered_from = "gap-a0043b"
anchors = ["CLAUDE.md"]
lane = "docs"
links = { depends_on = [], blocks = [], related = ["gap-a0043b", "gap-2339e2"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -q 'Canonical signal log' CLAUDE.md && grep -q 'attempts.jsonl' CLAUDE.md"

[[verify]]
command = "! grep -q 'prompt_experiment: None' CLAUDE.md && ! grep -q '12 watchers' CLAUDE.md"
+++

## Problem

Two merged packages left their CLAUDE.md rows unapplied. PK08 (gap-a0043b, task 2121) says the signal log is the
kernel substrate's log, which plan runs don't write, and names the records plan runs do write (`attempts.jsonl`,
`manifest.json`, `events.jsonl`). PK78 (gap-2339e2, task 9213) replaces the stale `prompt_experiment: None` example
and the "12 watchers" count. The code they describe is merged (gates 4c and 5a). The rows aren't, because the main
checkout carries an uncommitted CLAUDE.md edit (the Goal line) that isn't the batch's, and a merge that touches
CLAUDE.md would overwrite it.

## Why it matters

CLAUDE.md is what every agent reads first. Today it calls `signals.jsonl` the canonical log, which plan runs never
write, and it names a `prompt_experiment: None` that Graph dispatch no longer passes.

## Where

`CLAUDE.md`: the Components table (Signal log row), the Absolute paths table, the Key crates table (roko-conductor
row) and the long-term priorities list.

## Current state

Both patches are below, against CLAUDE.md at 8765adea9 (the committed text under the Goal-line edit). They don't
touch the Goal line.

## Plan

1. Once the Goal-line edit is committed (or Will says to apply the rows on top of it), apply both patches to
   CLAUDE.md and commit them.
2. Run the two verifies.

## Done when

- [ ] Both `[[verify]]` commands pass.

## Notes

- Moved out of PK08's and PK78's package items on 2026-10-03 (coordinator, after gate 6b) so that the packages that
  wait on them (PK30, PK80) can start.

PK08 (task 2121):

```diff
diff --git a/CLAUDE.md b/CLAUDE.md
index 8765adea9..6551758bc 100644
--- a/CLAUDE.md
+++ b/CLAUDE.md
@@ -42,7 +42,8 @@ What each subsystem is and where it lives. The table makes no maturity or status
 | GitHub integration | `roko github status`; `GitHubOps` trait with no-op and live adapters; `[github]` config | `crates/roko-cli/src/commands/github.rs`, `crates/roko-cli/src/github_ops.rs`, `crates/roko-cli/src/github_ops_impl.rs`, `roko.toml` |
 | Plugins | Plugin manifests, declarative tools, capability policy, dependency resolution | `crates/roko-plugin/` |
 | Chain primitives | Optional chain client plus local registry, marketplace, arena and DeFi state machines | `crates/roko-chain/` |
-| Signal log | Canonical signal log (a legacy `engrams.jsonl` is read only as a fallback) | `.roko/signals.jsonl` (path logic in `crates/roko-fs/src/layout.rs`) |
+| Signal log | The kernel substrate's log (`FileSubstrate`; a legacy `engrams.jsonl` is read only as a fallback). Serve and `roko prd plan` write it; plan runs do not. `roko status` and `roko replay` read it | `.roko/signals.jsonl` (path logic in `crates/roko-fs/src/layout.rs`) |
+| Plan run records | What a plan run writes: per run, an attempt-open line and one settled verdict per attempt (`attempts.jsonl`) and the run manifest (`manifest.json`); for the workspace, the run's dashboard events (`events.jsonl`); per plan, the execution state above | `.roko/runs/<run>/` (`attempts.jsonl`, `manifest.json`), `.roko/events.jsonl` |
 
 ## Critical rules
 
@@ -327,7 +328,8 @@ safety, auth, persistence, migration, payment, or other high-risk changes. FAST
 | **Graph checkpoints** | `/Users/will/dev/nunchi/roko/roko/.roko/state/graph/` |
 | **Plans** | `/Users/will/dev/nunchi/roko/roko/plans/` |
 | **Research artifacts** | `/Users/will/dev/nunchi/roko/roko/.roko/research/` |
-| **Signal log** | `/Users/will/dev/nunchi/roko/roko/.roko/signals.jsonl` |
+| **Signal log (kernel substrate; plan runs do not write it)** | `/Users/will/dev/nunchi/roko/roko/.roko/signals.jsonl` |
+| **Plan run records** | `/Users/will/dev/nunchi/roko/roko/.roko/runs/<run>/` (`attempts.jsonl`, `manifest.json`), `/Users/will/dev/nunchi/roko/roko/.roko/events.jsonl` |
 | **Episode log** | `/Users/will/dev/nunchi/roko/roko/.roko/episodes.jsonl` |
 
 ## Reference material (read-only, do not modify)
```

PK78 (task 9213):

```diff
diff --git a/CLAUDE.md b/CLAUDE.md
index 8765adea9..847d203a2 100644
--- a/CLAUDE.md
+++ b/CLAUDE.md
@@ -287,7 +287,7 @@ safety, auth, persistence, migration, payment, or other high-risk changes. FAST
 | roko-serve | `crates/roko-serve/` | HTTP control plane: REST routes + SSE + WebSocket on :6677 |
 | roko-gate | `crates/roko-gate/` | 19 gates, 7-rung pipeline, adaptive thresholds |
 | roko-compose | `crates/roko-compose/` | Prompt assembly, 11 role templates, enrichment |
-| roko-conductor | `crates/roko-conductor/` | 12 watchers, circuit breaker, diagnosis |
+| roko-conductor | `crates/roko-conductor/` | 13 watchers, circuit breaker, diagnosis |
 | roko-learn | `crates/roko-learn/` | Episodes, playbooks, bandits, model routing, experiments, efficiency |
 | roko-cli | `crates/roko-cli/` | CLI, plan DAG/runner, merge queue, worktree manager, ratatui TUI |
 | roko-fs | `crates/roko-fs/` | FileSubstrate (JSONL), GC, layout |
@@ -387,8 +387,9 @@ Long-term priorities that still hold:
    end. The blockers from the first dogfood run have regression fixes, but no live rerun has been
    recorded.
 2. **Learning loops on the Graph path**: several feedback paths were attached to the deleted
-   Runner-v2 event loop and have not been re-attached to Graph runs. For example, Graph task
-   dispatch passes `prompt_experiment: None` (`crates/roko-cli/src/graph_task_dispatch.rs:1791`).
+   Runner-v2 event loop and have not been re-attached to Graph runs. For example, Graph runs never
+   emit `FeedbackEvent::PlanCompleted`, so the dream, daimon, theta and delta plan-completion sinks
+   never fire (q-6b7cca).
 3. **roko tracks its own work**: the `roko work` CLI (it extends `roko backlog`, per
    `work/README.md`) plus plan-task `closes = [...]` links, so that roko itself maintains the work
    graph.
```
