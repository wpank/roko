+++
id = "gap-795a58"
kind = "gap"
title = "Trigger Composition with Boolean Logic"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-core"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/backlog/archive/421-trigger-composition-boolean-logic.md#421 — Trigger Composition with Boolean Logic"
discovered_from = "audit:tmp/backlog/archive/421-trigger-composition-boolean-logic.md#421 — Trigger Composition with Boolean Logic"
anchors = ["crates/roko-core/src/trigger.rs", "crates/roko-serve/src/trigger_runtime.rs", "trigger_runtime.rs", "TriggerKind", "TriggerCoordinator", "ActiveBinding", "start_source()", "apply_concurrency()"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
enables multi-source trigger workflows without manual orchestration. The trigger system supports seven source kinds (`TriggerKind` enum at line 832 of `crates/roko-core/src/trigger.rs`): `Cron`, `Webhook`, `FileWatch`, `Bus`, `ChainEvent`, `Manual`, and `SignalPattern`. Each `TriggerBinding` has…

Imported without verification from:
- `tmp/backlog/archive/421-trigger-composition-boolean-logic.md#421 — Trigger Composition with Boolean Logic`

How to verify: Check: `TriggerKind::Composite` with `CompositeOp::{And, Or, Sequence, Debounce}` is; `Or` composites fire the parent when any child source activates.; `And` composites fire the parent only when all children activate within `window_ms`. [evidence: 00-INDEX (2026-09-21) listed active: 2026-09-21 Channel/Webhook/Reactive Audit (#409-#4]
