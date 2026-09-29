+++
id = "gap-836ae9"
kind = "gap"
title = "Live and replayed tool streams lack proven deterministic store/order parity"
status = "open"
triage = "verified"
severity = "p2"
goal = "visibility"
subsystem = ["roko-core/transcript_store"]
created = 2026-09-14
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "tmp/archive/tool-audit-2026-09-21/10-FINDINGS-REGISTER.md#register"
discovered_from = "audit:tmp/archive/tool-audit-2026-09-21/10-FINDINGS-REGISTER.md#register"
anchors = ["crates/roko-core/src/transcript_store.rs::replay", "crates/roko-core/src/tool/transcript.rs"]
links = { depends_on = [], blocks = [], related = ["gap-9c8ac0", "gap-633184"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rq --include='*.rs' 'fn live_and_replayed_tool_streams_are_identical' crates/ && cargo test --workspace live_and_replayed_tool_streams_are_identical"
+++
Monotonic sequence/replay contract exists in TranscriptStore (On main), but release gate 'Live and replay output are semantically identical' is unchecked: store contract exists, end-to-end not verified.

Imported without verification from:
- `tmp/archive/tool-audit-2026-09-21/10-FINDINGS-REGISTER.md#register`
- `tmp/archive/tool-audit-2026-09-21/11-IMPLEMENTATION-CHECKLIST.md#release-gate`
- `tmp/archive/tool-audit-2026-09-21/00-INDEX.md#status-note-2026-09-14`

How to verify: Look for an end-to-end test that replays a persisted transcript and asserts semantic equality with the live projection.

Verified 2026-09-28: the sequence/replay contract has unit tests (transcript_store.rs:818 replay_from_sequence, :872 auto_assigns_monotonic_sequence, :887 duplicate_sequence_is_rejected; tool/transcript.rs:955 golden_fixture_replay_ordering_by_sequence). No end-to-end test replays a persisted transcript and asserts semantic equality with the live projection; roko-runtime/src/event_bus.rs:475 only compares a cursor value. Severity p2 (missing proof).

Re-checked 2026-09-29 at d9e79e9d8: unchanged. The live tool steps added in c41e78c7a came with no live-vs-replay parity test. crates/roko-fs/src/classified_writer.rs:340 (live_replay_equivalence_through_persistence) is a store-level test only, and the writer it exercises has no production caller.
