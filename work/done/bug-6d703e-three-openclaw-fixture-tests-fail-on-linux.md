+++
id = "bug-6d703e"
kind = "bug"
title = "Three openclaw fixture tests fail on Linux: they execute their fake binary while it is still open for writing"
status = "done"
triage = "verified"
severity = "p2"
goal = "tooling"
size = "S"
subsystem = ["roko-agent/openclaw"]
created = 2026-10-08
updated = 2026-10-08
last_verified = 2026-10-08
last_verified_rev = "5eba50188"
source = "PR #84 CI run 37798627818 (Test + Clippy)"
discovered_from = "bug-68a33f (the first CI run past roko-acp)"
anchors = ["crates/roko-agent/src/openclaw/infer_agent.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'script.into_temp_path()' crates/roko-agent/src/openclaw/infer_agent.rs && cargo test -p roko-agent --lib openclaw::infer_agent::fixture_tests"

[closed]
at = 2026-10-08
at_ts = "2026-10-08T17:06:57Z"
commit = "48b2bf137"
forced = false
evidence = "PR #84 CI run 37806395795 (ubuntu-latest, Test + Clippy): the three openclaw fixture tests pass; cargo test --workspace now gets past roko-acp and roko-agent and stops only at roko-cli's doctor tests (dec-01be49)"
+++

## Problem

Three roko-agent lib tests fail on CI (ubuntu-latest) and pass on macOS:
`openclaw::infer_agent::fixture_tests::{fixture_model_run_basic, fixture_model_run_empty_output,
fixture_usage_estimation}` (`assertion failed: result.success`). Seen on PR #84's `cargo test --workspace`, the first
run that got past roko-acp (bug-68a33f had stopped the step earlier, so these were never reached).

## Why it matters

`cargo test --workspace` stops at the first failing test binary, so these failures hide every crate after
roko-agent from CI.

## Where

`crates/roko-agent/src/openclaw/infer_agent.rs::fixture_tests::fake_openclaw_script`.

## Current state

The helper wrote a shell script into a `NamedTempFile`, made it executable and returned the open file; the tests then
ran it as the `openclaw` binary. Linux refuses to execute a file that is still open for writing (`ETXTBSY`), so the
spawn failed; macOS allows it.

## Plan

Return `NamedTempFile::into_temp_path()`, which closes the write handle and still deletes the file on drop.

## Done when

- [ ] The fixture tests pass on CI's Linux runner.
- [ ] `[[verify]]` passes.
