+++
id = "gap-2db980"
kind = "gap"
title = "Daimon phase-2 affect stubs not wired into dispatch (contrarian retrieval, fatigue, somatic markers)"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-daimon"]
created = 2026-09-21
updated = 2026-09-28
source = "crates/roko-daimon/src/phase2_stubs.rs:1"
discovered_from = "audit:crates/roko-daimon/src/phase2_stubs.rs:1"
anchors = ["crates/roko-daimon/src/phase2_stubs.rs::BehavioralStateTracker", "crates/roko-daimon/src/phase2_stubs.rs::AffectBehaviorModulation"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
phase2_stubs.rs defines AffectOctant, AffectBehaviorModulation, hysteresis BehavioralStateTracker, TierBias, SomaticMarkerFiredEvent etc. 'tested but not yet wired into the runtime dispatch path'; affect docs list VCG per-bidder modulation and somatic landscape as partial.

Imported without verification from:
- `crates/roko-daimon/src/phase2_stubs.rs:1`
- `docs/v3/depth/11-affect/integration-points.md:194`
- `docs/v3/depth/11-affect/8-dimensional-strategy-space.md:450`

How to verify: grep for phase2_stubs symbol uses outside roko-daimon tests.
