+++
id = "bug-dfb28f"
kind = "bug"
title = "Graph runs save the cascade router only when the run ends, so a crash loses the run's routing learning"
status = "done"
triage = "verified"
severity = "p2"
goal = "cybernetic"
size = "S"
subsystem = ["roko-cli/graph-execution", "roko-learn/cascade-router"]
created = 2026-09-29
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "9c0b9aed0"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (16:47, wk-router's report on bug-8da8ba, branch work/bug-8da8ba)"
anchors = ["crates/roko-cli/src/graph_execution/plan_runner.rs:1556", "crates/roko-cli/src/runtime_feedback/routing.rs::RoutingObservationSink", "crates/roko-cli/src/graph_execution/feedback.rs", "crates/roko-learn/src/model_call_feedback.rs"]
lane = "rust-hot"
parent = "spec-6ac537"
links = { depends_on = ["bug-8da8ba"], blocks = [], related = ["find-0dc1d5", "bug-012303", "bug-3ea1f5"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn graph_run_routing_observations_survive_a_crash' crates/roko-cli/src/ && cargo test -p roko-cli --lib graph_run_routing_observations_survive_a_crash"

[closed]
at = 2026-09-30
by = "coordinator (session 7622b882)"
evidence = "Merged in 4c0722281 (rustfmt a0a680212). The Graph routing sink journals each outcome through ModelCallJournal; a crashed run's journal replays through load_recovered_router before the next run loads the router. Batch 15a gate on 9005da604, re-assembled as 8a2ee8bca with only settle's rustfmt commit changing two files' formatting (MAIN 9c0b9aed0 has the same code): cargo check --workspace --tests clean; nightly fmt clean; clippy -p roko-agent -p roko-cli -p roko-compose -p roko-core -p roko-learn -p roko-serve --keep-going -D warnings clean; lib tests pass: roko-cli 3190 (two flakes, the turn_policy escalated-timeout test and graph_run_routing_observations_survive_a_crash, pass alone and in their module), roko-agent 2268, roko-core 1952, roko-learn 1204, roko-serve 986, roko-compose 560; cargo test -p roko-cli --test learning_wiring_census: 2 passed. Verify: its test passes in that run and its static checks pass on MAIN."
+++

## Problem

A Graph plan run feeds routing outcomes into its `CascadeRouter` in memory, through `RoutingObservationSink` (`runtime_feedback/routing.rs`) and the routing sink in `graph_execution/feedback.rs`. The router is written to `.roko/learn/cascade-router.json` once, after the run finishes: the "Persist cascade router observations (UX34)" block at `plan_runner.rs:1556`. Nothing journals the observations in between. If the process crashes, is killed or runs out of memory mid-run, every routing observation from that run is lost.

## Why it matters

Routing is one of the cybernetic loops (epic spec-6ac537). Long plan runs are the ones most likely to be interrupted, and they produce the most observations. bug-8da8ba adds `ModelCallJournal`, a write-ahead log for model-call observations, and moves the chat, `dispatch_v2`, serve-runtime and ACP paths onto it. Graph runs are left out.

## Where

- `crates/roko-cli/src/graph_execution/plan_runner.rs`: adds `RoutingObservationSink` to the feedback facade when `graph_run_config.cascade_router` is set (about :954), and saves the router at the end (:1556).
- `crates/roko-cli/src/runtime_feedback/routing.rs`: `RoutingObservationSink::on_event` and `observe_router_outcome` apply each observation to the router.
- `crates/roko-cli/src/graph_execution/feedback.rs`: the routing sink that settles override receipts.
- `crates/roko-learn/src/model_call_feedback.rs`: `ModelCallJournal` (`for_snapshot`, `observe`, `save`), on `work/bug-8da8ba`.

## Current state

Checked on `work/bug-8da8ba` (`26947cd62`, not merged at BASE `2a9312985`): `plan_runner.rs` has no `ModelCallJournal`, and neither Graph routing sink writes a WAL entry. The save at the end of the run is the only persistence. `LearningRuntime` (Path A) has its own WAL, but Graph runs don't record routing through it.

## Plan

1. After bug-8da8ba merges, build one `ModelCallJournal::for_snapshot(<cascade_router_path>)` in `plan_runner.rs` next to the router, and pass it to both Graph routing sinks.
2. In each sink, record through the journal (append the WAL entry, then apply it to the router) instead of calling the router directly.
3. Replace the run-end `cascade.save(..)` with `journal.save(&cascade)`, so the fold marker is written and a replay doesn't count observations twice.
4. Check that the router a Graph run loads replays the journal first, as `LearningRuntime::replay_and_open_wal` does for Path A; add the replay if it doesn't.
5. Add `graph_run_routing_observations_survive_a_crash`: record an outcome through a Graph sink, drop the router without saving, load it again from the snapshot path, and check the observation is there.

## Done when

- [ ] Every Graph-run routing observation is journaled before it is applied, and a load after a simulated crash recovers it.
- [ ] The `[[verify]]` command passes.

## Notes

- Depends on bug-8da8ba (`ModelCallJournal`), which is in Rust batch 3.
- `plan_runner.rs` is a hot file: coordinate with other Graph workers.
- bug-3ea1f5 (the Path A failure reward) touches the same learning path but different files.
- Implemented on `work/bug-f81e9b` at `6a5e1e3af` (`75a76d62c` plus a test fix); cargo verification deferred to the
  batch check.
  The run journals each outcome in the learning WAL before applying it: `ModelCallJournal::observe_task_outcome`,
  and `observe_override_outcome`, whose dampened weight the WAL entry now records. It saves through the journal
  at the end. `RunConfig::from_roko_config` loads the router with `load_recovered_router`, which runs the
  `LearningRuntime` WAL recovery first.
- Not journaled: per-category stats, which the snapshot does not persist either. Also not journaled: the routing
  sink of the receipt settler (`graph_execution/feedback.rs`), because `build_settler` has no production caller.
- 2026-09-30 (wk-settle, `work/bug-dfb28f-flake`): `graph_run_routing_observations_survive_a_crash` failed now and
  then under parallel load because recovery skipped the gone writer's segment. A child process forked while the
  segment was open shares the open file until its exec, and closing the file does not release the flock while the
  child shares it, so the segment looked like a live writer's. A journal, drop and recover loop run beside threads
  that spawn `true` measured it. Before an explicit unlock on drop, 134 to 233 of 400 recoveries skipped the segment
  with fork-and-exec spawns, 55 of 400 with posix_spawn, and none of 400 with no spawns; every skipped segment was
  still on disk. After it, none of 1000 skipped under either kind of spawn. The unlock lands with bug-779ae7
  (`2c61e9053`); this branch adds a deterministic test (a child holds the segment as its stdin) and makes the crash
  test prove the writer is gone (`Arc::try_unwrap` on the journal) before it recovers.
