+++
id = "spec-5c8b9c"
kind = "spec"
title = "Canonical Layer-Safe Runtime Event Schema"
status = "done"
triage = "verified"
severity = "p0"
subsystem = ["workspace"]
created = 2026-09-07
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/backlog/archive/208-unified-event-schema.md#208 — Canonical Layer-Safe Runtime Event Schema"
discovered_from = "audit:tmp/backlog/archive/208-unified-event-schema.md#208 — Canonical Layer-Safe Runtime Event Schema"
anchors = ["crates/roko-core/src/runtime_event.rs::RuntimeEventEnvelope", "crates/roko-core/src/runtime_event.rs::RuntimeEventPublisher", "crates/roko-core/src/runtime_event.rs::RuntimeEventProjector"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'pub fn new_v2' crates/roko-core/src/runtime_event.rs && grep -q 'trait RuntimeEventProjector' crates/roko-core/src/runtime_event.rs && grep -q 'RuntimeEventEnvelopeV1' crates/roko-core/src/runtime_event.rs"

[closed]
at = 2026-09-28
commit = "91b4745f8"
evidence = "Implemented in crates/roko-core/src/runtime_event.rs, which is clean in the working tree: schema_version 2 on all new serialization (:186, :225), RuntimeEventEnvelope::new_v2 (:198), a private RuntimeEventEnvelopeV1-to-v2 conversion (:242-265), the RuntimeEventPublisher and RuntimeEventProjector traits, and the frozen variants (WaveStarted, TaskRetrying, GateRungOutput, ApprovalRequested, FeedbackSinkSettled, PredictionPublished, SequenceGap, Extension, and the rest). The module docs cover the SequenceGap and replay rules. This matches the backlog's own 'Implemented (2026-09-04)'. cargo tests were not run in this triage."
+++
Canonical Layer-Safe Runtime Event Schema

Imported without verification from:
- `tmp/backlog/archive/208-unified-event-schema.md#208 — Canonical Layer-Safe Runtime Event Schema`
- `tmp/archive/MASTER-ACTION-PLAN-2026-09-23.md#Phase A: Foundation (Wave 0-1, ~3-5 days #208`

How to verify: Check: Implement the frozen v2 envelope and v1 compatibility conversion above in the existing module.; Add exactly the listed canonical variants without renaming or removing v1 variants.; Classify terminal/usage/receipt/gate/control events as… [evidence: own status: Implemented (2026-09-04); 00-STATUS-SUMMARY 3. Open / P2 -- Medium (Open): M | engine DAG | Done (Wave 0; 2026-09-04); (newer evidence overrides own status…]

Verified 2026-09-28: implemented (runtime_event.rs). See [closed].
