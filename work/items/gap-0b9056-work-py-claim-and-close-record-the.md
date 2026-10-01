+++
id = "gap-0b9056"
kind = "gap"
title = "work.py claim and close record the executor, pick-up route, size and claim time; release records a reason"
status = "done"
triage = "verified"
severity = "p1"
goal = "proof"
size = "S"
subsystem = ["tools/work"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "6e90df1e0"
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e13"
discovered_from = "tmp/cybernetic-harness/workstreams/assessment/W12-evidence-from-dev-process.md (F1, A2)"
anchors = ["tools/work.py::cmd_claim", "tools/work.py::cmd_release", "tools/work.py::close_item", "tools/work.py::cmd_sync", "tools/test_work.py"]
lane = "tracker"
parent = "spec-f2463d"
links = { depends_on = ["gap-d0643c"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_close_copies_claim_fields_into_closed' tools/test_work.py && grep -qw 'def test_release_records_its_reason' tools/test_work.py && python3 tools/test_work.py -k test_close_copies_claim_fields_into_closed -k test_release_records_its_reason"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T08:14:57Z"
commit = "6e90df1e0"
by = "wk-gates"
executor = "claude-agent"
via = "work-batch"
model = "claude-opus-5-5"
forced = false
evidence = "6e90df1e0: claim --executor/--via/--size (size defaults to the item's) stores them and claimed_at in the claim and the claim event; release --reason records the reason in the release event; close copies the claim's executor/via/size/claimed_at into [closed] (claim read from the main checkout, so a worktree close gets them) plus at_ts, forced, --model, --assist; sync closures get executor roko-plan (plan) or the claim's/unknown (commit) and a reconciled closed event; check validates the new optional fields (1498 items, 0 problems). tools/test_work.py: claim with the three flags then close in a second worktree fills the four claim fields and at_ts; release --reason blocked is recorded; sync executors; the [[verify]] command passes."
+++

## Problem

A claim records only `by`, `at`, `branch` and `worktree`, and closing the item deletes it. `[closed]` then keeps a
date, `commit`, `run_id`, a free-text `by` and `evidence`. Afterwards nobody can tell who executed an item (a Claude
worker, a session, a Roko plan or Will), how it was picked up (`/work-batch`, `/work-next`, by hand), how big it was
judged before work started, or how long it took. A release records no reason.

## Why it matters

Goal `proof`, epic spec-f2463d. Executor share, cost per merged item by executor and claim-to-merge time (W12 F1)
need these fields. Size must be fixed at claim time; set afterwards, it gets fitted to the outcome.

## Where

- `tools/work.py`: `cmd_claim`, `cmd_release`, `close_item` (writes `[closed]`), `cmd_sync` (closures from
  trailers and plan tasks), and their parsers in `main`.
- Tests: `tools/test_work.py`.

## Current state

Checked at `41c7ffbd6`: none of these fields exist. `close_item` deletes the claim only in the main checkout, so a
worker closing in its worktree leaves the claim for `prune_claims`. The event log comes from gap-d0643c.

## Plan

1. `claim`: add `--executor claude-agent|claude-session|roko-plan|human`,
   `--via work-batch|work-next|manual|roko-plan` and `--size S|M|L` (default: the item's `size`). Store them and
   `claimed_at` in the claim file and the `claim` event.
2. `release`: add `--reason verify-fail|blocked|decision-needed|conflict|timeout|session-limit`, recorded in the
   `release` event.
3. `close`: read the claim from the main checkout's claims directory and copy `executor`, `via`, `size` and
   `claimed_at` into `[closed]`. Add `at_ts` (a timestamp), `--model` and `--assist` (a second executor that helped).
4. `sync`: plan closures get `executor = "roko-plan"`; commit closures without a claim get `executor = "unknown"`
   and a `reconciled` event.
5. Keep `by`. All new fields are optional, so existing items still validate.

## Done when

- [ ] Claim with the three flags, then close in a second worktree: `[closed]` holds the four claim fields and `at_ts`.
- [ ] `release --reason blocked` records the reason.
- [ ] `check` passes on today's items unchanged.
- [ ] The `[[verify]]` command passes.

## Notes

- The skills start passing these flags in gap-92033c; until then they stay optional.
- `claims_dir()` already resolves the main checkout from a worktree.
