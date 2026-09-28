+++
id = "gap-5f4852"
kind = "gap"
title = "Secret-canary persistence test never run for scrubbers/persistent sinks"
status = "open"
triage = "verified"
severity = "p1"
goal = "release"
subsystem = ["roko-fs/observability"]
created = 2026-09-14
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/archive/tool-audit-2026-09-21/10-FINDINGS-REGISTER.md#register"
discovered_from = "audit:tmp/archive/tool-audit-2026-09-21/10-FINDINGS-REGISTER.md#register"
anchors = ["crates/roko-fs/src/observability.rs::RunScrubber", "crates/roko-cli/tests/secret_canary.rs::canary_absent_from_all_output_sinks", "crates/roko-core/src/transcript_store.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-cli --test secret_canary"
+++
RunScrubber with literal secrets exists (On main; related T006 ClassifiedRecord redaction), but release gate 'Secret canaries are absent from normal output and every persistent sink' is unchecked (not yet tested).

Imported without verification from:
- `tmp/archive/tool-audit-2026-09-21/10-FINDINGS-REGISTER.md#register`
- `tmp/archive/tool-audit-2026-09-21/11-IMPLEMENTATION-CHECKLIST.md#release-gate`
- `tmp/archive/tool-audit-2026-09-21/00-INDEX.md#status-note-2026-09-14`

How to verify: Check that configured literal secrets reach RunScrubber in production wiring and that a canary test covers episodes/traces/audit/transcripts.

Verified 2026-09-28: partly done. crates/roko-cli/tests/secret_canary.rs (added in 244f564e1) plants env-file canaries and checks LogScrubber redaction, episodes.jsonl, efficiency.jsonl, gate-failures.jsonl and share transcripts; canary_absent_from_all_output_sinks covers the three JSONL sinks. Still true: RunScrubber (crates/roko-fs/src/observability.rs:186) has no production call site; it is only re-exported at roko-fs/src/lib.rs:74, so configured literal secrets never reach it. No canary covers traces, audit logs or the transcript store. The canary suite was not run in this triage.
