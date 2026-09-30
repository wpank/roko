+++
id = "bug-32d57f"
kind = "bug"
title = "roko bench swe's --agent-mode still defaults to gold; the flag should be required"
status = "open"
triage = "unverified"
severity = "p2"
goal = "release"
size = "S"
subsystem = ["roko-cli/main", "roko-cli/bench"]
created = 2026-09-30
updated = 2026-09-30
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-honestbench's report, checked on work/bug-28becc at abb181f65)"
anchors = ["crates/roko-cli/src/main.rs"]
lane = "rust-cold"
links = { depends_on = ["bug-28becc"], blocks = [], related = ["bug-28becc"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -q 'default_value_t = roko_cli::bench::SweAgentMode::Gold' crates/roko-cli/src/main.rs"
+++

## Problem

`roko bench swe`'s `--agent-mode` defaults to `gold` (`crates/roko-cli/src/main.rs:1695`, `default_value_t = roko_cli::bench::SweAgentMode::Gold`). In gold mode the bench applies the reference patch. A run that forgets the flag measures the gold patch, not an agent, and its results look like an agent's.

## Why it matters

Release: bug-28becc is about the gold patch leaking into results. A default of gold makes that the path of least resistance.

## Where

The `--agent-mode` argument in `main.rs`.

## Plan

1. Make `--agent-mode` required (no default), and label gold runs as such in every record.

## Done when

- [ ] `roko bench swe` refuses to run without an explicit `--agent-mode`.
- [ ] The `[[verify]]` command passes.
