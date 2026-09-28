+++
id = "spec-5fbfa4"
kind = "spec"
title = "Collective-intelligence metrics: CohortWeightsLearner and WisdomGate (target design)"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-learn/c-factor"]
created = 2026-09-15
updated = 2026-09-28
source = "docs/v3/depth/16-coordination/12-collective-intelligence-metrics.md:350"
discovered_from = "audit:docs/v3/depth/16-coordination/12-collective-intelligence-metrics.md:350"
anchors = ["crates/roko-learn/src/quality_judge.rs::CFactorSummary"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
c-factor infrastructure is wired per dispatch, but CohortWeightsLearner and WisdomGate are Specified/target design and the dashboard tile is partial.

Imported without verification from:
- `docs/v3/depth/16-coordination/12-collective-intelligence-metrics.md:350`

How to verify: grep for CohortWeightsLearner/WisdomGate.
