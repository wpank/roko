+++
id = "gap-c57f32"
kind = "gap"
title = "[cybernetic ML-05] Agent-to-agent affect contagion not propagated via Bus"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-daimon"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/cybernetic-audit/28-feedback-loops.md#ML-05: Agent-to-Agent Affect Contagion via Bus"
discovered_from = "audit:tmp/archive/cybernetic-audit/28-feedback-loops.md#ML-05: Agent-to-Agent Affect Contagion via Bus"
anchors = ["ContagionEvent", "ContagionTrigger", "contagion_susceptibility"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
contagion(), contagion_susceptibility(), ContagionEvent, ContagionTrigger exist but no runtime publishes/subscribes PAD states over the Bus for fleet-wide mood coordination (P1-15 only wired contagion into Runner-v2 dispatch).

Imported without verification from:
- `tmp/archive/cybernetic-audit/28-feedback-loops.md#ML-05: Agent-to-Agent Affect Contagion via Bus`

How to verify: grep ContagionEvent publish/subscribe sites on the Bus.
