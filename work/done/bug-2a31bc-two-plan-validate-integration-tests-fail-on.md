+++
id = "bug-2a31bc"
kind = "bug"
title = "Two plan validate integration tests fail on main: their diagnostic counts predate PLAN_041"
status = "done"
triage = "verified"
severity = "p2"
size = "S"
subsystem = ["roko-cli/plan_validate"]
created = 2026-10-02
updated = 2026-10-26
last_verified = 2026-10-26
last_verified_rev = "da7df0194"
source = "roko-7d: cargo nextest run --workspace for the workflow-audit merge (2026-10-02)"
discovered_from = "merge:bfd36512f"
anchors = ["crates/roko-cli/tests/plan_validate.rs", "crates/roko-cli/src/plan_validate.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn plan_validate_warns_on_known_model_aliases' crates/roko-cli/tests/ && cargo test -p roko-cli --test plan_validate plan_validate_"

[closed]
at = 2026-10-26
at_ts = "2026-10-26T11:25:25Z"
commit = "da7df0194"
executor = "claude-agent"
via = "manual"
size = "S"
claimed_at = "2026-10-26T10:26:17Z"
forced = false
evidence = "da7df0194 (on main 15335e2f9): crates/roko-cli/tests/plan_validate.rs now expects PLAN_041 beside PLAN_012 and PLAN_009. plan_validate_warns_on_known_model_aliases asserts 6 diagnostics: three PLAN_012 alias lines and three PLAN_041 lines, one per task, each naming its model_hint. plan_validate_preserves_unknown_model_warning asserts 2: PLAN_009 and PLAN_041. PLAN_041 is correct there, not a validator bug. Dispatch pins any non-empty model_hint ahead of the routing ladder (on by default), whether it is an alias or an unconfigured name: dispatch/model_routing.rs ModelRouter::choose returns it as TaskHint before ladder_choice, and graph_task_dispatch/ladder.rs records the attempt as LadderReason::Pinned. The fixes for PLAN_012 (the full name) and PLAN_009 (configure the model) both keep the pin, and only `rung` clears PLAN_041. The same fixture with rung = \"strong\" in place of the pin validates with 0 diagnostics. The built roko printed exactly 6 and 2 diagnostics on the two test fixtures. The verify passes on the tree committed as da7df0194: cargo test -p roko-cli --test plan_validate plan_validate_ ran 20 passed, 0 failed, once in the session environment and once in env -i with a scratch HOME, CI=true and no keys. cargo +nightly fmt --all --check is clean, and so is cargo clippy -p roko-cli --no-deps -- -D warnings. Clippy over the plan_validate test target flags none of the changed lines; its 52 errors and 3 warnings are pre-existing (unwrap_used in the tests' setup, and tests/common/mod.rs)."
+++

## Problem

Two `plan validate` integration tests fail on main (checked on a build of main at f49242f63):

- `plan_validate_warns_on_known_model_aliases` expects "3 diagnostics in 1 plan"; the run prints 6: the three
  PLAN_012 alias warnings plus a PLAN_041 warning for each task ("sets model_hint = 'haiku', which pins a model
  and bypasses the routing ladder; use `rung`").
- `plan_validate_preserves_unknown_model_warning` expects "1 diagnostics in 1 plan"; the run prints 2: PLAN_009
  plus PLAN_041.

PLAN_041 arrived with 5b8efa148 (2026-09-30, "routing: parse TaskDef hints and let plans pin a ladder rung");
the tests were not updated.

## Why it matters

A red `plan_validate` suite hides real regressions in plan validation. The batch gates run lib tests only, so
nothing flags it.

## Where

- `crates/roko-cli/tests/plan_validate.rs` (the two tests)
- `crates/roko-cli/src/plan_validate.rs` (PLAN_041)

## Plan

Decide whether PLAN_041 should fire on a task that already gets PLAN_009 or PLAN_012 for the same `model_hint`.
If yes, update the expected counts to 6 and 2 and assert that PLAN_041 is present. If not, suppress it in that
case and keep the counts.

## Done when

- Both tests pass. Verify:
  `grep -rqw 'fn plan_validate_warns_on_known_model_aliases' crates/roko-cli/tests/ && cargo test -p roko-cli --test plan_validate plan_validate_`

## Notes

- Found by roko-7d's full `cargo nextest run --workspace` for the workflow-audit merge (2026-10-02).
- 2026-10-02 (roko-90): both tests already failed at backlog gate 1 on a43288b5f, where main's prebuilt roko
  printed the same 6 and 2 diagnostics, so they predate this week's waves.
