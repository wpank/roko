+++
id = "gap-811571"
kind = "gap"
title = "Per-Plan Config Overrides and Per-Task Routing Metadata"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli"]
created = 2026-09-07
updated = 2026-09-28
source = "tmp/backlog/archive/180-per-plan-config-overrides.md#180 — Per-Plan Config Overrides and Per-Task Routing Metadata"
discovered_from = "audit:tmp/backlog/archive/180-per-plan-config-overrides.md#180 — Per-Plan Config Overrides and Per-Task Routing Metadata"
anchors = ["crates/roko-cli/src/task_parser.rs", "crates/roko-core/src/config/schema.rs", "crates/roko-core/src/config/presets.rs", "crates/roko-cli/src/runner/types.rs", "crates/roko-cli/src/commands/plan.rs", "roko-core/src/config/schema.rs", "crates/roko-cli/src/runner/event_loop.rs", "crates/roko-cli/src/commands/validate.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
enables fine-grained model/provider control that operators need for cost management and quality tuning on long multi-plan runs. Mori supported two mechanisms for per-plan and per-task routing control:

Imported without verification from:
- `tmp/backlog/archive/180-per-plan-config-overrides.md#180 — Per-Plan Config Overrides and Per-Task Routing Metadata`
- `tmp/backlog/_archive/_mori-old-gaps.md#MO-10`

Some cited files are gone: `crates/roko-cli/src/commands/validate.rs`, `crates/roko-cli/src/runner/event_loop.rs`.

How to verify: Check: Task TOML files can include `category`, `reasoning_level`, `speed_priority`, `preferred_model`, and `preferred_provider` fields without breaking parsing.; `[plan_overrides."my-plan"]` in `roko.toml` overrides model/provider/effort for that… [evidence: 00-STATUS-SUMMARY 3. Open / P2 -- Medium (Open): M | 5 |]
