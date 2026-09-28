+++
id = "bug-e7b27f"
kind = "bug"
title = "DA-bak-15: Cascade router keeps role_table entries for unconfigured models (silent fallbacks)"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-learn/cascade-router"]
created = 2026-09-02
updated = 2026-09-28
source = "tmp/archive/dev-audit-2026-09-21/dev-audit-backup/15-provider-model-config.md#The Cascade Router Problem"
discovered_from = "audit:tmp/archive/dev-audit-2026-09-21/dev-audit-backup/15-provider-model-config.md#The Cascade Router Problem"
anchors = ["detect_version_changes", "model_routing.rs", "cascade-router.json"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
On snapshot load, detect_version_changes leaves models with no configured provider (claude-opus, gemini-2.5-flash-lite) in role_table; dispatch then silently falls back to the default (448 warnings in one day). Proposed fix (prune role_table on load) is not in the fix runbook.

Imported without verification from:
- `tmp/archive/dev-audit-2026-09-21/dev-audit-backup/15-provider-model-config.md#The Cascade Router Problem`
- `tmp/archive/dev-audit-2026-09-21/dev-audit-backup/15-provider-model-config.md#Solutions`

How to verify: Load a router snapshot naming an unconfigured model; check prune/warn behavior.
