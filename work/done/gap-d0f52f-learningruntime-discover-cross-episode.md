+++
id = "gap-d0f52f"
kind = "gap"
title = "LearningRuntime::discover_cross_episode_patterns has no caller, so EpisodeView::succeeded has no production reader"
status = "done"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["roko-learn/runtime_feedback", "roko-learn/pattern_discovery"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "db778db9b"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-settle's report on gap-88c547, branch work/gap-88c547 at 24b23580d)"
anchors = ["crates/roko-learn/src/runtime_feedback/mod.rs", "crates/roko-learn/src/pattern_discovery.rs"]
lane = "rust-cold"
parent = "spec-9a3131"
links = { depends_on = ["gap-88c547"], blocks = [], related = ["gap-88c547"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -rq 'fn discover_cross_episode_patterns' crates/ || grep -rn --include='*.rs' 'discover_cross_episode_patterns(' crates/ | grep -v 'fn discover_cross_episode_patterns' | grep -q ."

[closed]
at = 2026-10-01
at_ts = "2026-10-01T18:45:51Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-01T16:12:53Z"
forced = false
evidence = "Gate 6b on 9e32a0d64, merged as db778db9b (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 11 crates; lib tests pass (roko-cli 3375, roko-agent 2241, roko-core 1971, roko-learn 1216, roko-serve 1003, roko-gate 696, roko-compose 561, roko-graph 483, roko-execution 245, roko-acp 200); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, gemini, dispatch-feedback, e2e_domain, run_serve_share, property, job_runner and plan_execute integration tests pass; bin 447; Cargo.lock unchanged; portal tsc clean and vitest 800/800. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++

## Problem

On gap-88c547's branch, `LearningRuntime::discover_cross_episode_patterns` (`crates/roko-learn/src/runtime_feedback/mod.rs:1111`) has no caller. The `EpisodeView` trait's `succeeded` (`pattern_discovery.rs:57`, implemented for the runtime's episodes at `runtime_feedback/mod.rs:127`) is read only through that discovery, so it has no production reader either. The branch updated both to the settled label, but nothing runs them.

## Why it matters

Hygiene (epic spec-9a3131): dead code that looks wired gets maintained, and misleads readers about which learning loops run.

## Where

The two anchors.

## Plan

Pick one:

- **(a) Wire it:** call pattern discovery from the offline consolidation or dream cycle, where cross-episode patterns belong.
- **(b) Delete it:** remove `discover_cross_episode_patterns`, and the `EpisodeView` implementation if nothing else uses it.

## Done when

- [ ] The function is called from production code, or it is gone.
- [ ] The `[[verify]]` command passes.

## Notes

- Build on gap-88c547's branch.
- 2026-10-01 (wk-learn2): implemented on work/gap-14f08e (option b); cargo verification deferred to the batch check.
  `LearningRuntime::discover_cross_episode_patterns` is deleted, with its two now-unused imports. Option (a) already
  exists: the dream cycle runs its own `CrossEpisodeConsolidator` pass (`roko-dreams/src/cycle.rs::build_cross_episode_report`),
  and the TUI another (`tui/dashboard_model.rs`), so the wrapper was a third, unused entry point.
- The title's "so" does not hold: the wrapper called `CrossEpisodeConsolidator::discover(&[Episode])`, which reads
  `Episode::learning_success()` and never goes through `EpisodeView`. `EpisodeView::succeeded` had no reader before
  and has none now: `PatternMiner::ingest_episode` reads only `actions()`. The `EpisodeActions` impl stays, since the
  trigram miner uses it. Dropping `succeeded` from the public trait (its doctest, the test `Ep`, `EpisodeActions.success`)
  is a separate API cleanup, left undone.
