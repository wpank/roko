+++
id = "bug-17544e"
kind = "bug"
title = "main.rs help text still documents plan run flags that plan run now rejects"
status = "open"
triage = "unverified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["roko-cli/main"]
created = 2026-10-01
updated = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "gap-d60281"
anchors = ["crates/roko-cli/src/main.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["gap-d60281"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-cli --bin roko plan_run_help"
+++

## Problem

gap-d60281 made `roko plan run` fail on the seven flags the Graph engine ignored. Their `#[arg]` help text in `main.rs` still describes them as working options.

## Plan

Hide the rejected flags from `--help` (`hide = true`), or reword them as rejected, and add a test named `plan_run_help_*` that checks `--help` no longer advertises them.

## Done when

- `cargo test -p roko-cli --bin roko plan_run_help` passes.

## Notes

- Reported on 2026-10-01 by the worker on gap-d60281, during the evening close-out round.
