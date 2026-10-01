+++
id = "bug-09ffa1"
kind = "bug"
title = "DF-0925 P1-7: TUI tokens and cost stay at $0.00 (TuiBridge::token_usage never called)"
status = "superseded"
triage = "verified"
severity = "p1"
subsystem = ["roko-cli/tui_bridge"]
created = 2026-09-25
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/dogfood/2026-09-25-portal-programme-run.md#P1-7. Tokens and cost are structurally $0.00"
discovered_from = "audit:tmp/dogfood/2026-09-25-portal-programme-run.md#P1-7. Tokens and cost are structurally $0.00"
anchors = ["crates/roko-cli/src/runner/tui_bridge.rs::TuiBridge::token_usage"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "gap-038eaa" }

[closed]
at = 2026-09-28
evidence = "duplicate of gap-038eaa (verified 2026-09-28): TuiBridge::token_usage (runner/tui_bridge.rs:329) still has no Graph-path caller, only the runner/output_sink.rs forwarding impls and their tests."
+++
token_usage has zero callers, so stats.cost_usd_total and token counters stay 0 and the header shows $0.00, although efficiency.jsonl has the real numbers.

Imported without verification from:
- `tmp/dogfood/2026-09-25-portal-programme-run.md#P1-7. Tokens and cost are structurally $0.00`

How to verify: grep token_usage callers; run a task and check header cost.

Verified 2026-09-28: closed as duplicate; see [closed].evidence.
