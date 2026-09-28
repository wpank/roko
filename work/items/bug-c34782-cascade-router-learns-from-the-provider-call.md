+++
id = "bug-c34782"
kind = "bug"
title = "Cascade router learns from the provider call's success flag before gates run"
status = "open"
triage = "verified"
severity = "p2"
subsystem = ["roko-cli/graph-dispatch", "roko-learn/cascade-router"]
created = 2026-09-28
updated = 2026-09-28
last_verified = 2026-09-28
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-cli/src/graph_task_dispatch.rs::emit_feedback", "crates/roko-cli/src/runtime_feedback/routing.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = 'cargo test -p roko-cli runtime_feedback'
+++

Both Graph dispatch paths call `emit_feedback(.., dispatch.result.success, ..)` straight after the provider call (`graph_task_dispatch.rs:1963` non-streaming, `:3318` streaming), before any verify or gate step.
The routing sink turns that flag into router observations, and `costs.jsonl` records the same flag as `success`, so a model that answers confidently but fails compile/test gates is rewarded.
Fix: settle routing feedback once per attempt after gates (including retries) with an explicit verdict (passed / failed / skipped / inconclusive) and have the router learn only from that settlement.
