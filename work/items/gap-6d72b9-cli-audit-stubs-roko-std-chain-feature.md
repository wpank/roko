+++
id = "gap-6d72b9"
kind = "gap"
title = "[cli-audit stubs] roko-std `chain` feature never enabled; 17 chain tool handlers compiled out"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-std"]
created = 2026-09-14
updated = 2026-09-28
source = "tmp/archive/cli-audit-2026-09-21/SUMMARY.md#Dead / Stale Code"
discovered_from = "audit:tmp/archive/cli-audit-2026-09-21/SUMMARY.md#Dead / Stale Code"
anchors = ["crates/roko-cli/Cargo.toml", "crates/roko-std/Cargo.toml features.chain", "backlog #338"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
roko-cli depends on roko-std without the `chain` feature, so 17 chain tool handlers are compiled out of the shipped binary. SUMMARY marks this OPEN even though #338 (workspace feature matrix) is checked; disposition (intentional optional vs. missing) unclear.

Imported without verification from:
- `tmp/archive/cli-audit-2026-09-21/SUMMARY.md#Dead / Stale Code`
- `tmp/archive/cli-audit-2026-09-21/19-feature-flags.md`

How to verify: Check roko-cli Cargo.toml roko-std features; check #338 feature-matrix doc records chain as intentionally off-by-default.
