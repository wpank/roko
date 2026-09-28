+++
id = "gap-e256dd"
kind = "gap"
title = "[provider F077] Cognitive energy costs not tied to actual provider API costs"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-daimon"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F077"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F077"
anchors = ["crates/roko-daimon/src/lib.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Energy depletion costs are fixed constants (T0: 0.01, T1: 0.05, T2: 0.15) regardless of actual API spend. A $0.001 haiku call and a $5.00 opus call in the same tier incur the same energy charge.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F077`
- `tmp/archive/provider-audit/16-daimon-affect.md`

How to verify: E23 claims energy accounting; check linkage to provider costs. Confirm in crates/roko-daimon/src/lib.rs whether still true: Cognitive energy costs not tied to actual provider API costs
