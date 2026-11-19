+++
id = "gap-f465ce"
kind = "gap"
title = "The roko.verdict/1 record has no field saying a try passed after reaching its rung's bound; only attempt_settled.detail says so"
status = "done"
triage = "verified"
severity = "p3"
size = "S"
subsystem = ["roko-learn/telemetry"]
created = 2026-10-17
updated = 2026-10-20
last_verified = 2026-10-20
last_verified_rev = "5f285c475"
source = "team-lead, round v6 (roko-caps' gap-2c9412 follow-up)"
discovered_from = "gap-2c9412"
anchors = ["crates/roko-learn/src/telemetry/records.rs::AttemptVerdictRecord", "crates/roko-cli/src/graph_task_dispatch/attempt.rs::SettledAttempt"]
links = { depends_on = [], blocks = [], related = ["gap-2c9412", "gap-3217a0"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'gap-f465ce' crates/roko-cli/src/graph_task_dispatch/ladder.rs && cargo test -p roko-cli --lib -- a_bounded_try_is_judged_by_its_checks a_try_that_runs_its_time_is_judged_by_its_checks"

[closed]
at = 2026-10-20
at_ts = "2026-10-20T21:05:22Z"
commit = "5f285c475"
by = "roko-speed"
executor = "claude-agent"
via = "manual"
size = "S"
claimed_at = "2026-10-20T20:27:43Z"
forced = false
evidence = "5f285c475. gap-3217a0 (a1d12fc1c) added the field: roko.verdict/1's `bound` (\"turns\", \"spend\" or \"time\", records.rs BOUND_*), set in AttemptContext::settle from the try's ended_at_bound, which graph_task_dispatch.rs and streaming.rs call when a try stops at its rung's bound. This item adds the test: the ladder's bounded_tries helper now reads each verdict's `bound` from the run's attempts.jsonl, and a_bounded_try_is_judged_by_its_checks asserts [\"turns\"] for a pass after the 30-turn bound and [\"turns\", \"turns\", \"-\"] for two bounded failures then the unbounded top rung's pass; a_try_that_runs_its_time_is_judged_by_its_checks asserts [\"time\"] for a pass after the 2 s time bound. The literal verify passes on the branch (graph_task_dispatch::ladder::tests: 21 passed); nightly fmt and clippy -D warnings on the four packages are clean."
+++

## Problem

Since gap-2c9412, the task's checks judge a try that ends at its rung's bound. When they pass, only the hub's
`attempt_settled.detail` records that the try passed after reaching its bound ("passed its checks after reaching
its 30-turn bound"). The `roko.verdict/1` record in `attempts.jsonl` has no field for it, so the run's own records
cannot tell such a pass from a try that finished on its own.

## Why it matters

Learning and audits read the verdict records, not hub events. A pass at the bound used all of its turns or spend,
which is worth knowing when routing or setting rung bounds.

## Where

- `crates/roko-learn/src/telemetry/records.rs::AttemptVerdictRecord`
- `crates/roko-cli/src/graph_task_dispatch/attempt.rs::SettledAttempt` (`bound`)

## Plan

Add an optional field to the verdict record, for example `ended_at_bound = "turns"` or `"spend"`, set from
`SettledAttempt.bound`.

## Done when

A try that passes after reaching its bound writes the field in `attempts.jsonl`, and a test covers it.
