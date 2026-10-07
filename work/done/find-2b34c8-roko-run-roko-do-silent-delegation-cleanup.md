+++
id = "find-2b34c8"
kind = "finding"
title = "`roko run` / `roko do` silent delegation cleanup"
status = "done"
triage = "verified"
severity = "p3"
subsystem = ["roko-cli"]
created = 2026-09-01
updated = 2026-10-02
last_verified = 2026-10-02
last_verified_rev = "bfd36512f"
source = "tmp/archive/MASTER-ACTION-PLAN-2026-09-23.md#6.9 `roko run` / `roko do` silent delegation cleanup"
discovered_from = "audit:tmp/archive/MASTER-ACTION-PLAN-2026-09-23.md#6.9 `roko run` / `roko do` silent delegation cleanup"
anchors = ["roko run", "roko do"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[closed]
at = 2026-10-02
at_ts = "2026-10-02T20:40:15Z"
commit = "bfd36512f"
by = "roko-7d"
executor = "claude-session"
via = "manual"
forced = false
evidence = "Done in merge bfd36512f: `roko run` no longer delegates to `roko do`. `commands/run_cmd.rs::cmd_run` routes a prompt to one checked task or to a plan written first (`resolve_run_route`, table-tested), and `roko do` is a hidden stub that exits 1 naming `roko run`."
+++
`roko run` silently delegates to `roko do` based on flags. `roko do` classifies prompt intent via keyword matching. Single-word prompts match plan slugs on disk. Make routing explicit or move to graph templates.

Imported without verification from:
- `tmp/archive/MASTER-ACTION-PLAN-2026-09-23.md#6.9 `roko run` / `roko do` silent delegation cleanup`

How to verify: Source: Engine audit report 04 (18 instances of silent delegation). Check the described code path for: `roko run` silently delegates to `roko do` based on flags. `roko do` classifies prompt intent via keyword matching. Single-word prompts match plan slugs on…
