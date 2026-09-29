+++
id = "gap-e21595"
kind = "gap"
title = "Integration test C8: the ladder routes by tier and escalates after two failures"
status = "open"
triage = "unverified"
severity = "p1"
goal = "golden-path"
size = "M"
subsystem = ["roko-cli/tests"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e5"
discovered_from = "tmp/cybernetic-harness/workstreams/assessment/W8-roko-as-executor.md (gate G8, canary C8)"
anchors = ["crates/roko-cli/tests/tier_ladder_canary.rs"]
lane = "rust-cold"
parent = "spec-98f76d"
links = { depends_on = ["gap-8c0a20", "gap-0f3980", "gap-9cbf35", "gap-dbf2a6", "gap-460230", "gap-b62e95"], blocks = [], related = ["gap-3aa9cb"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn tier_ladder_canary' crates/roko-cli/tests/ && cargo test -p roko-cli --test tier_ladder_canary"
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
