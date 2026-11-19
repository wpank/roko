+++
id = "reg-1bf684"
kind = "regression"
title = "Two roko-cli tests fail on main since find-697944 put audit_off and holdout_off in a run manifest's ablation_flags: frozen_learning_switch_reaches_config_and_manifest and frozen_learning_run_writes_no_learned_state still assert the exact list, and their workspaces turn audits off"
status = "done"
triage = "verified"
severity = "p1"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/graph_execution"]
created = 2026-10-26
updated = 2026-10-26
last_verified = 2026-10-26
last_verified_rev = "37aef669b"
source = "round-v7 roko-speed chain47 on main 7675b37f7, the lead's ask"
discovered_from = "agent:roko-speed (round v7)"
anchors = ["crates/roko-cli/src/graph_execution/plan_runner.rs::frozen_learning_switch_reaches_config_and_manifest", "crates/roko-cli/src/graph_execution/plan_runner.rs::frozen_learning_run_writes_no_learned_state", "crates/roko-cli/src/graph_execution/run_manifest.rs::capture"]
links = { depends_on = [], blocks = [], related = ["find-697944"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-cli --lib -- frozen_learning_switch_reaches_config_and_manifest frozen_learning_run_writes_no_learned_state"

[closed]
at = 2026-10-26
at_ts = "2026-10-26T19:48:43Z"
commit = "37aef669b"
executor = "claude-agent"
via = "manual"
size = "S"
claimed_at = "2026-10-26T19:25:25Z"
forced = false
evidence = "chain48 on main 1cf3cf3fe with the fix (37aef669b): frozen_learning_switch_reaches_config_and_manifest and frozen_learning_run_writes_no_learned_state pass, asserting learning_frozen is in the frozen run's ablation flags (config and manifest) and not in the live run's; every lib test of the six packages passes: roko-cli 4039 passed, 0 failed (8 ignored, share_run_scrubs_output_text_and_prompt skipped), roko-agent 2415, roko-core 2049, roko-learn 1405, roko-runtime 213, roko-serve 1086"
+++

## Problem

find-697944 (9d68ed77f, merged 754f45ebd) made a run's manifest record `audit_off` and `holdout_off` in
`experiment.ablation_flags`, beside `learning_frozen`, whenever the run's config turns audits off or holds nothing out.
It did not update two roko-cli tests that assert the exact list. Their workspaces turn audits off (`[audit] enabled =
false`, the plan-runner fixture's fence), and one also runs in maximize mode. So both fail on main:

- `graph_execution::plan_runner::tests::frozen_learning_switch_reaches_config_and_manifest`: the live run's flags
  are `["audit_off"]` where the test wants none, and the frozen run's are `learning_frozen` plus `audit_off`.
- `graph_execution::plan_runner::tests::frozen_learning_run_writes_no_learned_state`: the flags are
  `["learning_frozen", "audit_off", "holdout_off"]` where the test wants `["learning_frozen"]`.

Every roko-cli lib suite run is red until they are fixed. roko-speed's chain47 on main 7675b37f7 found them: 4029
passed, 2 failed.

## Why it matters

Truth and the golden path. A red suite on main hides the next real failure, and every agent's merge chain fails on it.

## Where

`crates/roko-cli/src/graph_execution/plan_runner.rs`, both tests. `run_manifest.rs::RunManifests::capture` sets the
flags.

## Plan

The tests check what they are about. `learning_frozen` is in the frozen run's flags and not in the live run's. The
other flags depend on the workspace's audit and holdout settings, which these tests do not exercise.

## Done when

- Both tests pass on main, and the `[[verify]]` passes.
