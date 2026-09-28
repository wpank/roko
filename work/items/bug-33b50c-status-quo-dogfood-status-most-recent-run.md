+++
id = "bug-33b50c"
kind = "bug"
title = "[status-quo dogfood] `status` most-recent-run reads Runner-v2 snapshot, not Graph runs"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/status"]
created = 2026-09-10
updated = 2026-09-28
source = "tmp/archive/status-quo-audit-2026-09-21/10-dogfood-proof.md#Known display gaps (not failures)"
discovered_from = "audit:tmp/archive/status-quo-audit-2026-09-21/10-dogfood-proof.md#Known display gaps (not failures)"
anchors = [".roko/state/state-snapshot.json", ".roko/state/graph/", "backlog #323"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
`roko status` 'most recent run' refers to the last Runner-v2 state-snapshot.json rather than Graph engine runs; engines report to different state locations (despite #323 canonical status source).

Imported without verification from:
- `tmp/archive/status-quo-audit-2026-09-21/10-dogfood-proof.md#Known display gaps (not failures)`

Some cited files are gone: `.roko/state/state-snapshot.json`.

How to verify: Run a Graph plan then `roko status`; check which run is reported.
