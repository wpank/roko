+++
id = "spec-6d9ded"
kind = "spec"
title = "PEEK orientation cache as 10th SystemPromptBuilder layer (approved design upgrade)"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-compose/system-prompt"]
created = 2026-09-15
updated = 2026-09-28
source = "docs/v3/39-ROADMAP.md#1.3 PEEK Orientation Cache (10th SystemPromptBuilder Layer)"
discovered_from = "audit:docs/v3/39-ROADMAP.md#1.3 PEEK Orientation Cache (10th SystemPromptBuilder Layer)"
anchors = ["crates/roko-compose/src/system_prompt_builder.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Approved: constant-token orientation cache layer maintained by Distiller, Cartographer and Evictor modules to keep agents oriented over long sessions; builder doc lists '10th PEEK layer' as target design.

Imported without verification from:
- `docs/v3/39-ROADMAP.md#1.3 PEEK Orientation Cache (10th SystemPromptBuilder Layer)`
- `docs/v3/depth/06-composition/system-prompt-builder-9-layer.md:357`
- `docs/v3/06-COMPOSITION.md:9`

How to verify: Confirm builder has 9 layers only.
