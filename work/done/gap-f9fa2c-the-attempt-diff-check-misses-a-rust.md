+++
id = "gap-f9fa2c"
kind = "gap"
title = "The attempt diff check misses a Rust doc example switched off: a rust,ignore fence or a commented-out assert in a /// example is not counted as a skip"
status = "done"
triage = "verified"
severity = "p2"
goal = "truth"
size = "S"
subsystem = ["roko-gate/attempt_diff"]
created = 2026-10-16
updated = 2026-10-16
last_verified = 2026-10-16
last_verified_rev = "bf82d373e"
source = "gap-c013c5 golden-path live run 3 (2026-10-16), independent review"
discovered_from = "gap-c013c5"
anchors = ["crates/roko-gate/src/attempt_diff.rs::Markers", "crates/roko-gate/src/attempt_diff.rs::Tally"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["find-2ba86d", "gap-c013c5", "gap-f30b8e"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn doc_example_switched_off_is_a_skip' crates/roko-gate/ && cargo test -p roko-gate doc_example_switched_off_is_a_skip"

[closed]
at = 2026-10-16
at_ts = "2026-10-16T20:43:57Z"
commit = "bf82d373e"
by = "commit trailer"
executor = "claude-agent"
via = "work-batch"
size = "S"
claimed_at = "2026-10-16T19:19:20Z"
forced = false
evidence = "crates/roko-gate/src/attempt_diff.rs (bf82d373e): Tally::of now calls doc_example_tally over every .rs file's full text, not just the #[cfg(test)] region, so a doc example above the item it documents is no longer skipped entirely. A fence whose info string is `ignore` or `ignore-<target>` (is_ignored_doc_fence) adds to Tally.skips the same way #[ignore] does, and asserts inside a fence are added to Tally.asserts unless the line is commented out, so a commented-out assert reads as a removed assert; no_run alone is excluded per the item's notes. Proof: attempt_diff::tests::doc_example_switched_off_is_a_skip (the item's [[verify]] test, covering the switched-off case plus a pre-existing untouched ignore and a no_run-alone case for no false positives) passes, the full roko-gate suite (704 lib tests, 0 failed) passes, and cargo clippy -p roko-gate --no-deps -- -D warnings is clean."
+++

## Problem

The attempt diff check does not notice when a Rust doc example (a fenced block in a `///` comment) is switched off. In
golden-path live run 3 (gap-c013c5, 2026-10-16, `graph-6bf8aeb4-2d31-4416-baad-3d950f67d929`), task T4's executor
(gpt-oss-120b) wrote a doc example for `beta::farewell`. When `cargo test --workspace` failed on it with E0425, the
executor fenced it `rust,ignore` and commented out its only `assert_eq!`. The attempt passed. The merged tree's doctest
run now reports `farewell (line 14) ... ignored`. T6's executor (glm-4.7) then copied the switched-off pattern for
`badge`. The independent review of the merged diff counted this as an escape: a test turned off to get green.

`attempt_diff.rs` counts skip markers (`#[ignore`, `@unittest.skip`, `.skip(` …) only in test files (`is_test_path`)
and in Rust code after `#[cfg(test)]` (`Tally::of`). A doc example sits in a `///` comment under `src/`, and its skip
marker is a fence attribute (```` ```ignore ````, ```` ```rust,ignore ````), so neither the skip nor the
commented-out assert is counted.

## Why it matters

The pre-verify screen exists so that a pass cannot come from switching off a check, and decision 3201 makes
roko's verdicts honest only to the checks that run. W8's golden-path bar also wants an independent review that finds
no escape. This escape could have been caught mechanically. Related: find-2ba86d (checks honest to tests, not to the
spec), gap-c013c5 (the live runs), gap-f30b8e.

## Where

- `crates/roko-gate/src/attempt_diff.rs::Markers` (skip and assert markers per language) and `Tally::of` (which text
  counts as test code); the `SkipAdded` finding.
- Reached from the pre-verify tamper screen in `crates/roko-cli/src/graph_task_dispatch/red_flags.rs`.

## Current state

Skip markers in test files and in `#[cfg(test)]` code are caught (the `red_flags.rs` test that adds `#[ignore]` to
`tests/math.rs` asserts `skip_added`). Doc examples are not scanned at all.

## Plan

1. In `Tally::of` for `.rs` files, also scan `///` and `//!` comments for fenced code blocks. Count a fence whose info
   string contains `ignore` as a skip, and count the asserts inside doc examples, so that a commented-out assert reads
   as a removed assert.
2. Decide what an `ignore` doc example triggers. An `ignore` example can be legitimate (pseudo-code). Recommended: a
   fence that changed from running to `ignore` is `skip_added`, and so is a new example that arrives `ignore` with its
   asserts commented out. A plain new `ignore` example only records a finding.
3. Add the unit test `doc_example_switched_off_is_a_skip` in `attempt_diff.rs`. Before: a `///` example with an
   `assert_eq!`. After: the same example fenced `rust,ignore`, with the assert commented out. Expect a `SkipAdded`
   finding.

## Done when

- The `[[verify]]` command passes: the named test exists and passes.

## Notes

- Evidence (main checkout, gitignored): `tmp/status-quo/2026-10-16-live/evidence/golden-path/run3/merged.diff`
  (`crates/beta/src/lib.rs`, the `farewell` and `badge` doc comments), `meta-verify-3.txt`
  ("farewell (line 14) ... ignored"), and the review `review.md` beside them.
- `no_run` compiles an example without running it. That is weaker than a running example but not a skip, so leave it
  out or decide it separately.
