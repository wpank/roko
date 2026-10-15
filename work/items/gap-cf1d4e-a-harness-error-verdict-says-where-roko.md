+++
id = "gap-cf1d4e"
kind = "gap"
title = "A harness_error verdict says where roko failed, and the run keeps the scrubbed failure text locally, so the cause of a harness error can be read"
status = "open"
triage = "verified"
severity = "p2"
goal = "core"
size = "M"
subsystem = ["roko-cli/graph-task-dispatch", "roko-learn/telemetry"]
created = 2026-10-15
updated = 2026-10-15
last_verified = 2026-10-15
source = "status-quo-2026-10-15#finding-8"
anchors = ["crates/roko-cli/src/graph_task_dispatch/attempt.rs::failure_class", "crates/roko-cli/src/graph_task_dispatch/attempt.rs::harness_failure", "crates/roko-learn/src/telemetry/records.rs::AttemptFailureClass", "crates/roko-cli/src/commands/diagnose.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["gap-c013c5", "gap-f30b8e"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn harness_error_verdict_names_its_cause' crates/roko-cli/ && cargo test -p roko-cli harness_error_verdict_names_its_cause"
+++

## Problem

16 of the 44 attempts in the 10 real tally runs of 2026-10-14 (glm-5.1 on Z.ai, roko built from `86d629363` and
`e4e010844`) settled `harness_error`: 12 in run `4af3845a` and 4 in `9fc6970b`. Nobody can say why. The verdict keeps
only a digest of the failure text: `AttemptFailureClass::detail_sha256`, documented "The text itself is never stored"
(`crates/roko-learn/src/telemetry/records.rs:483-485`), filled by `failure_class` in
`crates/roko-cli/src/graph_task_dispatch/attempt.rs:1150-1169`. A harness failure's text is
`"harness: {error}"` (`AttemptResult::harness_failure`, `attempt.rs:900-908`), so the record says that roko failed,
not where. The tally workspace's `.roko` has since been removed, so these 16 can no longer be diagnosed.

## Why it matters

Goal `core`. 36% of real attempts failed in roko itself: the largest failure class in those runs (1 gate failure,
no provider error), and still the largest across the assessment's 49-run sweep (16 harness_error, 10 gate_failed,
6 provider_error in 97 verdicts). Learners correctly skip harness errors (blame `harness`, no learning label), so nothing else
records them either. `roko diagnose` and the dashboard can't show an operator what broke, and the next live run
(decision 5's $29 golden-path reruns, gap-c013c5, gap-f30b8e) would hit the same blind spot.

## Where

- `crates/roko-cli/src/graph_task_dispatch/attempt.rs::failure_class` (keeps only the digest) and
  `AttemptResult::harness_failure` (the reasons: prompt assembly, the cost ledger, `UNGUARDED_IN_CHECKOUT`).
- `crates/roko-learn/src/telemetry/records.rs::AttemptFailureClass`: the verdict's failure detail.
- `crates/roko-cli/src/commands/diagnose.rs`: what an operator reads.

## Current state

The digest exists so that a failure text, which can hold agent output or secrets, never lands in the verdict ledger.
That reason holds for gate and provider failures. A harness error's text is roko's own error chain, but it can still
name paths and, through a wrapped provider error, echo a response.

## Plan

1. Give harness errors a class: a small enum or string field on `AttemptFailureClass` (e.g. `harness_class`:
   `prompt_assembly`, `cost_ledger`, `unguarded_in_checkout`, `worktree`, `merge`, `other`) set where
   `harness_failure` is called, so the verdict says where roko failed without storing free text.
2. Keep the text locally: append the scrubbed failure text (`roko_agent::safety::scrub`, cut to a few hundred
   characters) to a per-run file next to `attempts.jsonl` (e.g. `.roko/runs/<run>/failures.jsonl`), keyed by
   `detail_sha256`. The verdict stays text-free and the operator's machine keeps the cause.
3. Have `roko diagnose <plan>` show the class and, when present, the local text for each harness-error attempt.
4. Test: `harness_error_verdict_names_its_cause` (a harness failure settles with its class on the verdict and its
   scrubbed text in the run's failure file, found by the verdict's digest).

## Done when

`harness_error_verdict_names_its_cause` passes (the `[[verify]]` command), and `roko diagnose` on a plan with a
harness-error attempt prints its class.

## Notes

Evidence: `tmp/status-quo/2026-10-15/programme/E2a-matrix-retag-AU-EX-IS-QA-RC.md:160-170` (live tally evidence) and
`backlog/A05-41-42-loops-and-prompts.md:137`. Don't put free failure text in the verdict ledger: it is the record
the papers' numbers and any shared bundle come from. `attempt.rs` is a hot file (one agent at a time).
