# B — HDC Foundations And Operations

Refresh of docs `04`, `05`, `06`, and `09` against current code.

## What Ships

- `HdcVector` is already implemented in
  `crates/roko-primitives/src/hdc.rs:24-255`.
- it is a **10,240-bit** vector stored as `[u64; 160]`.
- the file is **345 LOC** and already ships the core operations:
  `bind`, `bundle`, `permute`, `similarity`, `from_seed`, `to_bytes`,
  `from_bytes`, `fingerprint`, and `text_fingerprint`.
- HDC is already consumed by neuro, learning, dreams, serve, and index-adjacent
  codepaths.
- `roko-neuro` already has a dedicated HDC encoder module for knowledge entries.

## What Does Not Need To Be Built

### No Separate `roko-hdc` Crate

The audit conclusion is explicit: a dedicated `roko-hdc` crate is unnecessary.
The existing `roko-primitives` implementation is small, real, and already
shared.

### No Consensus / Semantic-Currency Story Yet

Consensus bundles, semantic voting, and other grand HDC coordination stories
should remain deferred. The current win is retrieval and clustering, not a new
collective protocol.

## The Real Gap

The gap is **not** low-level HDC math. The gap is that HDC is still unevenly
distributed across the architecture.

Top next step:

1. add an HDC fingerprint field to `Engram`
2. compute it when the kernel object is stored or emitted
3. make later similarity search build on that shared field

## What Is Not Yet There

- `Substrate` does **not** expose `query_similar()`
- there is no shipping `HdcSubstrate`
- there is no shipping cross-domain resonance stack
- there is no shipping `BundleAccumulator`, `ResonatorNetwork`, or ontology
  layer that should be presented as current architecture

## Doc Posture

- talk about HDC primitives in present tense
- talk about HDC-on-Engram as the next concrete step
- talk about advanced HDC search, consensus, and analogy as deferred
