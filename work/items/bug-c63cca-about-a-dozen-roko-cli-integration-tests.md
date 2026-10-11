+++
id = "bug-c63cca"
kind = "bug"
title = "About a dozen roko-cli integration tests pass only with a real claude CLI on PATH, so they fail on CI"
status = "open"
triage = "verified"
severity = "p2"
goal = "tooling"
size = "M"
subsystem = ["roko-cli/tests"]
created = 2026-10-11
updated = 2026-10-11
last_verified = 2026-10-11
source = "CI-like local sweep 2026-10-11 (roko-90)"
discovered_from = "PR #85 CI (cost_dedup)"
anchors = ["crates/roko-cli/tests/common/mod.rs::run_sample_plan", "crates/roko-cli/src/graph_execution/plan_runner.rs::preflight_provider_for_model"]
links = { depends_on = [], blocks = [], related = ["bug-2a31bc", "bug-13bbe9", "dec-01be49"], supersedes = [], duplicate_of = "" }
+++

## Problem

About a dozen roko-cli integration tests pass only on a machine with a real `claude` CLI on PATH. CI has none, so
they fail there. They were hidden behind earlier failures in CI's `cargo test --workspace`, which stops at the first
failing binary; a CI-like local sweep on 2026-10-11 (no API keys, empty HOME, no `claude` or `gh` on PATH,
`--no-fail-fast`) found them all:

- Plan runs on the sample workspace, which fail at the model preflight with "model 'claude-sonnet-4-6' requires
  ANTHROPIC_API_KEY but it is not set": `cost_dedup::single_task_plan_emits_one_efficiency_event_per_attempt`,
  `phase0_wiring::{phase0_episodes_still_logged, phase0_cascade_router_persists_after_plan_run}`,
  `smoke::{item_04_plan_runner_reports_non_zero_agent_calls, item_06_plan_run_persists_learning_feedback}`,
  `default_engine::default_engine_does_real_work`,
  `graph_plan_callers::{run_plan_executes_through_graph_engine, plan_run_log_file_records_one_bracketed_run}`,
  `worktree_task_diff::worktree_task_that_fails_verify_keeps_its_worktree_and_merges_nothing`.
- `roko doctor` through the binary, which exits 1 because no provider has credentials:
  `doctor::doctor_human_output_succeeds_for_bootstrapped_workspace`,
  `phase1_protocols::{doctor_human_output_contains_v2_abstractions, doctor_json_output_contains_v2_abstractions}`,
  `smoke::item_03_episode_logger_appends_memory_log`.

## Why it matters

CI's test step cannot go green while these depend on the developer's machine, and every crate after them goes
untested in CI. dec-01be49 fixed the same problem for the doctor lib tests, using injectable probes.

## Where

The harnesses that spawn the `roko` binary: `crates/roko-cli/tests/common/mod.rs` (`run_sample_plan`,
`setup_sample_plan_workspace`), plus the test files above that build their own `Command`.

## Current state

The harnesses already isolate HOME and drop `ANTHROPIC_API_KEY`. Locally, though, roko still finds the developer's real
`claude` on PATH and routes the default model through the claude CLI provider, so the API-key preflight
(`graph_execution/plan_runner.rs::preflight_provider_for_model`) never fires. Without `claude` on PATH, it fails.

## Plan

Make the harnesses hermetic. Prepend a per-test bin directory to PATH that holds a fake `claude`: the existing mock
script, which must also answer `claude --version` the way `auth_detect::version_probe` expects. Never let a test see
the real one. Put the PATH setup in one helper in `tests/common/mod.rs` and use it in every harness above.

## Done when

- [ ] The listed tests pass with no `claude` and no keys: run them under `env -i` with a minimal PATH, as in the
  sweep.
- [ ] CI's Test + Clippy gets past roko-cli's integration tests.

## Notes

The same sweep also found these:
- the two `plan_validate` tests (already bug-2a31bc);
- the stale prompt snapshot (filed separately);
- the coverage wrapper failure (bug-13bbe9).

`roko-gate`'s `shell::tests::timeout_kills_shell_descendants` failed once under the sweep's full parallel load and
passed alone.

The sweep script: run `cargo test --workspace --no-fail-fast -- --skip share_run_scrubs_output_text_and_prompt` under
`env -i` with HOME set to a temp dir, PATH set to cargo, node and python3 shims plus `/usr/bin:/bin`, and `CI=true`.
Don't share a `CARGO_TARGET_DIR` with nested fixture builds (see the shared-target item).
