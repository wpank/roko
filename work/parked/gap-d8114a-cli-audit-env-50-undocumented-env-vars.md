+++
id = "gap-d8114a"
kind = "gap"
title = "[cli-audit env] ~50+ undocumented env vars incl. `ROKO__*` hierarchical override"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-core/config"]
created = 2026-09-14
updated = 2026-09-28
source = "tmp/archive/cli-audit-2026-09-21/SUMMARY.md#Recommended Priority Order"
discovered_from = "audit:tmp/archive/cli-audit-2026-09-21/SUMMARY.md#Recommended Priority Order"
anchors = ["roko-core/src/config/loader.rs apply_env_overrides", "roko config env", "backlog #339"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
~105 env vars, ~50+ undocumented operator knobs (ROKO_BUDGET_USD, ROKO_SKIP_TESTS...), undocumented ROKO__* hierarchical override and 12 ROKO_* config overrides. SUMMARY item 11 OPEN though #339 registry is checked.

Imported without verification from:
- `tmp/archive/cli-audit-2026-09-21/SUMMARY.md#Recommended Priority Order`
- `tmp/archive/cli-audit-2026-09-21/20-environment-variables.md`

How to verify: Compare `roko config env` output against grep of env::var("ROKO_ across crates.
