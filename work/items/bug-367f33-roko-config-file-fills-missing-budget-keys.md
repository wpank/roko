+++
id = "bug-367f33"
kind = "bug"
title = "roko --config <file> fills missing [budget] keys with the CLI's legacy defaults ($10 per plan, $1 per task) instead of core [budget]'s"
status = "open"
triage = "verified"
severity = "p2"
goal = "release"
size = "S"
subsystem = ["roko-cli/config", "roko-core/config"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "407ce30d5"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (17:09, wk-onboard's report on bug-e1327f, branch work/bug-e1327f)"
anchors = ["crates/roko-cli/src/config.rs::BudgetConfig", "crates/roko-core/src/config/budget.rs::BudgetConfig"]
lane = "rust-cold"
parent = "spec-ae5f94"
links = { depends_on = [], blocks = [], related = ["bug-e1327f"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn config_flag_budget_defaults_match_core' crates/roko-cli/src/ && cargo test -p roko-cli --lib config_flag_budget_defaults_match_core"
+++

## Problem

roko-cli still has its own legacy `BudgetConfig` (`crates/roko-cli/src/config.rs:504`). Its serde defaults are `max_plan_usd = 10.0` and `max_task_usd = 1.0`. Core's `[budget]` (`crates/roko-core/src/config/budget.rs:27`) defaults both to `0.0`, which means unlimited. wk-onboard found that `roko --config <file>` loads the file through the legacy struct, so a `[budget]` table that omits a key gets $10 per plan and $1 per task, while the same file loaded as the workspace `roko.toml` gets core's defaults.

## Why it matters

Release blockers (epic spec-ae5f94): the same config file gives different budget limits depending on how it is passed, so a run started with `--config` can stop at $1 per task for no visible reason.

## Where

- `crates/roko-cli/src/config.rs::BudgetConfig` (:504 on `work/bug-e1327f`): `default_max_plan` and `default_max_task`.
- `crates/roko-core/src/config/budget.rs::BudgetConfig`: core's fields and defaults.
- The `--config` load path in roko-cli: start from where the global `--config` flag is resolved into a config.

## Current state

The two sets of defaults were checked on `work/bug-e1327f` (`fab9168a9`). This check did not trace the `--config` path itself; the report did.

## Plan

1. Make `--config` load through core's loader, like the workspace file, or build the CLI's budget from core's `[budget]` defaults instead of its own.
2. Remove or deprecate the legacy defaults, so there is one source of budget defaults.
3. Add `config_flag_budget_defaults_match_core`: a file with an empty `[budget]` table, loaded through `--config`, gives the same limits as the same file loaded as `roko.toml`.

## Done when

- [ ] A key missing from `[budget]` gets core's default however the file is passed.
- [ ] The `[[verify]]` command passes.

## Notes

- Premise confirmed at `407ce30d5` with `target/debug/roko` (built at `33e107da1`): `roko --config other.toml run` with `[agent] command = "claude"` and an empty `[budget]` fails admission with `predicted turn cost $1.5000 exceeds max_turn_usd $1.0000`. `resolve_config_for_workdir` loads `--config` through `Config::from_file`, whose `BudgetConfig` defaulted to $10 plan, $1 task and turn, $50 session.
- The CLI type's serde defaults now read core's `BudgetConfig::default()`; `max_session_usd`, the v1 name of `max_plan_usd`, follows the plan cap.
- Overlap: neither gap-4665ac nor bug-ae28ac covers this, and this covers neither. gap-4665ac asks whether core's defaults should stay 0.0 (parked as q-778b4f and q-e23804); after this change `--config` follows whatever core picks. bug-ae28ac is about the Graph engine enforcing `max_daily_usd` and `max_agent_lifetime_usd`. bug-4ed3c2 (`plan run` ignores `--config`) is a different path.
- Implemented on `work/bug-12153c` at `940c12561`; cargo verification deferred to the batch check.
