+++
id = "bug-0ba3d9"
kind = "bug"
title = "Attempt records leave inv null: with_inv is called only in tests"
status = "open"
triage = "verified"
severity = "p2"
goal = "truth"
size = "S"
subsystem = ["roko-learn/telemetry", "roko-cli/graph_task_dispatch"]
created = 2026-09-30
updated = 2026-09-30
last_verified = 2026-09-30
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-telemetry2's report, branch work/gap-8cb382 at c5090e9a5)"
anchors = ["crates/roko-learn/src/telemetry/records.rs", "crates/roko-learn/src/telemetry/manifest.rs"]
lane = "rust-hot"
parent = "spec-b7303f"
links = { depends_on = ["gap-8cb382"], blocks = [], related = ["gap-8cb382", "bug-d5fb74", "gap-dad97b"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn attempt_records_carry_the_invocation_ordinal' crates/roko-cli/src/ && cargo test -p roko-cli --lib attempt_records_carry_the_invocation_ordinal"
+++

## Problem

On gap-8cb382's branch, S01 attempt records have an `inv: Option<u32>` field (`roko-learn/src/telemetry/records.rs:239`) that is `None` by default (:256) and set by `with_inv` (:268-274). `with_inv` is called only in that file's tests (:1110, :1118). The run manifest numbers each invocation (`manifest.rs:74`, `invocation.inv = self.next_inv()`), but attempt records never get the number. So every attempt record has `inv: null`.

## Why it matters

One settled record per attempt (epic spec-b7303f): after a resume, `inv` is the only way to tell which invocation, and so which build and config, an attempt ran under. The benchmark's Roko arm needs the same join (gap-dad97b).

## Where

The places that build attempt records on the Graph path, and the manifest's current invocation number.

## Plan

1. Pass the current invocation's number into the attempt context, and call `with_inv` wherever attempt-open, verdict and settled records are built.
2. Add `attempt_records_carry_the_invocation_ordinal`, including a resumed run.

## Done when

- [ ] Every attempt record carries its invocation's number.
- [ ] The `[[verify]]` command passes.

## Notes

- Build on gap-8cb382's branch.
- Implemented on `work/bug-0ba3d9` at `09906922e`; cargo verification deferred to the batch check. On the branch, `attempt_records_carry_the_invocation_ordinal` passes: a failed run resumed, with attempts 1 and 2 under invocations 1 and 2. So do `graph_plan_run_writes_run_manifest` and the `graph_task_dispatch::attempt` tests. Check, clippy with `-D warnings` and nightly fmt are clean.
- `run_one_plan` passes the ordinal from `RunManifests::open` to `GraphTaskDispatcher::attach_run_invocation`, and `AttemptBook::open` stamps it on every identity. The attempt-open line, the verdict and the settled record all carry it. The `attempt.rs` change is purely additive.
