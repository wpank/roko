+++
id = "gap-ee03d6"
kind = "gap"
title = "[cybernetic ML-13] Section effectiveness recommendations never hard-drop negative sections"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-compose"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/cybernetic-audit/28-feedback-loops.md#ML-13: Section Effectiveness Auto-Apply (Hard Drop of Negative Sections)"
discovered_from = "audit:tmp/archive/cybernetic-audit/28-feedback-loops.md#ML-13: Section Effectiveness Auto-Apply (Hard Drop of Negative Sections)"
anchors = ["PriorityChange::Decrease", "section_effect.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Section recommendations are advisory; PriorityChange::Decrease shifts priority but never drops. After >50 trials with lift < -0.05 sections should be removed from composition (P0-01 bandit only re-ranks).

Imported without verification from:
- `tmp/archive/cybernetic-audit/28-feedback-loops.md#ML-13: Section Effectiveness Auto-Apply (Hard Drop of Negative Sections)`

How to verify: Check prompt assembly for a drop path driven by section outcome evidence.
