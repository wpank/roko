+++
id = "gap-e21595"
kind = "gap"
title = "Integration test C8: the ladder routes by tier and escalates after two failures"
status = "done"
triage = "verified"
severity = "p1"
goal = "golden-path"
size = "M"
subsystem = ["roko-cli/tests"]
created = 2026-09-29
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "1288aeb35"
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e5"
discovered_from = "tmp/cybernetic-harness/workstreams/assessment/W8-roko-as-executor.md (gate G8, canary C8)"
anchors = ["crates/roko-cli/tests/tier_ladder_canary.rs"]
lane = "rust-cold"
parent = "spec-98f76d"
links = { depends_on = ["gap-8c0a20", "gap-0f3980", "gap-9cbf35", "gap-dbf2a6", "gap-460230", "gap-b62e95"], blocks = [], related = ["gap-3aa9cb"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn tier_ladder_canary' crates/roko-cli/tests/ && cargo test -p roko-cli --test tier_ladder_canary"

[closed]
at = 2026-09-30
by = "coordinator (session 7622b882)"
evidence = "Merged in 1288aeb35. Canary C8 (crates/roko-cli/tests/tier_ladder_canary.rs) runs roko plan run through the built binary with a scripted provider behind three ladder rungs: tier start rungs, a rung hint, escalation after two failures, a pinned model that never moves, and each verdict's ladder field. It fails with escalation disabled by hand. Batch 19 gate on 1ce6526f8, re-assembled as 65ce6d506 with only a rustfmt commit on bug-ba8d42's test (b3ccce141, made on a separate branch; MAIN 1288aeb35 has the same code): cargo check --workspace --tests and clippy -p roko-agent -p roko-cli -p roko-core -p roko-fs -p roko-gate -p roko-learn -p roko-serve --keep-going -D warnings clean; nightly fmt clean; lib tests: roko-cli 3254 passed with 0 failed (the first full run with no load flakes, after bug-779ae7), roko-agent 2271, roko-core 1956, roko-fs 260, roko-gate 690, roko-serve 989, roko-learn 1204 (one pre-existing sub-millisecond timestamp test, append_preserves_first_seen_timestamp, passes alone); --test tier_ladder_canary 1 passed; --test plan_branch_integration 2 passed."
+++

## Problem

Each item of epic spec-98f76d has unit tests. Nothing checks, through `roko plan run`, that a task with no hint starts
on its tier's rung, that two failures move it up exactly one rung, that a pinned task never moves, and that the
records say which model ran and why. A later change to routing, retries or failover could quietly send every task to
one model again. On the portal runs, all 210 attempts ran one pinned model (B7).

## Why it matters

This is the exit check for epic spec-98f76d and canary C8 for W8's gate G8. It joins the golden-path suite (E11.1,
gap-3aa9cb). The whitepaper's claim that cheap models execute and escalation recovers cites it.

## Where

- **New file:** `crates/roko-cli/tests/tier_ladder_canary.rs`.
- **Pattern to copy:** `crates/roko-cli/tests/graph_budget_resume.rs`: a scripted fake `claude_cli` provider set up in
  `roko.toml`, and the built binary run with `assert_cmd`. The adapter passes `--model <slug>`
  (`claude_cli_agent.rs:381`), so the script can tell which rung it serves.
- **Surfaces to assert:** the script's call log, the Graph checkpoint, gap-96f7ed's attempt records and
  `.roko/learn/cascade-router.json`.

## Current state

No such test exists at `41c7ffbd6`.

## Plan

1. In a temp workspace, put three models on one fake provider script, with a `[routing.ladder]` over them. The script
   logs the model and task id of every call.
2. Fixture plan, `max_parallel = 1`, every task `mechanical`:
   - T1: no hint, verify passes;
   - T2: no hint, `max_retries = 3`; its verify passes only once the rung-1 model has run (the script writes the checked
     file only for that model);
   - T3: `model_hint` pinned to the rung-0 model, verify always fails, `max_retries = 1`.
3. Assert:
   - T1 ran once, on rung 0; T2 ran on rung 0, rung 0, then rung 1, and passed;
   - T3 ran twice on its pinned model and failed, and the plan fails because of T3 only;
   - each attempt records its rung, model and reason;
   - the router state holds observations for at least two models;
   - once attempt records carry `cost.source` (S01.P0-6, E4), it is set on every attempt.

## Done when

- [ ] The test exists and passes in under a minute, with the fake provider only.
- [ ] Reverting gap-460230's escalation makes it fail. Check this once by hand and say so in the closing evidence.
- [ ] The `[[verify]]` command passes.

## Notes

The test edits no hot file. It can be written while the E5 items are in progress, and it merges last.
- 2026-09-30 (wk-tiers): implemented on `work/gap-e21595` at `245927910`. The canary passes in the worktree in about 13 s of test time, with the fake provider only. Disabling gap-460230's escalation by hand (`climb(&inputs.role, start, 0)` in `ladder_choice`) makes it fail: T2 ran four times on the cheap rung. Beyond the plan, T4 checks that a `rung` hint picks the start rung, T5 (integrative) that the tier picks it, and T3 retries twice, so a pin that moved would show on its third attempt. Workspace verification is deferred to the batch check.
