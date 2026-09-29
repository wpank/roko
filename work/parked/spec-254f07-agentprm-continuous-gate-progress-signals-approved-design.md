+++
id = "spec-254f07"
kind = "spec"
title = "AgentPRM continuous gate progress signals (approved design upgrade)"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-gate"]
created = 2026-09-15
updated = 2026-09-28
source = "docs/v3/39-ROADMAP.md#1.4 AgentPRM Continuous Gate Progress Signals"
discovered_from = "audit:docs/v3/39-ROADMAP.md#1.4 AgentPRM Continuous Gate Progress Signals"
anchors = ["crates/roko-cli/src/runner/gate_dispatch.rs", "roko_core::Verdict"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Approved: gates emit continuous progress signals (TD/GAE estimation) alongside binary verdicts to enable partial-success replanning and cheaper verification; target roko-gate pipeline and gate_dispatch.rs.

Imported without verification from:
- `docs/v3/39-ROADMAP.md#1.4 AgentPRM Continuous Gate Progress Signals`

How to verify: Check Verdict for a progress/score field used by replan.
