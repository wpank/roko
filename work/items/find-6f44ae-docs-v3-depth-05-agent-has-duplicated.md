+++
id = "find-6f44ae"
kind = "finding"
title = "docs/v3/depth/05-agent has duplicated numbered + unnumbered pages"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["docs/v3"]
created = 2026-09-15
updated = 2026-09-28
source = "docs/v3/depth/05-agent/"
discovered_from = "audit:docs/v3/depth/05-agent/"
anchors = ["docs/v3/depth/05-agent/"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
At least 8 page pairs are duplicated (e.g. 10-temperament-profiling.md vs temperament-profiling.md, 16-/domain-profiles, 17-/dispatcher-architecture, 19-/cognitive-autonomy-e23, 18-safety-policies vs safety-layer). Risk of divergent edits and double-counted banners.

Imported without verification from:
- `docs/v3/depth/05-agent/`

How to verify: diff each numbered/unnumbered pair; check VitePress sidebar for which is linked.
