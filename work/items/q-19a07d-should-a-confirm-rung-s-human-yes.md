+++
id = "q-19a07d"
kind = "question"
title = "Should a confirm rung's human 'yes' get its own TaskGateVerdict variant, instead of collapsing into Passed?"
status = "open"
triage = "verified"
severity = "p2"
goal = "truth"
size = "S"
subsystem = ["roko-graph/cells", "roko-cli/graph-task-dispatch", "roko-learn/telemetry"]
created = 2026-10-03
updated = 2026-10-03
last_verified = 2026-10-03
source = "wave-8 follow-up reports 2026-10-03 (PK77 gap-a9c156)"
discovered_from = "gap-a9c156"
anchors = ["crates/roko-graph/src/cells/task_executor.rs::TaskGateVerdict", "crates/roko-learn/src/telemetry/records.rs::AttemptVerdictRecord", "crates/roko-cli/src/graph_task_dispatch/pack_rungs.rs::confirm_rung"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++

## Problem

A `confirm` rung's "yes" is indistinguishable, at the task's own verdict type, from an ordinary, fully
automated pass.

`crates/roko-cli/src/graph_task_dispatch/pack_rungs.rs:305`: `let confirmed = rung.kind == RungKind::Confirm &&
verdict.passed && !verdict.skipped;` — the per-step record does carry a separate signal,
`AttemptVerdictRecord::confirmed_by_user: bool` (`crates/roko-learn/src/telemetry/records.rs:521-523`), whose own
doc comment is explicit about the intent: "Whether the person the work is for confirmed the outcome... kept
apart from the machine checks, which routing and audits read **as such**" — i.e. the field exists specifically
so routing/audits *could* tell the two apart.

But `TaskGateVerdict` (`crates/roko-graph/src/cells/task_executor.rs:125-145`), the task-level verdict that
routing and audits actually read, has exactly five variants — `Passed`, `PassedWithPreexistingFailures`,
`AlreadySatisfied`, `Unverified`, `ForcedAccept` — none of which distinguishes "every check passed on its own"
from "a confirm rung's human 'yes' made it pass." A task whose pass came entirely from a person's confirmation
collapses to the same `TaskGateVerdict::Passed` as a task that never asked a person anything.

## Why it matters

Goal: cybernetic/truth. Every downstream consumer of `TaskGateVerdict::Passed` (the audit lottery's π at line
~7121's "green verdicts only," learning labels, routing's own "did this succeed" signal) currently treats a
human-confirmed outcome identically to a machine-verified one. If that's not intended, every one of those
consumers is implicitly over-trusting a judgement that was actually a person's, not a check's.

## Where

- `crates/roko-cli/src/graph_task_dispatch/pack_rungs.rs::confirm_rung` (builds the per-step `confirmed` signal).
- `crates/roko-learn/src/telemetry/records.rs::AttemptVerdictRecord::confirmed_by_user` (the per-step field that
  already carries the distinction, per its own doc comment's stated intent).
- `crates/roko-graph/src/cells/task_executor.rs::TaskGateVerdict` (the task-level type that doesn't).

## Current state

The per-step distinction exists and is recorded; it does not propagate into the task-level verdict type that
routing/audits actually consult.

## Why this needs Will, not just a fix

Whether this is worth a new `TaskGateVerdict` variant (e.g. `PassedWithUserConfirmation`, mirroring how
`PassedWithPreexistingFailures` and `AlreadySatisfied` already carve out "passed, but worth knowing why") is a
design choice with real downstream consequences: every exhaustive `match` on `TaskGateVerdict` across routing,
learning and audits would need a new arm, and the policy question — should a confirm-rung pass count toward the
same learning signals and audit sampling as an automated pass, or be treated differently — isn't a fact to verify
in code, it's a decision about what the self-model and audits should trust.

## Notes

- Discovered during PK77's work (gap-a9c156, done).
- If Will decides a new variant is warranted, that becomes a regular `gap`/`bug` item with its own anchors and
  verify command; this item is the decision question, not the implementation.
