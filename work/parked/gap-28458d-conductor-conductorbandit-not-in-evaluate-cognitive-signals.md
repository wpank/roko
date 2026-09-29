+++
id = "gap-28458d"
kind = "gap"
title = "Conductor: ConductorBandit not in evaluate(), cognitive signals subset, c-factor not fed back"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-conductor"]
created = 2026-09-15
updated = 2026-09-28
source = "docs/v3/30-CONDUCTOR.md:7"
discovered_from = "audit:docs/v3/30-CONDUCTOR.md:7"
anchors = ["roko_conductor::Conductor::evaluate", "roko_learn::ConductorBandit", "CFactorSummary"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
ConductorBandit learned policy exists in roko-learn but isn't wired into the live evaluate() path; ConductorDecision (Continue/Restart/Fail) covers only a subset of cognitive signals; CFactorSummary is computed but not fed back to the Conductor.

Imported without verification from:
- `docs/v3/30-CONDUCTOR.md:7`
- `docs/v3/depth/30-conductor/cognitive-signals.md:181`
- `docs/v3/depth/16-coordination/13-conductor-integration.md:340`

How to verify: grep ConductorBandit callers; check evaluate() inputs.
