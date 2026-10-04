+++
id = "gap-71c60a"
kind = "gap"
title = "An audit unit rebuilt from audit.selection alone has no prediction_id: the event doesn't carry one"
status = "done"
triage = "verified"
severity = "p2"
goal = "cybernetic"
size = "S"
subsystem = ["roko-cli/audit"]
created = 2026-10-04
updated = 2026-10-04
last_verified = 2026-10-04
last_verified_rev = "2539f2c77"
source = "wave-14 follow-up reports 2026-10-04 (gap-45c8fe, gate 14b)"
discovered_from = "gap-45c8fe (in flight on work/gap-3cd890; fixes the normal path, not this fallback)"
anchors = ["crates/roko-gate/src/audit/ledger.rs::AuditEvent", "crates/roko-cli/src/audit/worker.rs::AuditUnit"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn unit_rebuilt_from_selection_alone_keeps_its_prediction_id' crates/ && cargo test -p roko-cli unit_rebuilt_from_selection_alone_keeps_its_prediction_id"

[closed]
at = 2026-10-04
at_ts = "2026-10-04T08:38:28Z"
commit = "2539f2c77"
executor = "claude-agent"
via = "work-batch"
size = "S"
claimed_at = "2026-10-04T06:54:02Z"
forced = false
evidence = "Gate 15b (merged 2539f2c77): verify unit_rebuilt_from_selection_alone_keeps_its_prediction_id passes; roko-gate test suites pass. audit.selection carries prediction_id (omitted when None, so older records parse and re-hash unchanged) and from_selection reads it back."
+++

## Problem

An audit unit rebuilt from its `audit.selection` ledger record (because its vault queue file is
missing) has no `prediction_id`, since the selection event doesn't carry one. This is a residual
gap in `gap-45c8fe`'s own fix (in flight on `work/gap-3cd890`, not yet merged): that branch adds
`AuditUnit.prediction_id: Option<String>`
(`crates/roko-cli/src/audit/worker.rs:132-134`, doc comment: "`None` without one, **or for a
unit taken up from its selection alone**") and threads it into `vs.label` rows
(`crates/roko-cli/src/audit/labels.rs::vs_label`), fixing `gap-45c8fe`'s original "always null"
bug for the normal path. But `AuditWorker::pending`
(`crates/roko-cli/src/audit/worker.rs:507-527`) builds each unit as
`load_unit(&queue, &unit.sel_id).unwrap_or(unit)` — it reads the full queued `AuditUnit` JSON
from the vault's queue directory when present, and falls back to
`AuditUnit::from_selection(record)` (worker.rs:135-166) when that file is missing or unreadable.
`from_selection` destructures `AuditEvent::Selection` (`crates/roko-gate/src/audit/ledger.rs:68-88`:
`sel_id, attempt_key, run_id, task_id, stratum, pi, prf_u, selected, base_tree, result_tree` — no
`prediction_id` field at all) and hardcodes `prediction_id: None` (worker.rs:165), because there
is nothing on the event to recover it from. So a unit that survives via its queue file keeps its
real `prediction_id`; the same unit, rebuilt after its queue file is lost, silently loses it.

## Why it matters

Goal: cybernetic, M3/M4 interface (S04/S05), same goal as `gap-45c8fe`. The whole point of
`gap-45c8fe`'s fix is to let the audit see M3's own forecast per attempt. That guarantee quietly
breaks exactly when the vault queue file is missing — an operational failure mode (crash,
cleanup, disk pressure) rather than a rare edge case — silently degrading back to the "always
null" behavior `gap-45c8fe` set out to fix, for exactly the units whose queue data was lost.

## Where

- `crates/roko-gate/src/audit/ledger.rs::AuditEvent::Selection` (the event schema; needs a
  `prediction_id: Option<String>` field).
- `crates/roko-cli/src/audit/worker.rs::AuditUnit::from_selection`, `::pending` (the rebuild
  fallback; needs to read the new field instead of hardcoding `None`).
- Whatever writes the `audit.selection` event in the first place (needs to populate the new
  field from the same source `AuditUnit.prediction_id` is populated from).

## Current state

On `main`, `AuditUnit` has no `prediction_id` field yet (`gap-45c8fe` is unfixed there). On
`work/gap-3cd890`, the field exists and the normal (queue-file-present) path carries it
correctly; the fallback (queue-file-missing) path does not, because the event it rebuilds from
was never given the field.

## Plan

1. Add `prediction_id: Option<String>` to `AuditEvent::Selection`.
2. Populate it wherever the selection event is first appended, from the same source
   `AuditUnit.prediction_id` already uses.
3. In `AuditUnit::from_selection`, read it from the event instead of hardcoding `None`.
4. Add a regression test: a unit whose queue file is deleted, rebuilt via `from_selection`
   alone, still carries the original `prediction_id`.

## Done when

- A unit rebuilt from its `audit.selection` record carries the same `prediction_id` it would
  have had from its queue file.
- The `[[verify]]` command passes.

## Notes

- 2026-10-04 (wave-14 follow-up, gap-45c8fe, gate 14b not yet merged): confirmed on
  `work/gap-3cd890` (not yet merged to main HEAD `b42c34ff9`), where `gap-45c8fe`'s own fix is
  implemented. This finding is a gap in that fix, not a duplicate of it — recorded as a
  separate item since `gap-45c8fe`'s own scope (per its text on `main`) is specifically about
  `write_label`'s `vs.label` rows, not the selection-event schema or the queue-file-missing
  fallback. Best addressed alongside `gap-45c8fe`'s merge, or as an immediate follow-up.
