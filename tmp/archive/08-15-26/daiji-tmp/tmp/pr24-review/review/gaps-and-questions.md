# PR #24: Design Gaps and Open Questions

## Security concerns

**Room key distribution is plaintext (v1).** `RoomKeyWrap.ciphertext_hex` is the plaintext
room key in hex, broadcast on the lobby. Any mesh participant can read every room key.
Impact: complete loss of room confidentiality. Mitigation: ECDH wrapping in a future PR.

**No per-message signatures (except Final).** Hello, Status, PartialResult, Vote have
`from_pubkey_hex` but no signature. Transport-level auth only. A compromised node inside
a room could forge messages.

**Lobby is fully observable.** All lobby traffic (jobs, claims, room keys) is plaintext.
By design, but worth acknowledging.

## Reliability concerns

**No message persistence.** Fire-and-forget. Agent crashes mid-job → misses all prior
messages → no recovery path.

**No message ordering.** commonware-p2p doesn't guarantee order across peers. Agents must
tolerate out-of-order messages.

**No ACKs or delivery confirmation.** Sender knows who received the frame but not if
processing succeeded.

**No heartbeat/liveness.** Only `Hello.wall_clock_ms` as baseline. No periodic keepalive.
Can't distinguish slow peer from dead peer.

## Architecture concerns

**Embedded in kora.** `run_chat()` requires `SupervisedContext` with 7+ trait bounds. Only
Rust callers. No API for external processes. Python/JS agents impossible without FFI/sidecar.

**64-slot pool is compile-time.** Can't dynamically add channels. Pool size change = recompile
+ coordinated upgrade. Is 64 enough for expected concurrent job load?

**Registry is a file polled 200ms.** Shared-state bottleneck. Multiple writers risk corruption
(mitigated by atomic write).

**No metrics.** Logs via tracing but no Prometheus counters.

**Supervisor is aggressive.** 3 failures in 60s kills chat. Network partition >1 min could
trigger false alarm.

## Key open questions

1. **Should chat be extractable from kora?** Biggest question for any agent integration.
   Keep embedded (simple) vs standalone binary (accessible) vs library (embeddable)?

2. **How do external agents send/receive messages?** No programmatic event stream exists.
   Demo driver hardcodes messages. Real agents need dynamic bidirectional message flow.

3. **What happens when the coordinator is offline?** Single point of failure: coordinator
   broadcasts JobAnnounce with room keys. If it crashes after JobAwarded but before
   broadcast, agents can't join.

4. **Is plaintext room key wrapping acceptable for merge?** Trusted mesh assumption OK
   for devnet? Or is ECDH wrapping a blocker?

5. **Chat is only in `run_legacy()`, not `run_validator()`.** Devnet validators (which use
   `kora validator`) don't run chat. Intentional?

6. **Contract alignment?** Chain watcher only watches `AgentRegistered` and `JobAwarded`.
   Plans for WorkerRegistry reputation, BountyMarket state transitions, ConsortiumValidator
   votes?
