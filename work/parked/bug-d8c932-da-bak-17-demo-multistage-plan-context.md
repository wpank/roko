+++
id = "bug-d8c932"
kind = "bug"
title = "DA-bak-17: demo-multistage plan context files resolved against the wrong root"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/plan_policy"]
created = 2026-09-02
updated = 2026-09-28
source = "tmp/archive/dev-audit-2026-09-21/dev-audit-backup/17-state-corruption.md#3. demo-multistage Plan Loading Failure"
discovered_from = "audit:tmp/archive/dev-audit-2026-09-21/dev-audit-backup/17-state-corruption.md#3. demo-multistage Plan Loading Failure"
anchors = ["plans/demo-multistage/tasks.toml", "PLAN_CONTEXT_MISSING", "plan_policy.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
plans/demo-multistage was skipped on every startup: read_files context (README.md, Cargo.toml, docs/v1/...) failed strict PLAN_CONTEXT_MISSING validation because paths were resolved against the wrong root.

Imported without verification from:
- `tmp/archive/dev-audit-2026-09-21/dev-audit-backup/17-state-corruption.md#3. demo-multistage Plan Loading Failure`

How to verify: Run `roko plan validate plans/demo-multistage`.
