+++
id = "bug-c59522"
kind = "bug"
title = "ACP stability hardening residue (P1 silent-failure and race items; backlog #17)"
status = "done"
triage = "verified"
severity = "p1"
subsystem = ["roko-acp"]
created = 2026-09-15
updated = 2026-09-28
last_verified = 2026-09-28
source = "docs/v3/39-ROADMAP.md#2.1 P0 -- Critical"
discovered_from = "audit:docs/v3/39-ROADMAP.md#2.1 P0 -- Critical"
anchors = ["crates/roko-acp/src/runner.rs:1555", "crates/roko-acp/src/acp_adapter.rs:191", "crates/roko-acp/src/session.rs:676"]
links = { depends_on = [], blocks = [], related = ["bug-b2a9de", "bug-f0f108"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-acp"

[[verify]]
command = "! grep -rn 'treating as approved' crates/roko-acp/src"

[closed]
at = 2026-09-28
commit = "91b4745f8"
evidence = "Fixed at HEAD 91b4745f8: every itemized P1 silent-failure/race item in tmp/backlog/archive/17-acp-stability-hardening.md is addressed. Reviewer errors no longer auto-approve (crates/roko-acp/src/runner.rs:1554-1555 and :1583-1584 set all_approved = false, and the treating-as-approved path is gone). AcpAdapter logs dropped events (acp_adapter.rs:189-194). The cognitive event channel holds 256 (bridge_events/mod.rs:551). Session busy uses an atomic idle-to-busy transition (session.rs:676). No production panics remain (see bug-b2a9de). The sustained-load residue (WorkflowCompleted can still be dropped; permission waits poll every 25ms) is tracked in bug-f0f108."
+++
Backlog #17 (P0): 7 ACP crash-path panics fixed, but broader P1 silent-failure and race items remain; the provider audit counted 7 P0 panics + 12 race conditions on the ACP path (F005).

Imported without verification from:
- `docs/v3/39-ROADMAP.md#2.1 P0 -- Critical`
- `tmp/nous-research/AUDIT-SUMMARY-2026-09-04.md#2. Provider Audit`

How to verify: Review tmp/backlog #17 spec open items vs roko-acp code.

Verified 2026-09-28: fixed; see `[closed].evidence`.
