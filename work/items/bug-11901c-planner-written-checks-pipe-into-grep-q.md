+++
id = "bug-11901c"
kind = "bug"
title = "Planner-written checks pipe into grep -q, which under the verifier's pipefail fails at random when the producer gets a broken pipe (exit 120), so passing work is retried until the cap"
status = "open"
triage = "verified"
severity = "p1"
goal = "core"
size = "M"
subsystem = ["roko-cli/plan_generate"]
created = 2026-10-13
updated = 2026-10-13
last_verified = 2026-10-13
source = "hermes-plugin-roko live end-to-end, 2026-10-13 (roko serve 1e3419a51)"
anchors = ["crates/roko-cli/src/graph_task_dispatch/verification.rs", "crates/roko-cli/src/plan_generate"]
links = { depends_on = [], blocks = [], related = ["bug-6fa025", "gap-1f08f4"], supersedes = [], duplicate_of = "" }
+++

## Problem

In the Hermes plugin's live end-to-end run (2026-10-13, roko serve built from `1e3419a51`, plan
`project-tally-a-small-python-cli-expense-ledger-2`, run `b7bca0f3…` started from the desktop UI with a $1.00 cap),
the planner-written check of T3 was a pipeline into `grep -q`:

```
python3 -m unittest tests.test_budgets -v 2>&1 | grep -q 'OK'
```

Graph verification runs every step as `bash -o pipefail -c <command>`. `grep -q` exits at its first match and
closes the pipe; when Python still has output to flush it dies writing to the closed pipe and exits 120, and with
`pipefail` that exit fails the step even though the text it looked for was there. Whether the check passes depends
on timing.

T3 retried five times on this check (each attempt told the agent its work failed a check), spent $0.89 of the run's
$1.00 cap, and stalled until the person cancelled the run at 39 minutes. The planner wrote the same pattern into the
dogfood plan earlier the same day (`… -v 2>&1 | grep -q 'OK'`, and `grep -q … && python3 …` chains).

## Why it matters

A check that fails at random wastes money on retries, blames the agent for passing work, and can sink a plan. The
planner writes this pattern by default, so most generated plans for Python projects carry it (goal `core`).

## Where

- `crates/roko-cli/src/graph_task_dispatch/verification.rs`: runs steps with `bash -o pipefail -c`.
- `crates/roko-cli/src/plan_generate/`: the planner's guidance for verify commands.
- `roko plan validate` (plan lint): could flag the pattern before a run.

## Current state

Nothing warns about a pipeline whose consumer exits early under `pipefail`; the planner is not told to avoid it.

## Plan

1. Tell the planner to write checks without early-exiting consumers in a pipeline: run the test command on its
   own (its exit status is the check), or capture output first (`out=$(…); grep -q … <<<"$out"`).
2. Make `roko plan validate` (and the spec gate) flag `| grep -q`, `| head` and similar after a producer, with the
   rewrite.
3. Optionally, when a step fails with exit 120 or 141 from a producer while the consumer succeeded, report it as a
   check defect rather than the agent's failure (see bug-6fa025 for exit 127).

## Done when

`[[verify]]` passes: plan lint warns on a `… | grep -q …` verify command, and the planner's prompt carries the rule.
