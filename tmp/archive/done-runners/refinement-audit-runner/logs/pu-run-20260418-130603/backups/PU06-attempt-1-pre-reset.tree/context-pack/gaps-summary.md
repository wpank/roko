# Gap Inventory — 06 Neuro

Post-audit gap list for the `06-neuro` docs-parity refresh.

## Focus Now

These are the gaps PU06 should make explicit in the docs:

### 1. HDC-On-Engram Is The Clear Next Step — HIGH

- `HdcVector` is already real in `crates/roko-primitives/src/hdc.rs` at 345 LOC.
- The HDC toolchain is already used for fingerprinting and retrieval-adjacent work.
- The highest-value missing seam is still an HDC fingerprint on `Engram`.
- PU06 should rank this first, but only as a documented priority.

### 2. Neuro Docs Still Undersell What Already Ships — HIGH

- `roko-neuro` is 7 source files and wired.
- `KnowledgeStore`, `Distiller`, and `TierProgression` are not speculative.
- Any wording that treats neuro as mostly future work is now stale.

### 3. Several Neuro-Adjacent Narratives Still Overclaim — HIGH

- `query_similar` is not yet a Substrate-level contract.
- Cross-domain HDC transfer is not a shipped runtime capability.
- Library of Babel, mesh sync, and publish/economics language should not read as active implementation.

### 4. Demurrage And Chain-Side Economic Stories Remain Design-Only — HIGH

- The audit direction is clear: mark demurrage as deferred.
- Do the same for publish/economics surfaces that depend on chain policy rather than current neuro code.

### 5. Target-State Vocabulary Needs Boundary Labels — MEDIUM

- Pulse, Datum, Worldview, and Custody are still target-state architecture terms in this context.
- They should appear only as deferred or target-state references, not as current batch deliverables.

## Defer From PU06

These are valid topics, but they are not the job of this docs refresh:

- runtime activation of HDC-on-Engram
- Substrate `query_similar` implementation
- demurrage and token economics
- Pulse / Datum / Worldview / Custody realization
- Library of Babel
- mesh sync
- cross-domain transfer
- publish policy and economics

## Working Rule

If a point requires new runtime wiring, chain policy, or frontier architecture, PU06 should describe the boundary honestly and hand it off. The docs-parity pass succeeds by clarifying status and priority, not by expanding the implementation plan.
