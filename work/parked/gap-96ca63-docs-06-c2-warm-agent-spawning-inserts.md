+++
id = "gap-96ca63"
kind = "gap"
title = "DOCS-06 C2: Warm agent spawning inserts placeholders, not pre-spawned processes"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-agent/pool"]
created = 2026-09-15
updated = 2026-09-28
source = "tmp/docs-audit/06-POTENTIAL-BACKLOG.md#C2. Warm Agent Spawning"
discovered_from = "audit:tmp/docs-audit/06-POTENTIAL-BACKLOG.md#C2. Warm Agent Spawning"
anchors = ["WarmPool", "roko-agent pools"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
WarmPool held placeholder structs (docs-audit). P3-AGT-3 marked warm pre-spawning done 2026-09-19, but the 2026-09-25 run found warm_pool.rs still says it does not pre-spawn real agents and was_warm_start=false on every record.

Imported without verification from:
- `tmp/docs-audit/06-POTENTIAL-BACKLOG.md#C2. Warm Agent Spawning`
- `tmp/dogfood/2026-09-19-session.md#Fixes Applied This Session`
- `tmp/dogfood/2026-09-25-portal-programme-run.md#P2-6. Minor per-task waste`
- `docs/v3/39-ROADMAP.md#3.3 Half-Implemented Features`

How to verify: Read WarmPool insertion code. / Locate the scaffolding named in the roadmap row and confirm no runtime caller.

Merged 2 mined candidates: m4-041, m5-074.
