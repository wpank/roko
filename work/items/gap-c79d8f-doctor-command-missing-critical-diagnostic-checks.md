+++
id = "gap-c79d8f"
kind = "gap"
title = "Doctor Command Missing Critical Diagnostic Checks"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli"]
created = 2026-09-07
updated = 2026-09-28
source = "tmp/backlog/archive/104-doctor-diagnostic-coverage.md#104 — Doctor Command Missing Critical Diagnostic Checks"
discovered_from = "audit:tmp/backlog/archive/104-doctor-diagnostic-coverage.md#104 — Doctor Command Missing Critical Diagnostic Checks"
anchors = ["crates/roko-cli/", "state-snapshot.json", "tasks.toml", "doctor.rs", "Cargo.toml", "crates/roko-cli/src/doctor.rs", "crates/roko-cli/Cargo.toml", "TasksFile::validate()"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
reliability/UX; `roko doctor` currently misses checks that would catch. `roko doctor` is the workspace health command (`roko doctor` in the CLI). It is the first thing a user should run when something goes wrong, and it is also run internally before plan execution. Currently it runs 22 diagnostic…

Imported without verification from:
- `tmp/backlog/archive/104-doctor-diagnostic-coverage.md#104 — Doctor Command Missing Critical Diagnostic Checks`

How to verify: Check: `roko doctor` samples each canonical JSONL file and reports a `[warn]` if any of the; `roko doctor` parses every `plans/*/tasks.toml` and reports `[warn]` for validation; On Unix systems, `roko doctor` checks the soft file descriptor limit… [evidence: 00-STATUS-SUMMARY 3. Open / P2 -- Medium (Open): M | 3 |]
