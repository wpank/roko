+++
id = "gap-c8f725"
kind = "gap"
title = "Wire SelfHealingState into Conductor"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-conductor"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/backlog/archive/388-self-healing-state-conductor-wiring.md#388 — Wire SelfHealingState into Conductor"
discovered_from = "audit:tmp/backlog/archive/388-self-healing-state-conductor-wiring.md#388 — Wire SelfHealingState into Conductor"
anchors = ["crates/roko-conductor/src/"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
module exists but Conductor struct has no self_healing field. Dev-audit found that SelfHealingState module exists with complete implementation but the Conductor struct has no `self_healing` field. The self-healing logic (restart cooldowns, automatic recovery) is never activated.

Imported without verification from:
- `tmp/backlog/archive/388-self-healing-state-conductor-wiring.md#388 — Wire SelfHealingState into Conductor`

How to verify: Check whether the gap described in tmp/backlog/archive/388-self-healing-state-conductor-wiring.md still exists at the anchored paths. [evidence: 00-INDEX (2026-09-21) listed active: 2026-09-21 Audit Sweep Items (#376-#395)]
