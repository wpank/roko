+++
id = "gap-2ce86f"
kind = "gap"
title = "Nothing on the Graph path writes learn/error-patterns.json: build_settler, which holds the ErrorPatternSink, has no production caller"
status = "open"
triage = "verified"
severity = "p2"
goal = "cybernetic"
size = "S"
subsystem = ["roko-cli/graph_execution"]
created = 2026-09-30
updated = 2026-09-30
last_verified = 2026-09-30
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-telemetry2's report, branch work/gap-8cb382 at c5090e9a5)"
anchors = ["crates/roko-cli/src/graph_execution/feedback.rs::build_settler", "crates/roko-learn/src/error_pattern_store.rs"]
lane = "rust-hot"
parent = "spec-6ac537"
links = { depends_on = [], blocks = [], related = ["gap-8cb382", "gap-eb82c9"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_failed_graph_attempt_updates_the_error_pattern_store' crates/roko-cli/src/ && cargo test -p roko-cli --lib a_failed_graph_attempt_updates_the_error_pattern_store"
+++

## Problem

`crates/roko-cli/src/graph_execution/feedback.rs::build_settler` (:31) assembles a `FeedbackSettler` whose sinks include `ErrorPatternSink`. For every attempt that didn't succeed, that sink loads `.roko/learn/error-patterns.json`, records the gate failure (`observe_gate_failure`) and saves it (:416 onward on `work/gap-8cb382`). But `build_settler` is called only from the file's own tests (:802-869, after `#[cfg(test)]` at :766). No production code builds this settler, so a Graph run never writes `learn/error-patterns.json`. wk-telemetry2's loop census found the file absent after runs with failed gates.

## Why it matters

Cybernetic core (epic spec-6ac537): error patterns feed prompt enrichment and diagnosis. On the Graph path, the only plan executor, that loop is built but not wired.

## Where

`build_settler` and `ErrorPatternSink` (`graph_execution/feedback.rs`), `roko-learn/src/error_pattern_store.rs`, and the plan runner that should settle each attempt.

## Plan

1. Find where Graph attempts settle today (`graph_task_dispatch/feedback.rs` and the settlement bus) and either call `build_settler` there, or move `ErrorPatternSink` into the settler that does run. Avoid two settlers.
2. Add `a_failed_graph_attempt_updates_the_error_pattern_store`.

## Done when

- [ ] A Graph attempt that fails its gate adds to `learn/error-patterns.json`.
- [ ] The `[[verify]]` command passes.

## Notes

- Prepared on `work/bug-f81e9b` at `bcb9da2ee`; cargo verification deferred to the batch check.
  `runtime_feedback::ErrorPatternSink` records each attempt whose settled learning label is a failure into the
  dispatch factory's shared `ErrorPatternStore`, the one `plan_runner.rs` loads from disk and prompts read, and
  saves it to `learn/error-patterns.json`. The key is the failure class plus the normalized first failing step, so
  a recurring failure merges into one pattern.
- **Still to do:** register the sink in `plan_runner.rs`'s facade block, after batch 12 merges. That wiring is
  `ErrorPatternSink::new(Arc::clone(shared_factory.error_pattern_store()), graph_learn_dir.join("error-patterns.json"))`.
- The receipt settler's `error_pattern` row stays: `FeedbackSettler::new` asserts all 12 `SINK_KEYS`, and
  `build_settler` has no production caller, so the facade holds the one writer that runs.
