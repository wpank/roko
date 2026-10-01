+++
id = "bug-5cd5ca"
kind = "bug"
title = "[refactor P1-07] ACP/serve prompt-experiment parity: ACP double-counts, serve lacks receipt protocol"
status = "superseded"
triage = "verified"
severity = "p1"
subsystem = ["roko-acp/experiments"]
created = 2026-09-15
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/archive/refactoring-audit-2026-09-21/IMPLEMENTATION-CHECKLIST.md#p1-high-blocks-self-hosting-significant-tech-debt"
discovered_from = "audit:tmp/archive/refactoring-audit-2026-09-21/IMPLEMENTATION-CHECKLIST.md#p1-high-blocks-self-hosting-significant-tech-debt"
anchors = ["crates/roko-acp/src/bridge_events/experiments.rs::record_acp_experiment_outcome", "crates/roko-serve/src/dispatch.rs::load_template_experiment_variant", "crates/roko-learn/src/prompt_experiment.rs::ExperimentStore::settle_attempt"]
links = { depends_on = [], blocks = [], related = ["gap-bc18c1"], supersedes = [], duplicate_of = "gap-6a673d" }

[closed]
at = 2026-09-28
evidence = "duplicate of gap-6a673d, which covers the serve residual (see also gap-bc18c1). The ACP half is fixed at HEAD 91b4745f8: crates/roko-acp/src/bridge_events/experiments.rs:82 prepares a durable receipt via prepare_attempt_assignments, and record_acp_experiment_outcome (:269-336) makes settle_attempt the sole stats writer whenever a receipt exists, recording directly only when no receipt bucket exists, so trials are not double-counted. Still true for serve: crates/roko-serve/src/dispatch.rs:548-593 and :2331 only load and inject a template variant, and crates/roko-serve/src has no prepare_attempt_assignments or settle_attempt call."
+++
Double-counting bug confirmed in ACP (direct mutation vs durable bucket); serve has no receipt protocol at all. Three-phase fix designed (~3.5d), not implemented. Related provider F050; CLAUDE.md still lists ACP/serve receipt parity as partial.

Imported without verification from:
- `tmp/archive/refactoring-audit-2026-09-21/IMPLEMENTATION-CHECKLIST.md#p1-high-blocks-self-hosting-significant-tech-debt`
- `tmp/archive/refactoring-audit-2026-09-21/P1-07-EXPERIMENT-PARITY.md`
- `tmp/archive/evals-audit/12-gaps-and-residuals.md#3.3 ACP/Serve Experiment Parity`
- `tmp/archive/evals-audit/09-prompt-experiments.md`
- `tmp/archive/status-quo-audit-2026-09-21/09-completion-tracking.md#P2 items: 8/10 done`

A source claims this was fixed; confirm against current code before closing.

How to verify: Check ACP experiment outcome recording for dual writes and whether serve dispatch uses the runner prelaunch-receipt/settlement protocol. / Check roko-serve experiment injection for prepare_attempt_assignments/settle_attempt calls.

Merged 2 mined candidates: m2-173, m3-054.

Verified 2026-09-28: superseded as a duplicate; see `[closed].evidence`. The cited prompt_experiment.rs is at `crates/roko-learn/src/prompt_experiment.rs`.
