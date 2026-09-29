+++
id = "gap-3e037a"
kind = "gap"
title = "Gate pipeline silently skips unconfigured rungs (generated tests stub, LLM judge unimplemented, unknown gates)"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-gate"]
created = 2026-09-21
updated = 2026-09-28
source = "crates/roko-gate/src/rung_dispatch.rs:397"
discovered_from = "audit:crates/roko-gate/src/rung_dispatch.rs:397"
anchors = ["crates/roko-gate/src/rung_dispatch.rs::run_generated_test_gate", "crates/roko-gate/src/gate_service.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
generated_test:cargo rung returns a stub verdict unless generated_test_artifacts is set; the LLM judge gate is skipped as 'not yet implemented'; custom/shell gates without commands and unknown gate names become skipped 'not wired' verdicts rather than config errors.

Imported without verification from:
- `crates/roko-gate/src/rung_dispatch.rs:397`
- `crates/roko-gate/src/gate_service.rs:255`
- `crates/roko-gate/src/gate_service.rs:267`
- `crates/roko-gate/src/gate_service.rs:331`
- `tmp/nous-research/AUDIT-SUMMARY-2026-09-04.md#8. Evals Audit`

How to verify: Check where generated_test_artifacts is populated in runner/graph gate dispatch; note 87 untracked generated-tests/*.rs at repo root.
