+++
id = "gap-d65a17"
kind = "gap"
title = "build_settler's RoutingSink updates the router without journaling, and build_settler still has no production caller"
status = "done"
triage = "verified"
severity = "p3"
goal = "cybernetic"
size = "S"
subsystem = ["roko-cli/graph_execution/feedback"]
created = 2026-09-30
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "aec267cac"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-settle's report, checked on work/bug-f81e9b at 7db865c81)"
anchors = ["crates/roko-cli/src/graph_execution/feedback.rs::build_settler"]
lane = "rust-hot"
parent = "spec-6ac537"
links = { depends_on = [], blocks = [], related = ["gap-2ce86f", "bug-84de98"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -q 'struct RoutingSink' crates/roko-cli/src/graph_execution/feedback.rs || (grep -rqw 'fn routing_sink_journals_its_observations' crates/roko-cli/src/ && cargo test -p roko-cli --lib routing_sink_journals_its_observations)"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T13:44:17Z"
by = "coordinator (session 7622b882)"
forced = false
evidence = "Batch 20e gate on 1a8aad603, re-checked with the compile fixes (cf722c1be, bed29287d), tiers' rustfmt (8a6c932ce) and the run-index scrub fix (d972959bd) on 32fe02384; MAIN aec267cac has the same crates: check --workspace --tests, nightly fmt and clippy -D warnings clean on roko-acp/agent/cli/core/dreams/fs/gateway/graph/learn/neuro/serve/std; lib tests roko-cli 3305, roko-agent 2294, roko-core 1962, roko-learn 1213, roko-serve 991, roko-graph 478, roko-fs 260, roko-neuro 239, roko-std 229, roko-acp 199, roko-dreams 100, roko-gateway 42 all pass; extras: golden_path_suite 2/2, all eight canaries pass (secret_canary 11/11 and C2 2/2 after the scrub fix), worktree_task_diff 2/2, plan_run_config_flag 1/1, default_engine 1, bin 429, routing crash loop 10/10, bench driver 18; option (a): build_settler, its 12 sinks and CompletionSinkResult were deleted (no production caller). Merged f68c5673c."
+++

## Problem

`build_settler` (`crates/roko-cli/src/graph_execution/feedback.rs:33` on the settle branch) still has no production caller; only its tests call it (:1016, :1049). gap-2ce86f got the error patterns written another way. Its `RoutingSink` (:60, :366) updates the cascade router directly, without the journal the other router writers use. If anyone wires `build_settler`, those observations bypass crash recovery and double-count protection (bug-84de98).

## Why it matters

Cybernetic core (epic spec-6ac537): dead code that would reintroduce a fixed bug if wired. p3.

## Where

`build_settler` and `RoutingSink` in `graph_execution/feedback.rs`.

## Plan

Pick one:

- **(a)** Delete `build_settler` and its sinks, if gap-2ce86f's path covers what they did.
- **(b)** Make `RoutingSink` write through the journal, and add `routing_sink_journals_its_observations`.

## Done when

- [ ] `RoutingSink` is gone, or it journals.
- [ ] The `[[verify]]` command passes.

## Notes

- Implemented on `work/bug-8a78e1` at `6b2118da1` (option a); cargo verification deferred to the batch check.
  `graph_execution/feedback.rs` is deleted: `build_settler`, its 12 sinks and `CompletionSinkResult`. Only their own
  tests called them. The feedback facade and dispatch's records cover what the rows did on the Graph path.
