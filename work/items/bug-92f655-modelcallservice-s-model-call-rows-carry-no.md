+++
id = "bug-92f655"
kind = "bug"
title = "ModelCallService's model_call rows carry no model_reported and no attempt key"
status = "open"
triage = "unverified"
severity = "p2"
goal = "truth"
size = "S"
subsystem = ["roko-agent/model_call_service"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-model-truth's report on branch work/bug-31438d at ee6a541ef)"
anchors = ["crates/roko-agent/src/model_call_service.rs"]
lane = "rust-hot"
parent = "spec-b7303f"
links = { depends_on = ["bug-31438d"], blocks = [], related = ["bug-31438d", "bug-62e3f4"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn model_call_rows_carry_the_reported_model_and_the_attempt_key' crates/roko-agent/src/ && cargo test -p roko-agent --lib model_call_rows_carry_the_reported_model_and_the_attempt_key"
+++

## Problem

The model-call bridge in `crates/roko-agent/src/model_call_service.rs` records calls under the role `model_call` when the request names no role (:512, :716, :834 on bug-31438d's branch). On that branch the file has no `model_reported` and no `attempt_key` anywhere. So these rows can't say which model served the call, or which attempt it belongs to, while the Graph path's records now say both.

## Why it matters

One settled record per attempt (epic spec-b7303f): calls made through the service (serve, helpers, chat) can't be joined to attempts, or checked against a pin.

## Where

The row construction in `model_call_service.rs`.

## Plan

1. Carry `model_reported` (from the response's usage) and the caller's attempt key (None when there is no attempt) into every row the service writes.
2. Add `model_call_rows_carry_the_reported_model_and_the_attempt_key`.

## Done when

- [ ] Every `model_call` row names the reported model (or unknown) and its attempt key, when it has one.
- [ ] The `[[verify]]` command passes.

## Notes

- Build on bug-31438d's branch.
