+++
id = "find-6f91c7"
kind = "finding"
title = "[cli-audit F189] ~25 hardcoded slug-heuristic tables override config for model behavior"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-agent/models"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/cli-audit-2026-09-21/08-config.md#Validation coverage note (2026-09-01, provider-audit F188/F189)"
discovered_from = "audit:tmp/archive/cli-audit-2026-09-21/08-config.md#Validation coverage note (2026-09-01, provider-audit F188/F189)"
anchors = ["tmp/provider-audit/31-slug-heuristic-map.md", "tui/state.rs:372-421", "tui/views/config_view.rs:680-704"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Valid [models.*] config is not authoritative: ~25 slug-heuristic tables govern thinking classification, tool format/caps, cost accounting and cascade routing; TUI infers context limits/tier/provider from slugs (triplicated attribution).

Imported without verification from:
- `tmp/archive/cli-audit-2026-09-21/08-config.md#Validation coverage note (2026-09-01, provider-audit F188/F189)`

Some cited files are gone: `tmp/provider-audit/31-slug-heuristic-map.md`, `tui/state.rs`.

How to verify: grep for slug.contains/starts_with model-family heuristics across roko-agent/roko-learn/roko-cli tui.
