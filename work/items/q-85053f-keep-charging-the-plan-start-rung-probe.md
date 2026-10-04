+++
id = "q-85053f"
kind = "question"
title = "Keep charging the plan-start rung probe to attempt 1, or give it its own plan-overhead cost row?"
status = "open"
triage = "verified"
severity = "p3"
goal = "proof"
size = "M"
subsystem = ["roko-cli/dispatch", "benchmarks/viabilitybench/driver"]
created = 2026-10-04
updated = 2026-10-04
last_verified = 2026-10-04
source = "wave-8 follow-up reports, filed 2026-10-04 (bug-0b7695)"
discovered_from = "bug-0b7695"
anchors = ["benchmarks/viabilitybench/driver/run_roko.py"]
lane = "bench"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++

## Problem

The plan-start rung probe (`crates/roko-cli/src/dispatch/rung_probe.rs::probe_ladder`, backlog task 1121) makes
one call per rung model before any task attempt starts. The bench driver's metering
(`benchmarks/viabilitybench/driver/run_roko.py::_meter_from_proxy`, whose own doc comment names "backlog 1121;
bug-0b7695") does **not** skip these calls: it explicitly buckets every proxy request that timestamps before the
first attempt's start (`S01`'s `timing.attempt_started_at`) into `attempts[0].plan_start_calls` and
`attempts[0].plan_start_usage`, separate fields from the attempt's own request/usage counts, each request priced
by its own model at `settle` time. This was the worker's deliberate choice when bug-0b7695 was fixed, not an
oversight — the question is whether it's the right one going forward.

## Why it matters

Goal: proof/truth (honest per-attempt cost accounting). Attempt 1's record carries the probe's cost as a
distinct field today, not silently merged into its own usage — so anything reading `plan_start_usage`
specifically can already separate them. But anything that reads an attempt's *cost* generically (without knowing
to special-case `plan_start_usage`) would still see attempt 1 as more expensive than its peers for reasons that
have nothing to do with the task it ran — the ladder's one-time setup cost, attributed to whichever attempt
happened to run first.

## Where

- `crates/roko-cli/src/dispatch/rung_probe.rs::probe_ladder` (where the probe calls happen).
- `benchmarks/viabilitybench/driver/run_roko.py::_meter_from_proxy` (where they're currently bucketed onto
  attempt 1, in `plan_start_calls`/`plan_start_usage`).

## Why this needs Will

Two options:

1. **Keep charging probes to attempt 1**, as today — simpler (no new cost-row kind needed), and the fields are
   already named distinctly so a careful reader *can* separate them; the risk is that most readers of per-attempt
   cost aren't careful about this and get a skewed number for attempt 1 specifically.
2. **Record probes as plan overhead in a cost row of their own** (not attributed to any attempt) — cleaner for
   every downstream reader of per-attempt cost, including the bench driver's own metering, but needs a new
   cost-row shape (or a `plan_id`-only row with no `task_id`/`attempt`) threaded through both the Rust side and
   `run_roko.py`'s parsing.

## Notes

- Discovered during the work that closed bug-0b7695.
- If Will decides on option 2, that becomes a regular `gap`/`bug` item; the current mechanism (option 1, already
  shipped) is sound as far as it goes, so this is about whether to change it, not a correctness defect.
