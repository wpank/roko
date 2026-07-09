# Neuro Summary — Batch 06

Concise post-audit picture for `tmp/docs-parity/06/context-pack/`.

## What Is Real As Of The 2026-04-17 Audit

- `roko-neuro` is a real, wired crate with 7 source files: `lib.rs`, `knowledge_store.rs`, `context.rs`, `hdc.rs`, `distiller.rs`, `tier_progression.rs`, and `episode_completion.rs`.
- The core runtime surfaces are already present: append-only knowledge storage, HDC-backed neuro indexing, distillation, and tier progression.
- `HdcVector` already exists in `crates/roko-primitives/src/hdc.rs` at 345 LOC. No separate `roko-hdc` crate is needed.
- The HDC stack is reused across the workspace: neuro, learn, dreams-side audit paths, CLI episode fingerprinting, and other retrieval/search surfaces already depend on it.

## Highest-Value Missing Seam

- `HDC-on-Engram` is the top priority item.
- In practice this means: the existing HDC substrate is strong enough that the next meaningful neuro step is attaching an HDC fingerprint directly to `Engram`, then treating any broader similarity API as follow-on work.
- This context pack should record that priority clearly, but it should not turn PU06 into a runtime activation plan.

## What The Docs Must Stop Implying

- `roko-neuro` is not a stub or future crate. It is present and wired now.
- Cross-domain transfer is not a shipped retrieval layer.
- Library of Babel, mesh sync, and publish/economics flows are not near-term runtime surfaces here.
- Demurrage is not partially implemented; it remains deferred.
- Pulse / Datum / Worldview / Custody belong to target-state architecture language, not current neuro-runtime status.

## What PU06 Should Actually Do

1. Refresh the docs to match the audit reality.
2. Mark HDC-on-Engram as the top queued neuro improvement.
3. State that `query_similar` on Substrate and broader cross-domain HDC transfer are not yet wired.
4. Push demurrage, Library of Babel, mesh sync, cross-domain transfer, publish/economics, and chain-state concepts into explicit deferred or target-state buckets.

## Scope Boundary

This pack is for parity and truth-in-advertising. It should help later agents describe the subsystem honestly, not instruct them to activate every dormant runtime seam in one pass.
