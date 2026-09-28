+++
id = "gap-4e23dc"
kind = "gap"
title = "[cli-audit I2] Invalid config/replay/sync/job/deploy input must fail before locks or mutation"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli"]
created = 2026-09-14
updated = 2026-09-28
source = "tmp/archive/cli-audit-2026-09-21/IMPLEMENTATION-CHECKLIST.md#Wave 2 integration node I2"
discovered_from = "audit:tmp/archive/cli-audit-2026-09-21/IMPLEMENTATION-CHECKLIST.md#Wave 2 integration node I2"
anchors = ["roko config/replay/knowledge sync/job/deploy commands"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Wave 2 integration criterion unchecked: invalid config/replay/sync/job/deploy input should fail before lock acquisition, provider construction or mutation. Never verified.

Imported without verification from:
- `tmp/archive/cli-audit-2026-09-21/IMPLEMENTATION-CHECKLIST.md#Wave 2 integration node I2`

How to verify: Feed invalid args to each command; confirm no lock files/state writes/provider init occur.
