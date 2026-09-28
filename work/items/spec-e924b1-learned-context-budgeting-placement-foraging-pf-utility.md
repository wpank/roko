+++
id = "spec-e924b1"
kind = "spec"
title = "Learned context budgeting/placement/foraging (pf_utility=0, attention curves, budget prediction, DSPy)"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-compose/budget"]
created = 2026-09-15
updated = 2026-09-28
source = "docs/v3/depth/06-composition/predictive-foraging-mvt.md:260"
discovered_from = "audit:docs/v3/depth/06-composition/predictive-foraging-mvt.md:260"
anchors = ["roko_compose::MultiPatchForager", "roko-compose pf_utility"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Designed/not-yet items: predictive-foraging pf_utility defaults to 0, no per-category calibration or social foraging; position-attention model, density-driven placement and per-model attention curves; learned budget prediction, leave-one-out influence, density scoring; DSPy role budgets.

Imported without verification from:
- `docs/v3/depth/06-composition/predictive-foraging-mvt.md:260`
- `docs/v3/depth/06-composition/lost-in-the-middle-u-shape.md:249`
- `docs/v3/depth/06-composition/token-budget-management.md:252`
- `docs/v3/depth/06-composition/role-templates-11.md:391`

How to verify: grep pf_utility and placement code; confirm defaults.
