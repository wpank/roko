+++
id = "gap-2911a1"
kind = "gap"
title = "Budget Safety Defaults"
status = "superseded"
triage = "verified"
severity = "p0"
subsystem = ["roko-learn"]
created = 2026-09-21
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/backlog/archive/408-budget-safety-defaults.md#408 — Budget Safety Defaults"
discovered_from = "audit:tmp/backlog/archive/408-budget-safety-defaults.md#408 — Budget Safety Defaults"
anchors = ["crates/roko-core/src/config/budget.rs:122", "crates/roko-cli/src/commands/util.rs", "crates/roko-learn/src/budget.rs"]
links = { depends_on = [], blocks = [], related = ["bug-b81021", "q-e23804"], supersedes = [], duplicate_of = "gap-4665ac" }

[closed]
at = 2026-09-28
evidence = "duplicate of gap-4665ac: backlog #408's defect is the 0.0 (unlimited) defaults in crates/roko-core/src/config/budget.rs:122-126, still present 2026-09-28. The #408 acceptance criteria ($10/$50/$3 defaults, roko learn inspect budget output) belong on gap-4665ac / decision q-e23804. The files the import flagged as gone (commands/util.rs, config/budget.rs, roko-learn/src/budget.rs) all exist."
+++
a fresh workspace has no spend protection; stuck agent loop can exhaust API credit overnight. The canonical `BudgetConfig` in `roko-core/src/config/budget.rs` defaults every cost ceiling to `0.0`, which in roko's enforcement semantics means unlimited. The enforcement guards in `commands/util.rs`…

Imported without verification from:
- `tmp/backlog/archive/408-budget-safety-defaults.md#408 — Budget Safety Defaults`

How to verify: Check: Create a fresh workspace with no `[budget]` section in `roko.toml`.; Run `roko learn inspect budget`. Verify the output shows `max_plan_usd = 10.00`,; Run `cargo test --workspace`. Fix any tests that relied on unlimited defaults. [evidence: no status line; no index/roll-up evidence]

Verified 2026-09-28: still true (budget.rs:122-126) but duplicate of gap-4665ac (p3 there vs p0 here: severity needs one decision).
