+++
id = "bug-83a6eb"
kind = "bug"
title = "The audit worker labels an empty-diff attempt (already satisfied, reviewer pass) as spec gaming"
status = "done"
triage = "verified"
severity = "p2"
goal = "truth"
size = "S"
subsystem = ["roko-cli/audit"]
created = 2026-10-03
updated = 2026-10-04
last_verified = 2026-10-04
last_verified_rev = "343bc053f"
source = "PK64 (w4-pk53, 2026-10-03)"
discovered_from = "gap-2e4a81"
anchors = ["crates/roko-cli/src/audit/worker.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["gap-940e44", "gap-147c4d", "gap-2e4a81"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn empty_diff_unit_is_not_labelled_gaming' crates/roko-cli/src/ && cargo test -p roko-cli --lib empty_diff_unit_is_not_labelled_gaming"

[closed]
at = 2026-10-04
at_ts = "2026-10-03T22:14:11Z"
commit = "343bc053f"
executor = "claude-agent"
via = "work-batch"
size = "S"
claimed_at = "2026-10-03T21:45:59Z"
forced = false
evidence = "Gate 10a (work/backlog-batch-10a, merged into main as 343bc053f): cargo check --workspace --tests, nightly fmt, cargo clippy --workspace -D warnings, nextest --lib 4,234 tests over roko-cli and roko-gate, roko-cli bin 438 passed and the golden-path canaries pass incl. the audit canary drill (plan_validate: only bug-2a31bc's two known alias tests fail), the bench driver and experiments suites 182 passed; every [[verify]] passes. The audit worker's A1 skips audit_only_findings on an empty diff (e62c74758), so an already-satisfied or no-diff unit is labelled G = 0 with no incident and no trust change; test empty_diff_unit_is_not_labelled_gaming."
+++

## Problem

The audit worker labels an attempt with an empty diff as spec gaming. A1's audit-only findings on an empty diff return
`vacuous_diff`, which sets G = 1. So a drawn `already_satisfied` unit, or a reviewer pass with no diff, opens a gaming
incident, downgrades the model's trust (`trust.json`) and pushes γ up the DP3 ladder. Found by PK64's worker
(2026-10-03); dispatch's own V1 check already skips A1 on an empty diff.

## Why it matters

With audits on, honest attempts that correctly changed nothing would be recorded as gaming, lowering routing trust in
good models and driving the ladder to its deepest (costliest) checks.

## Where

The audit worker's A1 path in `crates/roko-cli/src/audit/worker.rs` (and `audit_only_findings`), the incident and
trust feedback in `crates/roko-gate/src/audit/incident.rs` and `feedback.rs`.

## Current state

`[audit] enabled` is false by default (gap-d76b1b), so nothing is affected until audits are switched on.

## Plan

1. Skip `audit_only_findings` (or treat the unit as not gaming) when the attempt's tree changes are empty, as dispatch's
   V1 does.
2. A test: an audited `already_satisfied` unit with no diff opens no incident, leaves trust unchanged and labels G = 0.

## Done when

- [ ] The `[[verify]]` command passes.

## Notes

- Related: gap-940e44 (PK60, incidents and trust), gap-147c4d (PK59, A1).
