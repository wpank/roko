+++
id = "bug-8a78e1"
kind = "bug"
title = "chat, dispatch_v2 and serve load the cascade router with load_or_new, so a crashed run's journal replays only when a LearningRuntime opens"
status = "done"
triage = "verified"
severity = "p2"
goal = "cybernetic"
size = "S"
subsystem = ["roko-cli/chat", "roko-cli/dispatch_v2", "roko-serve/dispatch"]
created = 2026-09-30
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "aec267cac"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-settle's report, checked on work/bug-f81e9b at 7db865c81)"
anchors = ["crates/roko-cli/src/chat_session.rs", "crates/roko-cli/src/dispatch_v2.rs", "crates/roko-serve/src/dispatch.rs", "crates/roko-learn/src/model_call_feedback.rs"]
lane = "rust-hot"
parent = "spec-6ac537"
links = { depends_on = [], blocks = [], related = ["bug-f81e9b", "bug-84de98", "bug-7a2630"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -q 'CascadeRouter::load_or_new' crates/roko-cli/src/chat_session.rs crates/roko-cli/src/dispatch_v2.rs crates/roko-serve/src/dispatch.rs"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T13:44:17Z"
by = "coordinator (session 7622b882)"
forced = false
evidence = "Batch 20e gate on 1a8aad603, re-checked with the compile fixes (cf722c1be, bed29287d), tiers' rustfmt (8a6c932ce) and the run-index scrub fix (d972959bd) on 32fe02384; MAIN aec267cac has the same crates: check --workspace --tests, nightly fmt and clippy -D warnings clean on roko-acp/agent/cli/core/dreams/fs/gateway/graph/learn/neuro/serve/std; lib tests roko-cli 3305, roko-agent 2294, roko-core 1962, roko-learn 1213, roko-serve 991, roko-graph 478, roko-fs 260, roko-neuro 239, roko-std 229, roko-acp 199, roko-dreams 100, roko-gateway 42 all pass; extras: golden_path_suite 2/2, all eight canaries pass (secret_canary 11/11 and C2 2/2 after the scrub fix), worktree_task_diff 2/2, plan_run_config_flag 1/1, default_engine 1, bin 429, routing crash loop 10/10, bench driver 18, including chat_routes_with_what_a_crashed_writer_journaled, a_recorder_routes_with_what_a_crashed_writer_journaled and a_serve_observation_lands_on_a_crashed_writers_journal. Merged f68c5673c (work/bug-8a78e1 4fd72f112)."
+++

## Problem

The cascade router's journal segments from a crashed run are replayed by `load_recovered_router` (`crates/roko-learn/src/model_call_feedback.rs:459`), which only a `LearningRuntime` open calls. `roko chat` (`chat_session.rs:85`), `dispatch_v2` (`dispatch_v2.rs:138`) and serve (`crates/roko-serve/src/dispatch.rs`, :2851, :3472, :3693) load the router with `CascadeRouter::load_or_new`, which reads only the snapshot. Until some process opens a `LearningRuntime`, those surfaces route without the crashed run's learning, and the observations they make are saved on top of the stale snapshot.

## Why it matters

Cybernetic core (epic spec-6ac537): learning survives a crash only on some paths. On the others it's silently missing, and it can later be merged inconsistently.

## Where

The `load_or_new` calls listed above.

## Plan

1. Switch them to `load_recovered_router` (or one shared loader that replays the journal), keeping each surface's model list.
2. Add a test that a journal left by a crashed writer is replayed when chat, dispatch_v2 or serve loads the router.

## Done when

- [ ] Every surface that loads the router replays unsaved journal segments.
- [ ] The `[[verify]]` command passes.

## Notes

- Implemented on `work/bug-8a78e1` at `86f916d3a`; cargo verification deferred to the batch check.
  Every router writer now loads with `load_recovered_router`. That covers chat, dispatch_v2, serve_runtime's bench
  dispatch, `ModelCallFeedbackRecorder::from_learn_dir` (chat's recorder, the vision loop, serve's template
  dispatch), serve's shared router and the gateway's cached one, `record_cascade_router_observation`, and ACP's
  routing pick and observation recorder.
- Left on `load_or_new`, since recovery writes: the read-only route explanations in `roko config models route` and
  serve's providers endpoint. Also `model_experiment`'s role-table update, which merges into the snapshot on save.
