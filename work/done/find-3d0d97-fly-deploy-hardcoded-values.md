+++
id = "find-3d0d97"
kind = "finding"
title = "Fly deploy hardcoded values"
status = "done"
triage = "verified"
severity = "p3"
subsystem = ["roko-cli/deploy"]
created = 2026-09-01
updated = 2026-10-04
last_verified = 2026-10-04
last_verified_rev = "cb8cbfeed"
source = "tmp/archive/MASTER-ACTION-PLAN-2026-09-23.md#6.15 Fly deploy hardcoded values"
discovered_from = "audit:tmp/archive/MASTER-ACTION-PLAN-2026-09-23.md#6.15 Fly deploy hardcoded values"
anchors = []
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[closed]
at = 2026-10-04
at_ts = "2026-10-04T04:50:27Z"
commit = "cb8cbfeed"
forced = false
evidence = "PK85's task 9336 (gap-fcb44c, merged cb8cbfeed): roko deploy fly reads the app name and region from an existing Fly config (--fly-config) instead of the hardcoded roko-agent and iad, takes a build target and checks the G0-G2 posture; test deploy_fly_dry_run_reads_app_and_region_from_config passes at gate 13c."
+++
Fly deploy hardcodes app name `roko-agent` and region `iad`. Make configurable.

Imported without verification from:
- `tmp/archive/MASTER-ACTION-PLAN-2026-09-23.md#6.15 Fly deploy hardcoded values`

How to verify: Source: CLI audit report 12. Check the described code path for: Fly deploy hardcodes app name `roko-agent` and region `iad`. Make configurable.
