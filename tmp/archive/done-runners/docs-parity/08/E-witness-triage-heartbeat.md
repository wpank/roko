# E — Witness, Triage, Heartbeat

This section needs a hard separation between one small shipped primitive and a
much larger deferred pipeline.

## Shipped Claim

- `ChainWitnessEngine` in `crates/roko-chain/src/witness.rs` ships.

That shipped claim should stay narrow. It is enough to say there is a witness
primitive in the chain crate. It is not enough to say the full witness,
triage, anomaly, and heartbeat architecture exists.

## Deferred

- block-observer pipeline as described in the chain PRDs
- triage and curiosity scoring
- dedicated heartbeat publication loop
- Binary Fuse, MIDAS, or similar algorithmic machinery

## Parity Rule

Use present tense only for `ChainWitnessEngine`. Everything that depends on a
larger chain-observability pipeline should be labeled `Phase 2+` or
`DEFERRED`.
