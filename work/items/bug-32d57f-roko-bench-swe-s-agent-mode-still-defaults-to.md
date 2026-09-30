+++
id = "bug-32d57f"
kind = "bug"
title = "roko bench swe's --agent-mode still defaults to gold; the flag should be required"
status = "done"
triage = "verified"
severity = "p2"
goal = "release"
size = "S"
subsystem = ["roko-cli/main", "roko-cli/bench"]
created = 2026-09-30
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "2ae9d2a7f"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-honestbench's report, checked on work/bug-28becc at abb181f65)"
anchors = ["crates/roko-cli/src/main.rs"]
lane = "rust-cold"
links = { depends_on = ["bug-28becc"], blocks = [], related = ["bug-28becc"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -q 'default_value_t = roko_cli::bench::SweAgentMode::Gold' crates/roko-cli/src/main.rs"

[closed]
at = 2026-09-30
by = "coordinator (session 7622b882)"
evidence = "Merged in 89028c96f. roko bench swe requires --agent-mode. Batch 14 gate: first run on 4e030ab47 (check, clippy clean; tests pass: roko-cli 3171, roko-agent 2263, roko-core 1952, roko-learn 1203, roko-serve 986, roko-graph 472, roko-fs 259, roko-neuro 239), then re-gated on 8ce3bb131 (same code as MAIN 2ae9d2a7f) after the coordinator's rustfmt commits and serve-sec's bug-633b68 root fix: check, nightly fmt, clippy -p roko-cli -p roko-serve -p roko-core -p roko-agent -p roko-learn --keep-going -D warnings clean; roko-cli lib 3172 passed (one sibling-settle race flake passes alone, bug-779ae7); --test secret_canary 11 passed; --test secrets_and_git_guard_canary 1 passed, 1 ignored (bug-0d9ac4). Verify: static checks pass on MAIN."
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

## Notes

- **wk-honestbench (2026-09-30):** Implemented on `work/bug-32d57f` at `bad522714`; cargo verification deferred to the batch check. Parse test: `cli_bench_swe_requires_agent_mode` (main.rs). The v2 and v3 CLI references now list the flag as required.
