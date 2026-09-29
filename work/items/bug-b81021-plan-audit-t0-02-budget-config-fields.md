+++
id = "bug-b81021"
kind = "bug"
title = "[plan-audit T0-02] Budget config fields default to 0.0 (unlimited)"
status = "superseded"
triage = "verified"
severity = "p1"
subsystem = ["roko-core/config"]
created = 2026-09-21
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T0-02: Set non-zero budget defaults"
discovered_from = "audit:tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T0-02: Set non-zero budget defaults"
anchors = ["crates/roko-core/src/config/budget.rs:122"]
links = { depends_on = [], blocks = [], related = ["gap-2911a1", "q-e23804"], supersedes = [], duplicate_of = "gap-4665ac" }

[closed]
at = 2026-09-28
evidence = "duplicate of gap-4665ac: same defect. BudgetConfig::default() still sets max_plan_usd/max_turn_usd/max_daily_usd = 0.0 (unlimited) at crates/roko-core/src/config/budget.rs:122-126 (checked 2026-09-28). Note gap-4665ac is triaged p3; this import was p1."
+++
max_plan_usd, max_daily_usd, max_turn_usd code defaults are 0.0 = unlimited; a stuck loop can exhaust API credit. Proposed $10/$50/$3. Checked-in roko.toml sets values, but code defaults remain unlimited. Backlog #408.

Imported without verification from:
- `tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T0-02: Set non-zero budget defaults`
- `tmp/archive/plan-audit-2026-09-23/16-budget-defaults.md`

How to verify: Check Default impls for budget fields; run with an empty roko.toml.

Verified 2026-09-28: still true (budget.rs:122-126 defaults 0.0) but duplicate of gap-4665ac.
