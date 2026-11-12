+++
id = "bug-649557"
kind = "bug"
title = "roko diagnose counts a task's helper calls as attempts: a 2-try task reads 'failed after 4 attempts', two of them 'succeeded after 0.0 s, $0.00, helper-model', and the check's output prints once per try"
status = "done"
triage = "verified"
severity = "p2"
goal = "tooling"
size = "S"
subsystem = ["roko-cli/commands"]
created = 2026-11-12
updated = 2026-11-12
last_verified = 2026-11-12
last_verified_rev = "337914fb3"
source = "round v8 roko-recovery terminal captures (tmp/hermes-roko-v8/design/terminal/before, a0089c19c), the lead's approved proposal (tmp/hermes-roko-v8/design/terminal/README.md)"
discovered_from = "agent:roko-recovery (round v8, a0089c19c)"
anchors = ["crates/roko-cli/src/commands/diagnose.rs::task_attempts", "crates/roko-cli/src/commands/diagnose.rs::render_task"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn diagnose_counts_a_tasks_tries_without_helper_calls' crates/roko-cli/src && cargo test -p roko-cli --lib diagnose_counts_a_tasks_tries_without_helper_calls"

[closed]
at = 2026-11-12
at_ts = "2026-11-12T17:13:37Z"
commit = "337914fb3"
by = "roko-recovery"
executor = "claude-agent"
size = "S"
claimed_at = "2026-11-12T14:38:58Z"
forced = false
evidence = "337914fb3 on work/gap-5043c6: diagnose's task_attempts leaves out the cost rows of the calls a try makes beside its agent run (roles helper, audit, failover_refused), and the text counts a task's tries from the run's verdicts in attempts.jsonl (its provider calls for a run without them), printing the failed check's output once, from the last failed try. diagnose_counts_a_tasks_tries_without_helper_calls (2 tries read 2 in JSON and text, while total_agent_calls still counts all 4 calls) and diagnose_shows_each_try_and_the_check_output_once pass. On round v8's story workspace roko diagnose broken reads '2 tries', a line for try 1 and try 2 with each check error, and the traceback once (a0089c19c: 'failed after 4 attempts', two of them helper-model, and the traceback three times)."
+++

## Problem

`roko diagnose <plan>` lists a task's "attempts" from `.roko/learn/costs.jsonl`, every row with the task's id.
Those rows include the helper role's calls: role `helper`, $0.00, about 8 ms each. In round v8's capture
(`tmp/hermes-roko-v8/design/terminal/before/diagnose.*`), a task with 2 tries reads:

```
T1 [failed] Parse the settings file
    failed after 4 attempts; last error: …
    attempt 1: succeeded after 0.0 s, $0.00, helper-model
    attempt 2: failed after 4.6 s, $0.13, glm-4.7: verify: 1/2 verify step(s) failed …
    attempt 3: succeeded after 0.0 s, $0.00, helper-model
    attempt 4: failed after 5.4 s, $0.17, glm-4.7: …
```

The run's records (`.roko/runs/<run>/attempts.jsonl`) hold 2 tries, both `gate_failed`. The check's output is also
printed three times: in the last error, and in each failed attempt.

## Why it matters

Goal tooling. diagnose is how a person learns why a run failed. Phantom "succeeded" attempts make it say the
opposite of what happened.

## Where

`crates/roko-cli/src/commands/diagnose.rs::task_attempts`, which filters `records.attempts` (costs.jsonl rows) by
`task_id` only.

## Plan

Count a task's tries from the run's verdicts, or at least drop rows whose role isn't the task's agent role. Print the
failing check's output once, from the last try. Fixed with round v8's diagnose redesign.

## Done when

`diagnose_counts_a_tasks_tries_without_helper_calls` passes, and the `[[verify]]` passes.
