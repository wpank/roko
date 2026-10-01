+++
id = "bug-a70def"
kind = "bug"
title = "The Claude MCP isolation tests assume the host has no managed-mcp.json"
status = "done"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["roko-agent/claude_cli"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "db778db9b"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-cc-isolate's report on gap-b7a2d5, branch work/gap-b7a2d5 at 42859fc78)"
anchors = ["crates/roko-agent/src/claude_cli_agent.rs"]
lane = "rust-cold"
parent = "spec-9a3131"
links = { depends_on = ["gap-b7a2d5"], blocks = [], related = ["gap-b7a2d5"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn mcp_isolation_tests_ignore_the_hosts_managed_mcp_json' crates/roko-agent/src/ && cargo test -p roko-agent --lib mcp_isolation_tests_ignore_the_hosts_managed_mcp_json"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T18:45:24Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-01T16:13:00Z"
forced = false
evidence = "Gate 6b on 9e32a0d64, merged as db778db9b (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 11 crates; lib tests pass (roko-cli 3375, roko-agent 2241, roko-core 1971, roko-learn 1216, roko-serve 1003, roko-gate 696, roko-compose 561, roko-graph 483, roko-execution 245, roko-acp 200); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, gemini, dispatch-feedback, e2e_domain, run_serve_share, property, job_runner and plan_execute integration tests pass; bin 447; Cargo.lock unchanged; portal tsc clean and vitest 800/800. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++

## Problem

Claude Code reads a system-wide `managed-mcp.json` that can't be moved ("Claude Code 2.1.282 has no way to move it", `claude_cli_agent.rs:137` on gap-b7a2d5's branch). The isolation code checks for it (:150). Some MCP tests build their own managed directory (:2502, :2549), but wk-cc-isolate reports that the MCP tests as a whole assume the host has no `managed-mcp.json`. On a machine with one, they fail or test the wrong thing.

## Why it matters

Hygiene (epic spec-9a3131): the tests depend on the machine. p3.

## Where

The MCP isolation tests in `claude_cli_agent.rs`, and the function that locates the managed file.

## Plan

1. Make the managed-file directory injectable (a parameter, or an env override used only in tests), and point every test at a temporary directory.
2. Add `mcp_isolation_tests_ignore_the_hosts_managed_mcp_json`, which passes with and without a host file.

## Done when

- [ ] The MCP tests pass whether or not the host has a `managed-mcp.json`.
- [ ] The `[[verify]]` command passes.

## Notes

- 2026-10-01 (wk-guard2): implemented on work/bug-a70def; cargo verification deferred to the batch check.
- At BASE the directory was already injectable (`ClaudeIsolation::with_managed_settings_dir`), but `ClaudeCliAgent::new` read the host's real one, so `command_isolates_claude_code_from_the_user_configuration`, `only_explicit_caller_choices_widen_the_isolation` and `runs_fake_claude_binary_and_passes_flags` (one `--strict-mcp-config` each) would fail on a host with a `managed-mcp.json`. `claude_managed_settings_dir` now returns a path that never exists when this crate's tests are compiled (the crate's `cfg!(test)` idiom); tests that want a managed config pass their own directory, as `a_managed_mcp_config_is_reported_before_the_run` already did. New test `mcp_isolation_tests_ignore_the_hosts_managed_mcp_json`.
