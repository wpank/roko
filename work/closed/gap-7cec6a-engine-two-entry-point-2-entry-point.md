+++
id = "gap-7cec6a"
kind = "gap"
title = "[engine two-entry-point] 2-entry-point CLI reduction (`roko` + `roko do`; run/develop as aliases) not tracked"
status = "superseded"
triage = "verified"
severity = "p3"
subsystem = ["roko-cli/commands"]
created = 2026-09-01
updated = 2026-10-02
last_verified = 2026-10-02
last_verified_rev = "bfd36512f"
source = "tmp/archive/engine-audit/21-backlog-cross-reference.md#identified-gaps"
discovered_from = "audit:tmp/archive/engine-audit/21-backlog-cross-reference.md#identified-gaps"
anchors = ["crates/roko-cli/src/main.rs", "crates/roko-cli/src/commands/"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[closed]
at = 2026-10-02
at_ts = "2026-10-02T20:40:17Z"
commit = "bfd36512f"
by = "roko-7d"
executor = "claude-session"
via = "manual"
forced = false
evidence = "Superseded by the 2026-10-02 workflow-audit migration (merge bfd36512f): one entry point, `roko run`; `roko do` is a removed stub and `roko develop` is gone."
+++
Cross-reference item 12 'CLI UX cleanup' partial: audit Phase 5 envisioned collapsing to `roko` (conversation) and `roko do` (execution) with run/develop as aliases and no silent intent classification; #65 only hides verbs; #77 CLI UX consistency was 4/8 fixed. Product decision.

Imported without verification from:
- `tmp/archive/engine-audit/21-backlog-cross-reference.md#identified-gaps`
- `tmp/archive/engine-audit/11-cli-ux-consolidation.md`

How to verify: Check top-level verb count/aliases and whether #77's remaining 4/8 UX fixes landed; confirm whether 2-entry-point consolidation is still desired.
