# CTRL-02 StateHub/SSE precursor independent review

## Assignment and immutable candidate

- Verdict target: bounded Wave 0 `CTRL-02` attribution/correction adjacent to
  `SH03-T06`; this is not review of full `SH03-T06`.
- Exact base: `9c015378a37f75c81bacee946e1174ba9478eba1`.
- Exact candidate: `0d9b43781966f6488ccf385d6aa8f62825d8b855`.
- Historical precursor: `3041d095d4daebed2c9e05c63eacb18e668e37e3`
  relative to `1649c18b2c3d2b3602bfe17398b0e1454a19c5ef`.
- Review branch/worktree: `review/CTRL-02-statehub-sse-0d9b4378` at
  `/Users/will/dev/nunchi/roko/agent-worktrees/status-quo-20260714T073140Z/reviews/CTRL-02-statehub-sse-0d9b4378`.
- I did not implement the candidate. I read the complete master checklist, the
  complete SH03 manifest, issue 37, worker evidence, the historical/current/
  candidate diffs, the StateHub/EventBus implementation, all changed tests, the
  connected TUI paths, the mounted SSE route, and the unchanged serve bootstrap
  and bidirectional bridge call sites.

## Reconstructed requirement and scope

The inherited precursor must keep snapshot mutation, best-effort append,
sequence assignment, ring insertion, and broadcast ordered so a received event
never points at older state. Selected TUI and serve snapshot-field mutations must
not overwrite concurrent runner publications. The correction must additionally
make SSE reconnect an atomic replay/live handoff, replay exactly after
`Last-Event-ID`, replace an evicted or greater-than-256 suffix with an explicit
snapshot gap, and prevent pre-resync queued events from appearing after that
snapshot.

The candidate is intentionally bounded. It does not claim a durable event log,
persistent sequence numbers, mutation broadcasts for `apply_snapshot`/
`update_snapshot`, recovery for the separate StateHub-to-server bridge,
WebSocket/status/workflow streams, correction of the duplicate
`roko-core::state_hub`, the other issue 37 findings, completion of `SH03-T06`, or
closure of issue 37.

## Independent history and scope proof

- `git merge-base --is-ancestor 3041d095d... 9c015378a...` exited 0.
- The four historical production blobs at `3041d095d` and `9c015378a` were
  byte-identical. Their SHA-256 values were:
  - runtime StateHub: `ac0fc978e532a2b5db30af09d6a3f28dc2026c43035300d67140e7b705ac697e`
  - CLI TUI app: `a7689c5317af4a858fb72e532cea1fe61520c461240eb2f4f859a0af66dbc920`
  - serve lib: `b7e4300add597a9452ef1ed07e51c440b2c066e1a6287745bd9afc938eaed1b4`
  - serve SSE route: `c7e7b9b98725888c170504d7e04154b04798aca918e7cf3d1719f4bda24a63b3`
- `git diff 1649c18b..3041d095 -- <four production paths>` independently
  shows the inherited commit-before-broadcast lock, selected-field mutation
  migration, exclusive SSE cursor increment, and first live-lag gap behavior.
- `git diff --name-only 9c015378a..0d9b43781` contains exactly four paths:
  `crates/roko-runtime/src/state_hub.rs`, `crates/roko-cli/src/tui/app.rs`,
  `crates/roko-serve/src/routes/sse.rs`, and worker evidence. The assigned
  `crates/roko-serve/src/lib.rs` call path is unchanged in the candidate.
- `git diff --check 9c015378a..0d9b43781` passed. No candidate production
  path adds `TODO`, `FIXME`, `todo!`, or `unimplemented!`.

## Changed-line and production-path review

### Publication, snapshot, and cursor boundary

`StateHub::publish`, `publish_batch`, `apply_snapshot`, `update_snapshot`,
`replay_log_into_snapshot`, and `StateHubSender::publish` use the same
`publish_lock`. The event bus is private and its sender is exposed only through
the lock-bearing `StateHubSender`, so current production publishers cannot bypass
that boundary. Publication applies snapshot state before assigning/inserting/
broadcasting the sequence. Lock acquisition order is consistent
(`publish_lock`, then watch/log/event-bus internals); current mutation closures
perform only field assignment and do not re-enter the hub.

`subscribe_events_from` holds that lock while installing the broadcast receiver,
copying retained replay, reading `total_emitted`, and cloning the snapshot. Thus
the replay contains only sequences below `cursor.next_seq` and the receiver starts
at that exact boundary. `cursor_snapshot` takes the same lock, so live lag cannot
pair a pre-publication cursor with a post-publication snapshot or vice versa.
The additive public types and methods leave the existing publish/subscribe/replay
interfaces intact.

### Reconnect, eviction, size boundary, and live lag

No-header replay begins at sequence zero; a valid `Last-Event-ID = N` begins at
`N + 1`. An empty suffix at `requested_seq == next_seq` is accepted. For a
non-empty suffix, the first event must equal the request, every adjacent pair must
be consecutive, the last event plus one must equal the atomic cursor, and length
must be at most 256. Therefore eviction, a hole, a missing tail, and the 257-event
boundary all produce one `gap` frame rather than silent truncation.

The gap ID is `cursor.next_seq - 1`, so a reconnect continues at the first event
not represented by its snapshot. On live `Lagged`, the stream records its prior
floor, captures a new atomic cursor/snapshot, emits a gap whose `missed_events`
covers the whole replaced suffix, and raises the floor to the new cursor. Tokio's
retained pre-snapshot queue is then drained but suppressed below the floor; events
published after the capture remain queued at or above it. This closes both the
replay/subscribe race and stale-event-after-resync race without an unbounded replay
or busy loop.

### Connected TUI and compatibility

Both verdict refresh paths and agent-topology refresh mutate selected fields with
`update_snapshot`; unchanged serve bootstrap also uses it for cascade-router state.
The new connected topology regression exercises the real `App::new_connected`
topology method, and the independent runtime stress test covers interleaved
publishers and selected-field mutations. Ordinary SSE event JSON and numeric IDs
are unchanged; only explicit `event: gap` frames carry the snapshot recovery
payload.

I also inspected the unchanged StateHub/server bridge, `/api/statehub/events`,
WebSocket, workflow SSE, bootstrap replacement, duplicate core StateHub, JSONL
writer, and issue 37 ledger. Their distinct lag/durability/broadcast and accounting
defects remain real and are not silently represented as fixed by this candidate.

## Independent verification

All builds used the isolated target
`/private/tmp/roko-ctrl02-statehub-review-target` with incremental compilation
disabled.

- `cargo test -p roko-runtime state_hub -- --test-threads=1` — exit 0; 11
  passed, 0 failed/ignored. This includes commit-before-broadcast observation,
  gap-free replay/live handoff, sender publication, batch publication, log replay,
  and concurrent publication/mutation preservation. It emitted three pre-existing
  heartbeat-test warnings unrelated to the candidate.
- `cargo test -p roko-serve routes::sse -- --test-threads=1` — exit 0; 4
  passed, 0 failed/ignored. Exact suffix, evicted cursor snapshot, 257-event
  boundary, and Last-Event-ID increment all passed.
- `cargo test -p roko-cli --lib connected_ -- --test-threads=1` — exit 0; 8
  passed, 0 failed/ignored. This includes connected topology preservation, live
  completion/refresh behavior, and bounded/nonduplicating connected output.
- `cargo fmt --all -- --check` — exit 0.
- Candidate diff check, exact four-path scope check, historical ancestry/blob
  checks, and clean pre-review worktree check — all passed.

The worker's reported full `roko-serve` run (470 tests) and affected-package
all-target check were not duplicated after the independent cold focused builds:
the exact candidate source and call paths supporting those results were reviewed,
and the required runtime, serve, and CLI test targets all compiled independently.

## Verdict

**ACCEPTED**

Confidence: high. No required correction remains for this bounded candidate.
The integration owner should merge this review with exact candidate
`0d9b43781966f6488ccf385d6aa8f62825d8b855`, rerun the focused gates after merge,
and retain the explicit nonclaims. It must not mark `SH03-T06`, SH03, issue 37, or
the programme complete from this precursor acceptance.
