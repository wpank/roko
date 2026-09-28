+++
id = "gap-4665ac"
kind = "gap"
title = "Budget ceilings default to 0 (no plan or turn spend limit)"
status = "open"
triage = "verified"
severity = "p3"
goal = "release"
subsystem = ["roko-core/config"]
created = 2026-09-01
updated = 2026-09-28
last_verified = 2026-09-28
source = "gaps-md#from-cli-audit-tmpcli-auditsummarymd/budget-disabled"
anchors = ["crates/roko-core/src/config/budget.rs:122"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++

`BudgetConfig` defaults to `max_plan_usd = 0.0` and `max_turn_usd = 0.0` (`crates/roko-core/src/config/budget.rs:122-124`), and the loader treats 0.0 as unset. Plan and turn budgets are therefore not enforced unless a user sets them. Without `max_turn_usd`, Graph cost reservation also runs calls one at a time. Backlog #408 (budget safety defaults) is archived without a status.

Fix: pick safe non-zero defaults, or require an explicit value at `roko init`. Warn when a plan runs without a ceiling.
