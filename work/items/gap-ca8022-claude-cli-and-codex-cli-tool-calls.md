+++
id = "gap-ca8022"
kind = "gap"
title = "Claude CLI and Codex CLI tool calls leave no safety provenance"
status = "open"
triage = "unverified"
severity = "p3"
goal = "truth"
size = "S"
subsystem = ["roko-cli", "roko-agent"]
created = 2026-10-02
updated = 2026-10-02
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "gap-ff95f5"
anchors = ["crates/roko-agent/src/claude_cli_agent.rs", "crates/roko-cli/src/safety_provenance.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["gap-ff95f5"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-cli --lib cli_attempts_record_provenance"
+++

## Problem

CLI providers (Claude CLI, Codex CLI) run their tools outside ToolDispatcher, so gap-ff95f5's provenance sink sees none of their tool calls. That item allows at best one record per dispatch turn for them.

## Plan

Record one provenance record per CLI dispatch turn, from the live-output tap's tool events. Add a test named `cli_attempts_record_provenance`.

## Done when

- The test passes.

## Notes

- Reported on 2026-10-02 by wk-tamper, working on gap-ff95f5, during the overnight close-out round.
