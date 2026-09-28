+++
id = "spec-6a4c20"
kind = "spec"
title = "SystemPromptBuilder: dynamic layer ordering, prompt compression, dominance affect guidance"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-compose/system-prompt"]
created = 2026-09-15
updated = 2026-09-28
source = "docs/v3/depth/06-composition/system-prompt-builder-9-layer.md:357"
discovered_from = "audit:docs/v3/depth/06-composition/system-prompt-builder-9-layer.md:357"
anchors = ["crates/roko-compose/src/system_prompt_builder.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
9-layer builder ships; dynamic layer ordering and prompt-compression integration are Designed, dominance affect guidance is 'Not yet' (only arousal/pleasure). PEEK 10th layer is tracked separately.

Imported without verification from:
- `docs/v3/depth/06-composition/system-prompt-builder-9-layer.md:357`

How to verify: grep builder for dominance handling and fixed layer order.
