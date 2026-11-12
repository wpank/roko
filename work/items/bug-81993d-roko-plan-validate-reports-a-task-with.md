+++
id = "bug-81993d"
kind = "bug"
title = "roko plan validate reports a task with no verify step twice: PLAN_035's schema error and PLAN_037"
status = "open"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["roko-cli/plan_validate"]
created = 2026-11-12
updated = 2026-11-12
last_verified = 2026-11-12
source = "round v8 roko-recovery terminal captures (tmp/hermes-roko-v8/design/terminal/before, a0089c19c), the lead's approved proposal (tmp/hermes-roko-v8/design/terminal/README.md)"
discovered_from = "agent:roko-recovery (round v8, a0089c19c)"
anchors = ["crates/roko-cli/src/plan_validate.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_task_without_a_check_is_reported_once' crates/roko-cli/src && cargo test -p roko-cli --lib a_task_without_a_check_is_reported_once"
+++

## Problem

`roko plan validate` reports a task that has no verify step twice:

```
  error PLAN_035 schema validation failed: T2: missing 'verify' (required for role 'implementer')
  error PLAN_037 task 'T2' has no verify steps: it can only end unverified, …
2 diagnostics in 1 plan
```

Capture: `tmp/hermes-roko-v8/design/terminal/before/validate-draft.*`.

## Why it matters

Goal tooling. One problem reads as two, and the count is wrong.

## Where

`crates/roko-cli/src/plan_validate.rs`: the schema rule PLAN_035 and the no-verify rule PLAN_037 each fire for the
same task.

## Plan

When PLAN_037 fires for a task, drop PLAN_035's "missing 'verify'" for the same task. PLAN_037 says more: why it
matters and how to fix it. Fixed with round v8's validate redesign. The JSON report loses the duplicate too.

## Done when

`a_task_without_a_check_is_reported_once` passes, and the `[[verify]]` passes.
