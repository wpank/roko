+++
id = "gap-c22583"
kind = "gap"
title = "A Graph attempt's verdict record leaves request_sha256 and diff_sha256 empty: only its output hash is ever set"
status = "open"
triage = "verified"
severity = "p3"
goal = "learning"
size = "S"
subsystem = ["roko-cli/graph_task_dispatch", "roko-learn/telemetry"]
created = 2026-10-16
updated = 2026-10-16
last_verified = 2026-10-16
source = "paper-review-2026-10-16 (gap-aed6f1)"
discovered_from = "gap-aed6f1"
anchors = ["crates/roko-learn/src/telemetry/records.rs::AttemptVerdictRecord", "crates/roko-cli/src/graph_task_dispatch/attempt.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["gap-aed6f1"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn verdict_record_carries_request_and_diff_hashes' crates/roko-cli/ && cargo test -p roko-cli verdict_record_carries_request_and_diff_hashes"
+++

## Problem

`AttemptVerdictRecord` (`crates/roko-learn/src/telemetry/records.rs`) declares three hashes: `request_sha256` ("sha256
of the provider request"), `output_sha256` and `diff_sha256` ("sha256 of the diff the attempt produced").
`AttemptVerdictRecord::settle` starts all three empty, and on the Graph path only `output_sha256` is ever filled
(`crates/roko-cli/src/graph_task_dispatch/attempt.rs:750` at `5e32264ce`). `git grep -n 'request_sha256\|diff_sha256'
-- crates/` at `1b8b62fb9` finds only the two declarations and the constructor, so every verdict line in
`.roko/runs/<run>/attempts.jsonl` carries `"request_sha256": null` and `"diff_sha256": null`.

## Why it matters

The verdict record is the one fact that the learning loops and the audits read. Without the request and diff hashes a
record cannot be tied to the exact request that produced it or to the diff that its checks judged, so an audit or a
replay of an attempt has to trust paths and timestamps instead. The research paper (*Loops Within Loops*, §4.3 and
Table C.3) said the record carries both; its 2026-10-16 review (gap-aed6f1) changed the paper to the output hash only.
Decision receipts already carry a request hash (`DecisionReceipt` in the same file).

## Where

- `crates/roko-learn/src/telemetry/records.rs::AttemptVerdictRecord`: the fields and `settle`.
- `crates/roko-cli/src/graph_task_dispatch/attempt.rs`: where the verdict is filled before it settles (the output hash).
- The request: the bytes the decision receipt hashes (`DecisionReceipt` request hash), or the provider request as sent.
- The diff: the attempt's diff against its starting tree, which the pre-verify screen already reads
  (`crates/roko-gate/src/attempt_diff.rs`).

## Current state

Only `output_sha256` is set. The two empty fields are `#[serde(default)]` without `skip_serializing_if`, so every record
prints them as null.

## Plan

1. In `attempt.rs`, before the verdict settles, set `request_sha256` from the same bytes the decision receipt hashes
   and `diff_sha256` from the attempt's diff against its starting tree.
2. Where a hash cannot be computed for a provider kind (for example a CLI agent given a prompt file), say in the field's
   doc what it hashes instead, or leave it empty and record why.
3. Add the test `verdict_record_carries_request_and_diff_hashes`: settle one attempt through a fake provider, check that
   both hashes are set, and that identical inputs give identical hashes.

If the fields are not wanted after all, delete them and their docs instead, and change this item's verify.

## Done when

The `[[verify]]` command passes: the named test exists and passes.

## Notes

Found by the research-paper review (gap-aed6f1) at `5e32264ce`; still true at `1b8b62fb9`. Do not change what
`output_sha256` hashes.
