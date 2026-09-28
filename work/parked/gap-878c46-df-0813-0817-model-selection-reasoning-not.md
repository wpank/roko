+++
id = "gap-878c46"
kind = "gap"
title = "DF-0813/0817: Model selection reasoning not surfaced to operators"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-learn/cascade-router"]
created = 2026-08-17
updated = 2026-09-28
source = "tmp/archive/dogfood-2026-08-13/DOGFOOD-DEBRIEF.md#6. MEDIUM: Cascade router ignores configured model"
discovered_from = "audit:tmp/archive/dogfood-2026-08-13/DOGFOOD-DEBRIEF.md#6. MEDIUM: Cascade router ignores configured model"
anchors = ["model_routing.rs", "ModelChoiceSource"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Cascade router picked kimi/sonar/gemma for implementer tasks over the configured default with no visible explanation; debriefs ask to show why a model was chosen (relates to UX34 override learning).

Imported without verification from:
- `tmp/archive/dogfood-2026-08-13/DOGFOOD-DEBRIEF.md#6. MEDIUM: Cascade router ignores configured model`
- `tmp/archive/dogfood-2026-08-17-retest/DOGFOOD-DEBRIEF.md#Remaining Observations`
- `tmp/archive/dogfood-2026-08-22/DOGFOOD-DEBRIEF.md#UX4: Cascade router picks unconfigured models`

How to verify: Check dispatch events/TUI for a selection-reason field.
