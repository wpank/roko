# PERSIST — Stage 7 of the Cognitive Loop

> Write the verified output back to the Substrate as durable Engrams.

**Status**: Shipping
**Crate**: `roko-agent`
**Depends on**: [Substrate trait](../03-substrate/README.md),
[Engram](../01-engram/README.md), [VerifyResult](06-stage-verify.md)
**Used by**: [REACT](08-stage-react.md), [loop\_tick()](09-loop-tick-code.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

PERSIST converts the `ActOutput` and `VerifyResult` into `Engram` records and writes
them to the Substrate. It is the stage that makes a tick's work durable. After
PERSIST, the agent's long-term memory has changed; before PERSIST, the tick is
ephemeral.

---

## The Idea

A tick without a PERSIST is a tick that forgets itself. PERSIST is where Roko's
"nothing is lost" principle lives at the implementation level: every significant
outcome — whether success, failure, soft-fail, or error — is captured as an Engram.

Three kinds of Engrams may be written per tick:

1. **Outcome Engram** — the main result of the tick (the model's response, the tool
   output, the sub-agent's report). Written with `verified = true` or `false` per the
   VERIFY result. This is the only Engram that may be absent (on HardFail).

2. **Provenance Engram** — records the full tick lineage: which stimulus triggered the
   tick, which candidates were composed, which route was taken, what the cost was. Always
   written; enables audit and debugging.

3. **Failure Engram** — written if VERIFY returned HardFail or if any prior stage
   errored. Never written on clean success. The Failure Engram carries the full failure
   reason and gate reports.

The invariant is: **every tick produces at least one Engram write.** Even a tick where
everything fails writes a Provenance Engram (and a Failure Engram).

---

## Specification

```rust
// source: crates/roko-agent/src/loop/persist.rs
pub struct PersistResult {
    pub outcome_id:    Option<EngramId>,   // None on HardFail
    pub provenance_id: EngramId,           // always present
    pub failure_id:    Option<EngramId>,   // present on HardFail or error
    pub written_at:    Timestamp,
}

pub trait PersistStage: Send + Sync {
    fn persist(
        &self,
        output:   &ActOutput,
        verify:   &VerifyResult,
        context:  &PersistContext,
        substrate: &dyn Substrate,
    ) -> Result<PersistResult, PersistError>;
}
```

`PersistContext` carries the tick ID, stimulus Pulse, route decision, composed context
summary, and budget consumption — all of which go into the Provenance Engram.

---

## Engram Construction

### Outcome Engram

```rust
Engram {
    id:          EngramId::new(),
    kind:        Kind::from_route_target(&route_decision),
    body:        Body::from_act_output(&output),
    fingerprint: HdcFingerprint::encode(&output.content),
    score:       Score::default(),   // scored at next QUERY
    provenance:  Provenance::from_tick(tick_id, stimulus_id),
    verified:    verify.verdict.is_pass(),
    created_at:  now(),
    half_life:   default_half_life_for_kind(&kind),
}
```

### Provenance Engram

The Provenance Engram captures the complete causal chain:
- Tick ID
- Stimulus Pulse ID
- Candidate IDs surfaced by QUERY
- Route decision (target, confidence)
- Token cost (prompt + completion)
- Wall time per stage
- Verification verdict

This is the Engram that powers audit trails and debugging. It is written even when the
Outcome Engram is absent.

---

## Substrate Write Properties

PERSIST calls `substrate.put(engram)` for each Engram. The substrate guarantees:

- **Durability**: the write is fsync'd before PERSIST returns (for durable substrates).
- **Idempotency**: re-writing the same `EngramId` is a no-op.
- **Atomicity**: all Engrams for a tick are written in a single transaction where the
  substrate supports transactions.

In-memory substrates (used in tests) do not fsync. Production deployments use
`roko-substrate-sled` or `roko-substrate-postgres`.

---

## Failure Modes

| Failure | Cause | Recovery |
|---|---|---|
| `PersistError::SubstrateDown` | Substrate unavailable | Retry 3× with backoff; publish `persist.failed` Pulse |
| `PersistError::QuotaExceeded` | Substrate storage limit reached | Trigger decay pass; retry once |
| `PersistError::SchemaError` | Engram body invalid | HardFail this tick; write Failure Engram to backup substrate |
| Partial write | Substrate connection dropped mid-write | Re-run from last successful write on reconnect |

---

## Performance

| Metric | Target | P99 budget |
|---|---|---|
| Wall time (in-process substrate) | < 1 ms | < 3 ms |
| Wall time (sled on NVMe) | < 3 ms | < 8 ms |
| Wall time (Postgres) | < 10 ms | < 25 ms |
| Engrams written per tick | 2–3 | — |

---

## Examples

### 1. Successful tick

VERIFY passed. PERSIST writes: (1) Outcome Engram with the model's response,
`verified = true`; (2) Provenance Engram with full tick lineage. Two Engrams written.

### 2. HardFail tick

VERIFY returned HardFail on safety grounds. PERSIST writes: (1) Provenance Engram;
(2) Failure Engram with gate reports. Zero Outcome Engrams. The long-term memory has
a record of the attempted and rejected tick.

### 3. ACT error (timeout)

ACT timed out. VERIFY skipped. PERSIST writes: (1) Provenance Engram with
`act_timed_out = true`; (2) Failure Engram. The agent can query these to detect
repeated timeouts on the same stimulus.

---

## See also

- [VERIFY](06-stage-verify.md) — the verdict that controls what is written here
- [REACT](08-stage-react.md) — fires Pulses based on what was persisted
- [Substrate trait](../03-substrate/README.md) — the backing store
- [Engram](../01-engram/README.md) — the data type written here
- [Invariants](12-invariants.md) — the every-tick-produces-one-write invariant
