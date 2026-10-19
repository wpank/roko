+++
id = "bug-718016"
kind = "bug"
title = "plan validate --dynamic runs red-on-base steps without [gates] env_passthrough and strip_env (only plan run begins a check run with the config's settings), so a project whose checks need a passed-through variable gets different answers from validate than from the plan-load gate"
status = "done"
triage = "verified"
severity = "p2"
goal = "core"
size = "S"
subsystem = ["roko-cli/plan_validate"]
created = 2026-10-19
updated = 2026-10-19
last_verified = 2026-10-19
last_verified_rev = "7aa57d688"
source = "round-v7-roko-speed#gap-e02ec4"
discovered_from = "agent:roko-speed (round v7, measuring gap-e02ec4, 2026-10-19)"
anchors = ["crates/roko-cli/src/commands/plan.rs::cmd_plan_validate"]
links = { depends_on = [], blocks = [], related = ["gap-e02ec4"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn validate_dynamic_steps_get_the_gates_env_passthrough' crates/roko-cli/src && cargo test -p roko-cli --bin roko -- validate_dynamic_steps_get_the_gates_env_passthrough"

[closed]
at = 2026-10-19
at_ts = "2026-10-19T19:00:41Z"
commit = "7aa57d688"
forced = false
evidence = "plan validate --dynamic now begins a check run with the workspace config's check_env_settings, as plan run does. Verify validate_dynamic_steps_get_the_gates_env_passthrough passes: in a child process, a check that needs a passed-through variable passes on the base with the config and does not without it. On the JUCE fixture, validate's plugin steps now get UNDERTOW_JUCE_SOURCE_DIR, and validate took 20.3 s; before the fix, each plugin step hung on a blocked JUCE download."
+++

## Problem

`roko plan validate --spec-quality --dynamic` runs each task's red-on-base steps without the workspace's
`[gates] env_passthrough` and `strip_env`. Only `plan run` begins a check run with the config's settings
(`child_env::begin_check_run(config.gates.check_env_settings())` in `graph_execution/plan_runner.rs`).
Validate's steps get the default check environment, so a variable the project's checks need is dropped.

Seen on round v7's JUCE fixture: the `plugins` preset reads `UNDERTOW_JUCE_SOURCE_DIR`, which `roko.toml` passes
through. Under `plan validate --dynamic` the variable is left out ("2 not on the allowlist", against 1 under
`plan run`). CMake then falls back to FetchContent's download, which the round's fenced proxy blocks, and every plugin
step hangs until its timeout.

Undertow passes `UNDERTOW_JUCE_PREFIX` and `UNDERTOW_DEPS_DIR` the same way, so its validate gets results the plan-load
gate would not. A check that passes on the base under the run's environment can read as red under validate's, which
hides HF3.

## Why it matters

`validate --dynamic` is the person's check of a plan before they run it. Its answers should be the plan-load gate's.
gap-e02ec4's cache also shares results between the two, so they must run steps in the same environment.

## Where

- `crates/roko-cli/src/commands/plan.rs`: `cmd_plan_validate`, around its `spec_red_on_base::check_plans` call.

## Plan

Begin a check run with the workspace config's `check_env_settings()` around validate's red-on-base check, as `plan
run` does.

## Done when

Validate's steps see a variable `[gates] env_passthrough` names. Checked by the test in `[[verify]]`.

## Notes

- Found by roko-speed while measuring gap-e02ec4, and fixed on the same branch. gap-e02ec4's cache key also covers the
  environment the steps get, so a result made under one environment is never reused under another.
