+++
id = "gap-4468bd"
kind = "gap"
title = "Credit or demote a T0 reflex rule only from the settled attempt record, after verify"
status = "open"
triage = "unverified"
severity = "p2"
goal = "truth"
size = "M"
subsystem = ["roko-cli/graph-task-dispatch", "roko-learn"]
created = 2026-09-29
updated = 2026-09-30
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (16:10, wk-gtd-split's report on bug-94151f)"
anchors = ["crates/roko-cli/src/graph_task_dispatch.rs", "crates/roko-learn/src/reflex_store.rs::record_gate_pass_for", "crates/roko-learn/src/reflex_store.rs::record_gate_fail_for"]
lane = "rust-hot"
parent = "spec-b7303f"
links = { depends_on = ["gap-96f7ed", "bug-94151f"], blocks = [], related = ["spec-e9d7ec", "gap-1f2661"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn reflex_rule_is_credited_from_the_settled_attempt' crates/roko-cli/src/ && cargo test -p roko-cli --lib reflex_rule_is_credited_from_the_settled_attempt"
+++

## Problem

bug-94151f (`9a4d8db5c` on `work/bug-94151f`) made two changes to the reflex path:

- it removed the premature `record_gate_pass_for` call;
- it put the path behind `[learning] t0_reflexes`, off by default.

A reflex now serves only tasks without verify steps, and their output stays `Unverified`. The item's plan step 2 was deferred, so nothing credits or demotes a rule afterwards. With the flag on, the T0 reflex store never learns which rules are good.

## Why it matters

The reflex loop is one of the learning loops the papers count, and it has to learn from verified outcomes, not from its own firing (epics spec-b7303f and spec-e9d7ec). The settled attempt record from gap-96f7ed is the one place where a verified outcome exists.

## Where

- `crates/roko-cli/src/graph_task_dispatch.rs`: the reflex check in dispatch. It calls `reflex_store.match_observation_with_id`, filtered on `t0_reflexes` and `task.verify.is_empty()`.
- `crates/roko-learn/src/reflex_store.rs`:
  - `ReflexStore::record_gate_pass_for`;
  - `ReflexStore::record_gate_fail_for`, which returns `true` when the failure deletes the rule.

## Current state

Blocked on gap-96f7ed, which threads the attempt context through dispatch and settles one outcome per attempt.

## Plan

Design choice, to settle first: what counts as "verified" for an attempt that a reflex served.

- **Option A (recommended):** a reflex skips only the provider. A task with verify steps that a reflex serves still runs its verify steps and gates, on the reflex output.
- **Option B:** keep today's filter, and credit from the gate verdict of the settled attempt, if gates run for tasks without verify steps.

Steps:

1. Carry the rule id in the attempt context (gap-96f7ed).
2. When the attempt settles, call `record_gate_pass_for` on a verified pass and `record_gate_fail_for` on a failure. A failed reflex attempt retries through the provider.
3. Tasks without verify steps keep today's behaviour: output `Unverified`, and no credit.
4. Add `reflex_rule_is_credited_from_the_settled_attempt`. With a matching rule, a task whose verify passes gives the rule one pass; a task whose verify fails gives it one fail and no pass.

## Done when

- [ ] A reflex rule's pass and fail counts change only from settled attempt outcomes.
- [ ] The `[[verify]]` command passes.

## Notes

- Keep the path behind `t0_reflexes`.
- `graph_task_dispatch.rs` is a hot file. Start after the dispatch-file split (gap-c8e1f1) has merged.
- 2026-09-30 (wk-gates, wk-settle): since bug-b4c565, T0 reflexes serve only tasks that nothing verifies, so this item's credit hook never fires in live runs. Whether reflexes should serve checked tasks, with verification run on their cached output and first attempts only, is dec-af63cc (recommended: keep them narrow until there's evidence; revisit with M1–M4).
