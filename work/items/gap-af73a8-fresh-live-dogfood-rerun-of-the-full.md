+++
id = "gap-af73a8"
kind = "gap"
title = "Fresh live dogfood rerun of the full self-hosting workflow (blocked by SnapshotRebased arm gap)"
status = "open"
triage = "verified"
severity = "p1"
subsystem = ["self-hosting"]
created = 2026-09-15
updated = 2026-09-28
last_verified = 2026-09-28
source = "docs/v3/39-ROADMAP.md#7.5 Fresh Dogfood Proof"
discovered_from = "audit:docs/v3/39-ROADMAP.md#7.5 Fresh Dogfood Proof"
anchors = ["crates/roko-cli/src/runner/output_sink.rs:1493"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Full live agent-dispatch rerun of prd→plan→run has not been performed since the 2026-08-13 fixes; roadmap says it was blocked by a dirty-tree build failure from the SnapshotRebased arm gap in output_sink.rs.

Imported without verification from:
- `docs/v3/39-ROADMAP.md#7.5 Fresh Dogfood Proof`
- `tmp/refactoring-audit/P1-06-DOGFOOD-PROOF.md`

Warning: every file this item cites is gone (`crates/roko-cli/src/**/output_sink.rs`) — likely obsolete or moved.

How to verify: cargo check -p roko-cli; grep SnapshotRebased match arms; then run the CLAUDE.md self-hosting workflow.

Verified 2026-09-28: the blocker is gone, but the full rerun is still unrecorded. output_sink.rs:1493 now handles DashboardEvent::SnapshotRebased, so the build failure cited in the title no longer applies. Some live evidence exists: tmp/dogfood/2026-09-20-final-session.md:170 records a fresh run in which 4 of 5 demo plans worked end to end, and the 2026-09-25 portal-programme run log records live `plan run` executions. None of these records the full prd idea -> draft -> plan -> run -> status workflow. What remains is to run that complete pass and record it.
