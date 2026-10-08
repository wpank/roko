+++
id = "bug-d986b2"
kind = "bug"
title = "Fix Efficiency Log WRN Threshold ($1 vs $5)"
status = "done"
triage = "verified"
severity = "p2"
goal = "tooling"
subsystem = ["roko-cli/tui"]
created = 2026-09-21
updated = 2026-10-08
last_verified = 2026-10-08
last_verified_rev = "ad7a3a337"
source = "tmp/backlog/archive/379-fix-efficiency-log-threshold.md#379 — Fix Efficiency Log WRN Threshold ($1 vs $5)"
discovered_from = "audit:tmp/backlog/archive/379-fix-efficiency-log-threshold.md#379 — Fix Efficiency Log WRN Threshold ($1 vs $5)"
anchors = ["crates/roko-cli/src/tui/state/signals.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'fn unified_log_warns_only_on_turns_above_five_dollars' crates/roko-cli/src/tui/state/tests.rs && cargo test -p roko-cli --lib unified_log_warns_only_on_turns_above_five_dollars"

[closed]
at = 2026-10-08
at_ts = "2026-10-08T13:21:50Z"
commit = "276aea948"
forced = false
evidence = "gate 24 (2026-10-08): its verify passes; check, nightly fmt, clippy -D warnings, CI feature checks, nextest --workspace --lib (15223 passed), touched crates' full tests, roko-acp integration, roko-cli bin (457) and CI canaries all pass"
+++
efficiency log spam floods TUI at low cost. The running-audit claimed the WRN threshold was raised from $1 to $5, but verification at line 127 shows `if event.cost_usd > 1.0 { LogEntryLevel::Warn }`. The $1 threshold causes excessive WARN-level entries for routine operations.

Imported without verification from:
- `tmp/backlog/archive/379-fix-efficiency-log-threshold.md#379 — Fix Efficiency Log WRN Threshold ($1 vs $5)`

How to verify: Check whether the gap described in tmp/backlog/archive/379-fix-efficiency-log-threshold.md still exists at the anchored paths. [evidence: 00-INDEX (2026-09-21) listed active: 2026-09-21 Audit Sweep Items (#376-#395)]
