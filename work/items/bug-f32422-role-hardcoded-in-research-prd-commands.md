+++
id = "bug-f32422"
kind = "bug"
title = "`--role` hardcoded in research/PRD commands"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/cli"]
created = 2026-09-15
updated = 2026-09-28
source = "docs/v3/39-ROADMAP.md#3.4 CLI Surface Fixes"
discovered_from = "audit:docs/v3/39-ROADMAP.md#3.4 CLI Surface Fixes"
anchors = ["crates/roko-cli/src/prd.rs", "roko research"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Roadmap §3.4: the --role flag is hardcoded in research and PRD commands instead of using the specified role.

Imported without verification from:
- `docs/v3/39-ROADMAP.md#3.4 CLI Surface Fixes`

How to verify: grep research/prd command handlers for fixed role strings.
