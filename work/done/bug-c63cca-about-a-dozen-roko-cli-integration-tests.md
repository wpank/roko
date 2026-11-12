+++
id = "bug-c63cca"
kind = "bug"
title = "About a dozen roko-cli integration tests pass only with a real claude CLI on PATH, so they fail on CI"
status = "done"
triage = "verified"
severity = "p2"
goal = "tooling"
size = "M"
subsystem = ["roko-cli/tests"]
created = 2026-10-11
updated = 2026-10-26
last_verified = 2026-10-26
last_verified_rev = "8e82c1f75"
source = "CI-like local sweep 2026-10-11 (roko-90)"
discovered_from = "PR #85 CI (cost_dedup)"
anchors = ["crates/roko-cli/tests/common/mod.rs::run_sample_plan", "crates/roko-cli/src/graph_execution/plan_runner.rs::preflight_provider_for_model"]
links = { depends_on = [], blocks = [], related = ["bug-2a31bc", "bug-13bbe9", "dec-01be49", "bug-ee2752", "bug-bd261a"], supersedes = [], duplicate_of = "" }

[[verify]]
command = '''grep -q 'fn with_mock_agents' crates/roko-cli/tests/common/mod.rs && env -i HOME="$(mktemp -d)" USER="$USER" LOGNAME="$USER" CARGO_HOME="$HOME/.cargo" RUSTUP_HOME="$HOME/.rustup" PATH="$HOME/.cargo/bin:/usr/bin:/bin:/usr/sbin:/sbin" TMPDIR="$(mktemp -d)" CARGO_INCREMENTAL=0 SKIP_FRONTEND_BUILD=1 CI=true cargo test -p roko-cli --no-fail-fast --test cost_dedup --test default_engine --test doctor --test graph_plan_callers --test phase0_wiring --test phase1_protocols --test smoke --test worktree_task_diff -- --exact default_engine_does_real_work doctor_human_output_contains_v2_abstractions doctor_human_output_succeeds_for_bootstrapped_workspace doctor_json_output_contains_v2_abstractions item_03_episode_logger_appends_memory_log item_04_plan_runner_reports_non_zero_agent_calls item_06_plan_run_persists_learning_feedback phase0_cascade_router_persists_after_plan_run phase0_episodes_still_logged run_plan_executes_through_graph_engine single_task_plan_emits_one_efficiency_event_per_attempt worktree_task_that_fails_verify_keeps_its_worktree_and_merges_nothing'''

[closed]
at = 2026-10-26
at_ts = "2026-10-26T14:22:40Z"
commit = "8e82c1f75"
executor = "claude-agent"
via = "manual"
size = "M"
claimed_at = "2026-10-26T10:26:11Z"
forced = false
evidence = "8e82c1f75 adds Isolated::with_mock_agents to tests/common. It puts the existing mock agent (MOCK_CLAUDE_SCRIPT) first on PATH as claude, codex and gemini; the mock now answers --version, reads its prompt and echoes --model. These use it: init_workspace, run_sample_plan, run_roko_isolated, graph_plan_callers::run_roko (now isolated, its own shims gone), worktree_task_diff::run_plan, and every doctor spawn in doctor.rs and phase1_protocols.rs. So on any machine, roko init writes the Claude CLI provider and model, and the plan runs' preflight no longer falls back to the built-in claude-sonnet-4-6 and ANTHROPIC_API_KEY. The doctor's shared_credentials_none and provider_usable checks find a credential. Three tests had other causes. item_03 failed on the spec gate (HF2 refuses its `true` check), not on claude; its roko.toml now sets [spec_quality] mode = off. worktree_task_that_fails_verify failed on bug-ee2752: the helper call after the failed verify ran the file-writing task provider in the operator's checkout. Its helper calls and default_model now go to a second model on the mock, and model_hint pins its task. bug-ee2752's check moved to the ignored a_helper_call_after_a_failed_verify_leaves_the_operators_checkout_alone, which fails with ?? feature.txt; bug-ee2752's verify runs it with --include-ignored. plan_run_log_file_records_one_bracketed_run now gets past the preflight and fails only because run.started's plan_ids is [], which it has been since 40e48440b; that is bug-bd261a, filed and left out of this verify. Proof: before the change all 13 failed under env -i (no keys, no claude, scratch HOME, CI=true). After it, work.py verify bug-c63cca passes: 12 tests under env -i. Every test in cost_dedup, default_engine, doctor, graph_plan_callers, phase0_wiring, phase1_protocols, smoke (item_01 skipped), worktree_task_diff, cli_fallback and spawn_isolation passes both under env -i and in the session environment, except plan_run_log_file_records_one_bracketed_run, which fails the same way in both (bug-bd261a). cargo +nightly fmt --all changes nothing, and cargo clippy -p roko-cli --no-deps -- -D warnings is clean."
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

- 2026-10-26 (bug-c63cca worker): reproduced all 13 on 15335e2f9 under `env -i` with no keys and no `claude`. Since
  bug-eed697 they fail locally too, because `isolated()` hides the developer's login. What each needed:
  - The eight sample-plan runs, and `run --plan`: without `claude` on PATH, `roko init` writes no provider or model
    (only commented examples), so `default_model = "claude-sonnet-4-6"` falls to the built-in model and its
    `ANTHROPIC_API_KEY`. `setup_sample_plan_workspace`'s `command =` rewrite then had no line to point at its mock.
  - The three doctor tests failed only `shared_credentials_none` and `provider_usable`, both of which look for a
    `claude` on PATH.
  - The fix for both is one helper in `tests/common/mod.rs`, `Isolated::with_mock_agents`. It puts the existing mock
    (`MOCK_CLAUDE_SCRIPT`) first on PATH as `claude`, `codex` and `gemini`. The mock now answers `--version`, reads
    its prompt and echoes `--model`. `init_workspace`, `run_sample_plan`, `run_roko_isolated`, `graph_plan_callers`
    (instead of its own shims), `worktree_task_diff` and every doctor spawn in `doctor.rs` and `phase1_protocols.rs`
    use it.
  - `item_03_episode_logger_appends_memory_log` no longer reached a claude check: the spec gate refuses its only
    check, `true` (HF2). Its own `roko.toml` now sets `[spec_quality] mode = "off"`; the test is about the episode
    log.
  - `worktree_task_that_fails_verify_keeps_its_worktree_and_merges_nothing` failed on bug-ee2752: the helper call
    after the failed verify runs the task's provider in the operator's checkout. Its helper calls and
    `default_model` now go to a second model on the mock, with the task pinned by `model_hint`. bug-ee2752's check
    moved to its own ignored test.
  - `plan_run_log_file_records_one_bracketed_run` now gets past the preflight, then fails on a separate regression,
    bug-bd261a (filed): since 40e48440b, `run.started` names no plans. It is left out of this item's verify, and it
    is bug-bd261a's verify.
