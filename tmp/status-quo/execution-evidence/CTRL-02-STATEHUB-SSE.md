# CTRL-02 StateHub/SSE ordering precursor implementation evidence

Assignment:
- Plan: Wave 0 `CTRL-02`; bounded attribution and correction of the StateHub/SSE portion of the July 14 precursor, adjacent to `SH03-T06` in `tmp/status-quo/self-heal/plans/SH03-persistence-integrity/tasks.toml`.
- Base SHA: `9c015378a37f75c81bacee946e1174ba9478eba1`.
- Historical precursor: `3041d095d4daebed2c9e05c63eacb18e668e37e3` relative to parent `1649c18b2c3d2b3602bfe17398b0e1454a19c5ef`.
- Branch/worktree: `agent/CTRL-02-statehub-sse` at `/Users/will/dev/nunchi/roko/agent-worktrees/status-quo-20260714T073140Z/workers/CTRL-02-statehub-sse`.
- Integration branch: `status-quo/integration-status-quo-20260714T073140Z`.
- Reserved write scope: `crates/roko-runtime/src/state_hub.rs`, `crates/roko-cli/src/tui/app.rs`, `crates/roko-serve/src/lib.rs`, `crates/roko-serve/src/routes/sse.rs`, and this evidence record.
- Dependencies and their integration commits: the assigned base contains the historical precursor as an ancestor. Full `SH03-T06` remains gated on `SH03-T03`, which is not integrated.

Requirement:
- Original defect: before `3041d095d`, StateHub broadcast an event before committing it to the materialized snapshot, batch consumers could observe intermediate state, and TUI/serve performed lossy `current_snapshot`/modify/`apply_snapshot` round trips that could overwrite concurrent runner publications. SSE treated `Last-Event-ID` as inclusive and silently continued after live broadcast lag.
- Inherited correction: `3041d095d` serialized snapshot commit, best-effort event-log append, sequence assignment, and broadcast; added atomic `update_snapshot`; migrated the three TUI mutations and serve cascade-router bootstrap to it; made reconnect resume after the acknowledged ID; and emitted a snapshot-bearing `gap` frame on live lag.
- Residual defects found during attribution: SSE captured replay before installing its live receiver, so a publish between those operations was absent from both sources. It also truncated retained replay to 256 events without a gap, silently losing the remaining suffix. A live lag snapshot and its event cursor were read separately, and queued pre-snapshot events could be delivered after the snapshot resync.
- Expected behavior: subscriber event N observes snapshot state including N; concurrent publishers and selected-field snapshot mutations cannot overwrite each other; replay and live delivery have an atomic boundary; evicted or oversized reconnect suffixes produce an explicit snapshot resync; and a post-lag client never receives an event older than the supplied snapshot cursor.
- Acceptance for this bounded candidate: focused runtime StateHub tests, focused SSE tests, the complete `roko-serve` test suite, relevant connected-TUI tests, affected-package all-target check, workspace formatting, diff hygiene, exact-scope review, and independent review of the immutable candidate.
- Explicit non-goals: persistent sequence numbers across process restart; fsync/error propagation for the best-effort StateHub JSONL log; full event-bus durability; broadcast of snapshot-only mutations; recovery for WebSocket, projection, workflow, and other independent consumers; the duplicate `roko-core` StateHub; secret scrubbing of streaming producers; all `issue 37` findings; task/status/manifest changes; or completion of `SH03-T06`.

Reproduction:
- Historical attribution command: `git diff 1649c18b2c3d2b3602bfe17398b0e1454a19c5ef..3041d095d4daebed2c9e05c63eacb18e668e37e3 -- crates/roko-runtime/src/state_hub.rs crates/roko-cli/src/tui/app.rs crates/roko-serve/src/lib.rs crates/roko-serve/src/routes/sse.rs`.
- Historical result: the four production files contain the inherited commit-before-broadcast lock, atomic mutation call sites, cursor increment, and live lag snapshot frame. `git diff 3041d095d..9c015378a -- <the same paths>` was empty, proving the assigned base had not drifted from those precursor bytes.
- Residual reconnect reproduction: changed-line inspection of base `sse_handler` showed `state_hub.replay_from(replay_from)` completed before `state_hub.subscribe_events()`. A publication in that interval was newer than the captured replay but older than the receiver, so it was never delivered. The new `replay_live_handoff_has_no_missing_boundary_event` regression exercises the required atomic boundary.
- Residual truncation reproduction: the base handler applied `.take(256)` to replay and then chained directly to a receiver subscribed after replay capture. With 257 retained events from cursor zero, sequence 256 was neither replayed nor represented by a gap. `oversized_replay_requires_snapshot_instead_of_truncation` covers this boundary.

Implementation:
- `StateHub::subscribe_events_from` now holds the existing publication mutex while it installs the live receiver, captures retained replay, clones the snapshot, and records the next sequence. The returned replay contains events below `cursor.next_seq`; live delivery begins at that cursor, closing the handoff race without modifying the generic event bus.
- `StateHub::cursor_snapshot` atomically couples a materialized snapshot with its next sequence. Immutable log replay now also uses the publication mutex so this cursor cannot observe a concurrent snapshot replay half-way through mutation.
- Additive `StateHubCursorSnapshot` and `StateHubSubscription` types make the boundary explicit without changing existing `publish`, `subscribe_events`, or `replay_from` callers.
- SSE now resumes exactly after `Last-Event-ID`. A contiguous retained suffix of at most 256 events is replayed. An evicted, non-contiguous, or oversized suffix is replaced by one `event: gap` frame containing `missed_events`, `last_materialized_seq`, and the atomic snapshot.
- Live lag captures the same atomic snapshot/cursor and raises a per-stream live floor. Receiver entries older than that floor are discarded, preventing queued pre-resync events from being applied after the snapshot. `missed_events` counts the whole suffix replaced by the snapshot, including retained queued entries, rather than only Tokio's initially evicted count.
- The inherited TUI gate-trend/topology and serve cascade-router mutations remain `update_snapshot` call sites. A connected TUI regression proves the topology path preserves runner-published plan state.
- Compatibility/resource behavior: existing SSE ordinary event shape and IDs are unchanged. The new public runtime API is additive. Replay remains bounded by the StateHub ring; a client receives at most 256 replay event frames or one snapshot frame before live delivery. Publication remains synchronous and the event log remains explicitly best-effort.

Verification:
- Command: `CARGO_TARGET_DIR=/private/tmp/roko-ctrl02-statehub-sse-target CARGO_INCREMENTAL=0 cargo test -p roko-runtime state_hub -- --test-threads=1`.
- Exit/result: exit 0; 11 StateHub tests passed, including event-before-snapshot observation, concurrent publish/update preservation, and a gap-free replay/live boundary. Three unrelated pre-existing warnings in heartbeat test code were emitted.
- Command: `CARGO_TARGET_DIR=/private/tmp/roko-ctrl02-statehub-sse-target CARGO_INCREMENTAL=0 cargo test -p roko-serve routes::sse -- --test-threads=1`.
- Exit/result: exit 0 after the final source correction; 4 SSE tests passed, covering cursor increment, exact retained suffix, evicted cursor snapshot, and the 257-event replay boundary.
- Command: `CARGO_TARGET_DIR=/private/tmp/roko-ctrl02-statehub-sse-target CARGO_INCREMENTAL=0 cargo test -p roko-serve -- --test-threads=1`.
- Exit/result: exit 0; 470 tests passed and none failed: 383 library, 29 API integration, 22 job lifecycle, 20 job-runner integration, 2 server lifecycle, 2 PRD publish, 2 sanitize, 5 security-bind, and 5 workspace-persistence tests. Doc tests also passed with zero tests.
- Command: `CARGO_TARGET_DIR=/private/tmp/roko-ctrl02-statehub-sse-target CARGO_INCREMENTAL=0 cargo test -p roko-cli --lib connected_ -- --test-threads=1`.
- Exit/result: exit 0; 8 connected TUI tests passed, including the new topology mutation regression and existing connected snapshot replacement/bounding behavior.
- Command: `CARGO_TARGET_DIR=/private/tmp/roko-ctrl02-statehub-sse-target CARGO_INCREMENTAL=0 cargo check -p roko-runtime -p roko-serve -p roko-cli --all-targets`.
- Exit/result: exit 0. It emitted the same three unrelated heartbeat-test warnings and the pre-existing missing-crate-doc warning in `crates/roko-cli/tests/plan_validation.rs`.
- Command: `cargo fmt --all -- --check` and `git diff --check`.
- Exit/result: both exit 0 after formatting only the three changed Rust files.
- Artifact disposition: all builds used `/private/tmp/roko-ctrl02-statehub-sse-target`; it was removed after verification. The worker created no source-tree target, generated index, symlink, log, runtime process, or other artifact.

Review readiness:
- Implementation components: inherited four-file behavior in `3041d095d`; present-base race, resync, and focused-test corrections in this candidate.
- Exact candidate SHA: supplied to the independent reviewer after this evidence and the reserved source files are committed together.
- Cumulative candidate scope: `crates/roko-runtime/src/state_hub.rs`, `crates/roko-cli/src/tui/app.rs`, `crates/roko-serve/src/routes/sse.rs`, and this evidence record. `crates/roko-serve/src/lib.rs` remains byte-identical to the inherited precursor and is traced as an unchanged production call site.
- Required reviewer focus: mutex/cursor ordering; absence of replay/live gaps or duplicates; correct cursor meaning; initial and live gap frame semantics; stale queued-event suppression after resync; concurrent snapshot mutation; additive API compatibility; resource bounds; and the explicit non-claim of full `SH03-T06` or issue 37 closure.
- Known limitations: StateHub sequence state and replay ring are process-local. Snapshot-only mutations still do not emit dashboard events. The JSONL writer ignores append/flush errors and does not fsync. Other event transports retain their own recovery semantics and must be handled by their canonical tasks.

Integration:
- Independent review: pending.
- Integration commit: pending.
- Post-merge verification: pending integration-owner rerun.
- Final status: `IMPLEMENTED_UNREVIEWED`; this bounded CTRL-02 precursor does not mark `SH03-T06`, SH03, issue 37, Wave 3, or the programme done.
