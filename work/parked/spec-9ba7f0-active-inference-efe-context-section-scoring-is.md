+++
id = "spec-9ba7f0"
kind = "spec"
title = "Active-inference (EFE) context section scoring is designed, not wired"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-compose/scoring"]
created = 2026-09-15
updated = 2026-09-28
source = "docs/v3/depth/06-composition/active-inference-context-selection.md:261"
discovered_from = "audit:docs/v3/depth/06-composition/active-inference-context-selection.md:261"
anchors = ["roko_compose::SectionScorer", "roko_compose::PromptComposer"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Composition docs mark the EFE section scorer, episode track-record input, HDC belief-change term, softmax selection and cold-start fallback as Designed/'Designed, not wired'; PromptComposer uses the static SectionScorer.

Imported without verification from:
- `docs/v3/depth/06-composition/active-inference-context-selection.md:261`
- `docs/v3/depth/06-composition/composer-trait.md:289`
- `docs/v3/depth/06-composition/prompt-composer.md:326`
- `docs/v3/depth/06-composition/5-stage-assembly-pipeline.md:276`

How to verify: grep roko-compose for EFE/expected free energy scorer use in assembly.
