# 67 — HDC-Based Knowledge Retrieval in Prompt Assembly

**Priority**: P2 — HDC infrastructure is built but three compose-layer code paths are compiled out due to a missing feature flag; enabling it improves context deduplication and knowledge retrieval quality
**Size**: M (2 sections, feature-flag wiring + retrieval audit)
**Crates**: `crates/roko-cli` (`Cargo.toml`), `crates/roko-serve` (`Cargo.toml`), `crates/roko-compose` (`src/prompt.rs`, `src/memory_functor.rs`), `crates/roko-neuro` (`src/context.rs`, `src/knowledge_store.rs`)
**Depends on**: None

---

## Background

HDC (Hyperdimensional Computing) fingerprints are computed and stored per episode. `roko-neuro` has a complete HDC scoring infrastructure, and the runner's event loop already uses HDC similarity to find past episodes relevant to the current task. However, the `roko-compose` crate defines its own `hdc` feature that enables three additional code paths — dual-retrieval in `MemoryFunctor`, deduplication in `PromptComposer`, and semantic similarity in `ContextAssembler` — and neither `roko-cli` nor `roko-serve` enables this feature. Those three code paths are currently dead code even though the HDC primitives are already compiled in.

The fix is a one-line change per crate (`features = ["hdc"]` on the `roko-compose` dependency), followed by verification that the activated code paths compile and produce reasonable behavior, and an optional threshold configuration so users can control when dedup fires.

## Current State

1. **`roko-cli` Cargo.toml** — `crates/roko-cli/Cargo.toml` line 26:
   ```toml
   roko-compose = { path = "../roko-compose" }
   ```
   No `features = ["hdc"]`. The `hdc` feature on `roko-compose` is not enabled.

2. **`roko-serve` Cargo.toml** — `crates/roko-serve/Cargo.toml` line 35:
   ```toml
   roko-compose = { path = "../roko-compose" }
   ```
   No `features = ["hdc"]`.

3. **`roko-compose` `hdc` feature** — `crates/roko-compose/Cargo.toml` line 33:
   ```toml
   hdc = ["dep:roko-primitives", "roko-neuro/hdc"]
   ```

4. **`MemoryFunctor::query()` dual-retrieval** — `crates/roko-compose/src/memory_functor.rs`. When the `hdc` feature is disabled, `query()` (line 59–73) uses only keyword-based `query_hits()`. When enabled, lines 75–118 additionally call `store.query_similar()` with an HDC fingerprint derived from the task text, merge results by entry ID keeping the higher-scoring retrieval per entry, and return a combined ranked list with a `retrieval = "hdc"` tag on HDC-matched entries.

5. **`hdc_dedup_candidates()` in `PromptComposer`** — `crates/roko-compose/src/prompt.rs`. Gated by `#[cfg(feature = "hdc")]` at line 1740. The `hdc_dedup_threshold` field on `PromptComposer` is always defined (line 649) and defaults to `0.0` (line 669). When the feature is disabled the `with_hdc_dedup()` builder method is compiled but the `hdc_dedup_candidates()` call at line 915–916 is compiled out. The `hdc_dedup_candidates` function itself is at line 1741.

6. **`semantic_similarity()` in `ContextAssembler`** — `crates/roko-neuro/src/context.rs`. Two versions: the HDC-enabled version at lines 1147–1158 augments keyword overlap with calibrated HDC fingerprint similarity (threshold `HDC_RELEVANCE_THRESHOLD = 0.525` at line 461); the fallback at lines 1161–1164 uses pure keyword overlap. **This function lives in `roko-neuro`, not `roko-compose`, so it is already compiled in because `roko-cli` enables `roko-neuro` with `features = ["hdc"]` (Cargo.toml line 32).** `semantic_similarity()` is called from `dedup_similar_chunks()` at line 1358 and chunk scoring at line 1452 — both in `context.rs`.

7. **HDC in `roko-neuro` (already active)** — `crates/roko-cli/Cargo.toml` line 32 and `crates/roko-serve/Cargo.toml` line 30 both enable `roko-neuro` with `features = ["hdc"]`. This means:
   - `KnowledgeStore::query_similar()` (line 1222), `query_hdc()` (line 1272), `query_by_role_filler()` (line 1332), and `find_resonances()` (line 1345) are all compiled in.
   - `score_entry_for_query()` (line ~1255) includes `hdc_similarity` contribution when entries have a populated `hdc_vector`.
   - The `ContextAssembler`'s `semantic_similarity()` already uses HDC (because `roko-neuro/hdc` is enabled and `semantic_similarity` is in `roko-neuro`).
   - `backfill_hdc_vectors()` at line 2412 is compiled in.

8. **What is actually dead code** — Only the code in `roko-compose` behind `#[cfg(feature = "hdc")]`:
   - `MemoryFunctor` dual-retrieval (lines 75–118 of `memory_functor.rs`)
   - `hdc_dedup_candidates()` pre-pass in `PromptComposer` (`prompt.rs` lines 1740–end of function)
   - The call at `prompt.rs` lines 913–916

9. **`MemoryFunctor` wiring** — `MemoryFunctor` is used by the `AuctionRoom` in `crates/roko-compose/src/auction.rs` (line 740, 752, 893). The `AuctionRoom` is used inside `roko-compose` itself. It is NOT directly wired into `roko-cli`'s runner event loop (`grep -rn "MemoryFunctor\|AuctionRoom" crates/roko-cli/src/` returns no results). The runner uses `PromptAssemblyService` (from `roko-compose/src/prompt_assembly_service.rs`) and calls knowledge store methods directly via `roko-neuro`. The `AuctionRoom`/`MemoryFunctor` path is exercised by the prompt auction (for VCG-based context section assembly) but is not the primary path for knowledge query at dispatch time.

10. **Runner similar-episode injection (already active)** — `crates/roko-cli/src/runner/event_loop.rs` lines 9870–9910. The runner computes a task HDC fingerprint via `roko_learn::hdc_fingerprint::fingerprint_episode()` and calls `EpisodeLogger::query_similar_episodes()`. This is working and does not depend on the missing `roko-compose/hdc` feature.

## Implementation Plan

### Section A: Enable the `hdc` feature on `roko-compose`

#### A1. Add `features = ["hdc"]` to `roko-compose` in both consumer crates

**File: `crates/roko-cli/Cargo.toml`** (line 26):
```toml
# Before:
roko-compose = { path = "../roko-compose" }
# After:
roko-compose = { path = "../roko-compose", features = ["hdc"] }
```

**File: `crates/roko-serve/Cargo.toml`** (line 35):
```toml
# Before:
roko-compose = { path = "../roko-compose" }
# After:
roko-compose = { path = "../roko-compose", features = ["hdc"] }
```

These are the only two changes required to activate the three gated code paths in `roko-compose`.

#### A2. Verify the activated code compiles cleanly

Run:
```bash
cargo build -p roko-cli
cargo build -p roko-serve
cargo test -p roko-compose --features hdc
```

Potential compilation issues to check:

- **`MemoryFunctor::query()` HDC path** (`memory_functor.rs` lines 75–118): Calls `store.query_similar()` which returns `Vec<KnowledgeSimilarityHit>`. The `KnowledgeSimilarityHit` struct has a `similarity: f64` field (confirmed at `knowledge_store.rs` line 228). The `f64::from(hit.similarity)` conversion at `memory_functor.rs:103` should be valid. Confirm the `BTreeMap` dedup merge at lines 85–110 compiles without type issues.

- **`hdc_dedup_candidates()` (`prompt.rs` line 1741)**: Takes the candidate list and computes HDC fingerprints via `HdcVector::from_seed()`. This is a content-hash fingerprint (bytes of section content), not a semantic embedding. Confirm `roko-primitives::hdc::HdcVector` is accessible via the feature re-export in `roko-compose`.

- **`with_hdc_dedup()` builder** (`prompt.rs` line 806): This method exists unconditionally (no `#[cfg]`), so it compiles regardless. The call site at lines 913–916 is gated. Confirm no conflicts after enabling the feature.

#### A3. Set a default HDC dedup threshold

The `hdc_dedup_threshold` field defaults to `0.0` in `PromptComposer::default()` (`prompt.rs` line 669). When `0.0`, the `hdc_dedup_candidates()` call at line 915 is skipped even with the feature enabled. Change the default to `0.85`:

**File: `crates/roko-compose/src/prompt.rs`** (line 669):
```rust
// Before:
hdc_dedup_threshold: 0.0,
// After:
hdc_dedup_threshold: 0.85,
```

A threshold of `0.85` is conservative — it only deduplicates content sections whose HDC fingerprints are nearly identical at the byte level (catching copy-paste duplicates), not semantically similar content. This avoids removing legitimate contextual variation.

Alternatively, expose this as a `roko.toml` key under `[compose]`:

```toml
[compose]
hdc_dedup_threshold = 0.85  # 0.0 = disabled, 0.85 = conservative dedup
```

Add a `ComposeConfig` struct to `crates/roko-core/src/config/schema.rs` if this path is chosen. Either approach is acceptable; the inline default change is simpler.

### Section B: Validate the activated retrieval paths

#### B1. Confirm `ContextAssembler` HDC path is already active

Run `cargo tree -p roko-cli -e features | grep roko-neuro` to confirm `roko-neuro` appears with `hdc`. Since this is already enabled (line 32 of `roko-cli/Cargo.toml`), `semantic_similarity()` in `context.rs` already uses HDC fingerprints for chunk deduplication and scoring. No code change is needed here — this is already working.

Manually verify by running `roko do "test" --context /some/file` and checking that debug logs from `dedup_similar_chunks` appear (enable RUST_LOG=roko_neuro=debug).

#### B2. Confirm `MemoryFunctor` is exercised by the auction path

The `AuctionRoom` in `roko-compose/src/auction.rs` creates a `MemoryFunctor` at line 893. With the `hdc` feature enabled, `MemoryFunctor::query()` will use dual-retrieval. The `AuctionRoom` is used during prompt auction (VCG context section assembly). Confirm the auction is called during live plan execution by checking whether `AuctionRoom::run_auction` appears in runner traces.

If the `AuctionRoom` is active in production, the dual-retrieval in `MemoryFunctor` is automatically active after A1. If it is not active, the MemoryFunctor's HDC path is still dead code at runtime (even if compiled in). Document this finding in `.roko/GAPS.md`.

#### B3. Wire task-fingerprint knowledge query in event_loop (optional)

The runner's similar-episode query (lines 9870–9910) computes a task HDC fingerprint but only queries episodes, not the knowledge store. An improvement is to also query the knowledge store by the same fingerprint and merge results with the keyword-based technique query.

In `crates/roko-cli/src/runner/event_loop.rs`, after the existing `query_similar_episodes` call, add:

```rust
// Also query knowledge store by task HDC fingerprint
if let Ok(knowledge_store) = neuro_store_for_workdir(&ctx.paths.workdir) {
    let fp_bytes = task_fp.to_bytes();
    match knowledge_store.query_similar(&fp_bytes, 3) {
        Ok(hits) => {
            // Merge hits into technique context (same format as query_techniques())
            // ...
        }
        Err(err) => debug!(%err, "knowledge HDC query failed"),
    }
}
```

This closes the loop between the write path (episodes store HDC fingerprints) and the knowledge read path. This step is optional and can be done after A1–A3.

#### B4. Invoke `backfill_hdc_vectors()` for existing entries

The `backfill_hdc_vectors()` method at `crates/roko-neuro/src/knowledge_store.rs` line 2412 populates the `hdc_vector` field for existing knowledge entries that were ingested before HDC was enabled. It is gated on `#[cfg(feature = "hdc")]` and is already compiled in (because `roko-cli` enables `roko-neuro/hdc`).

Wire this into the `roko knowledge backfill-hdc` CLI command (check if it already exists: `grep -n "backfill" /Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/main.rs`). If not, add it as a subcommand of `roko knowledge` so users can run it after upgrading to a version with HDC enabled. The backfill is idempotent — entries that already have `hdc_vector` are skipped.

## Acceptance Criteria

### Section A

1. `cargo tree -p roko-cli -e features | grep roko-compose` shows `roko-compose` with `hdc` in its feature list.
2. `cargo tree -p roko-serve -e features | grep roko-compose` shows `roko-compose` with `hdc` in its feature list.
3. `cargo build -p roko-cli` compiles cleanly (no errors, no regressions from enabling the feature).
4. `cargo test -p roko-compose --features hdc` passes.
5. `cargo test --workspace` passes.
6. The `MemoryFunctor::query()` dual-retrieval path compiles and is linked into the `roko` binary (verify via `nm target/debug/roko | grep -i memory_functor` or equivalent).
7. The `hdc_dedup_candidates()` function in `prompt.rs` is compiled in.

### Section B

1. `ContextAssembler`'s `semantic_similarity()` uses the HDC branch (already active; verify no regression).
2. Knowledge entries ingested after this change have `hdc_vector` populated (already the case via `roko-neuro/hdc`; confirm no regression by checking a freshly ingested entry).
3. `roko knowledge backfill-hdc` command exists and successfully populates `hdc_vector` for legacy entries (or a note is added to GAPS.md explaining the backfill gap).
4. If `AuctionRoom` is active at runtime: `MemoryFunctor` dual-retrieval merges keyword and HDC results. If not active: finding is documented in GAPS.md.

## Verification Checklist

- [ ] Run `cargo tree -p roko-cli -e features | grep roko-compose`; confirm `hdc` in feature list
- [ ] Run `cargo tree -p roko-serve -e features | grep roko-compose`; confirm `hdc` in feature list
- [ ] Run `cargo build -p roko-cli`; confirm success
- [ ] Run `cargo build -p roko-serve`; confirm success
- [ ] Run `cargo test -p roko-compose --features hdc`; confirm all tests pass
- [ ] Run `cargo test --workspace`; confirm no regressions
- [ ] Run `RUST_LOG=roko_compose=debug roko do "hello"` and check logs for HDC dedup messages
- [ ] Ingest a knowledge entry; inspect `.roko/learn/knowledge.jsonl` (or similar) and confirm `hdc_vector` field is non-null
- [ ] Run `roko knowledge backfill-hdc` (or check GAPS.md for the gap)
- [ ] Update `.roko/GAPS.md` with finding about `AuctionRoom` activity in production

## Files to Modify

| File | Change |
|---|---|
| `crates/roko-cli/Cargo.toml` | Add `features = ["hdc"]` to `roko-compose` dependency (line 26) |
| `crates/roko-serve/Cargo.toml` | Add `features = ["hdc"]` to `roko-compose` dependency (line 35) |
| `crates/roko-compose/src/prompt.rs` | Change `hdc_dedup_threshold` default from `0.0` to `0.85` (line 669) |
| `crates/roko-cli/src/main.rs` | (Optional) Add `roko knowledge backfill-hdc` subcommand to `KnowledgeCmd` |
| `crates/roko-cli/src/commands/knowledge.rs` | (Optional) Implement `cmd_knowledge_backfill_hdc()` that calls `knowledge_store.backfill_hdc_vectors()` |
| `.roko/GAPS.md` | Document whether `AuctionRoom`/`MemoryFunctor` is active in the production runner dispatch path |
