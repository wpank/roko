# Quick Wins — Things That Can Be Done in Hours

These are changes that move toward v2 without any architectural risk. Each one is
self-contained and can be done independently.

## QW-1: Rename Engram → Signal (2-4 hours)

**What**: Make `Signal` the canonical type name, `Engram` the deprecated alias.

**Current state**: `signal.rs` already has `pub type Signal = Engram;`. The rename is
half-done.

**Steps**:
1. In `roko-core/src/engram.rs`, rename `pub struct Engram` → `pub struct Signal`
2. In `roko-core/src/lib.rs`, add `pub type Engram = Signal;` (backwards compat alias)
3. Update all internal uses in roko-core to use `Signal`
4. Run `cargo build --workspace` — the alias means external crates keep compiling
5. Gradually update external crates to use `Signal` directly
6. Mark `Engram` alias as `#[deprecated]` after all crates are updated

**Wire target**: Everything that uses Engram already works — this is a rename.

**Verification**: `cargo test --workspace` passes.

---

## QW-2: Add And/Or/Not to TopicFilter (1-2 hours)

**What**: Extend `TopicFilter` with combinators.

**Current state**: TopicFilter has `Exact`, `Prefix`, `AnyOf`, `All`. Missing: `And`,
`Or`, `Not`.

**Steps**:
1. Add variants to TopicFilter enum in `roko-core/src/pulse.rs`
2. Update `matches()` method
3. Add tests

**Wire target**: PulseBus subscriptions immediately gain richer filtering.

**Verification**: Existing PulseBus tests pass + new combinator tests.

---

## QW-3: Add `balance` field to Engram/Signal (1-2 hours)

**What**: Add the demurrage balance field that v2 requires.

**Current state**: Engram has `decay: Decay` (weight decay) but not `balance: f64`.

**Steps**:
1. Add `pub balance: f64` to Engram struct (default 1.0)
2. Add `#[serde(default = "default_balance")]` for backwards compat with existing JSONL
3. Update `ContentHash` computation to EXCLUDE balance (like score, it's mutable metadata)
4. Add `pub fn touch(&mut self)` that resets balance to 1.0 (demurrage reset on access)

**Wire target**: Store::put and Store::get — balance is just a field.

**Verification**: Existing serialization tests pass with default, new tests for touch().

---

## QW-4: Feature-gate orchestrate.rs (30 minutes)

**What**: Confirm the feature gate is clean and remove any non-gated references.

**Current state**: Already behind `legacy-orchestrate` feature. But there may be stale
imports or references.

**Steps**:
1. `grep -rn 'orchestrate' crates/roko-cli/src/ --include='*.rs' | grep -v target/ | grep -v test`
2. Ensure ALL references are behind `#[cfg(feature = "legacy-orchestrate")]`
3. Consider moving orchestrate.rs to a separate file or crate to reduce roko-cli compile times

**Wire target**: N/A — this is cleanup.

**Verification**: `cargo build -p roko-cli` compiles without the feature.

---

## QW-5: Delete roko-calc skeleton (15 minutes)

**What**: Remove the empty skeleton crate.

**Steps**:
1. Remove `crates/roko-calc/` directory
2. Remove from workspace Cargo.toml
3. Remove any Cargo.toml dependency references

**Wire target**: N/A — removing dead code.

**Verification**: `cargo build --workspace` passes.

---

## QW-6: Add `execute()` stub to Cell trait (1-2 hours)

**What**: Add the v2 execution method to Cell with a default implementation.

**Current state**: Cell trait has identity + metadata but no execution method.

**Steps**:
1. Add `CellContext` struct to `roko-core/src/cell.rs`:
   ```rust
   pub struct CellContext {
       pub bus: Arc<dyn Bus>,
       pub store: Arc<dyn Store>,
       pub cancel: CancellationToken,
       // Start minimal, grow later
   }
   ```
2. Add default method to Cell trait:
   ```rust
   async fn execute(&self, _input: Vec<Signal>, _ctx: &CellContext)
       -> Result<Vec<Signal>> {
       Err(Error::msg("Cell::execute not implemented"))
   }
   ```
3. This is backwards-compatible — existing Cell impls don't need to change yet

**Wire target**: The default impl means nothing breaks. Real impls come in Phase 1.

**Verification**: `cargo test --workspace` — nothing calls execute() yet, just proving
it compiles.

---

## QW-7: Audit and tag floating code (2-3 hours)

**What**: Add `#[doc(hidden)]` or module-level comments to all floating code identified
in `01-CURRENT-STATE.md`.

**Steps**:
1. For each floating module in roko-runtime (8 modules):
   - Add `//! STATUS: NOT WIRED — built but not called from any runtime path.`
2. For each floating module in roko-learn (14 modules):
   - Same status comment
3. For roko-lang-* crates:
   - Same status comment in lib.rs

**Wire target**: N/A — documentation/audit.

**Verification**: Visual inspection.

---

## QW-8: Wire `calibration_policy` from roko-learn (3-4 hours)

**What**: This is the predict-publish-correct loop that v2 makes structural. It's
already implemented but never called.

**Current state**: `roko-learn/src/calibration_policy.rs` exists. Zero callers.

**Steps**:
1. Read the existing implementation
2. Wire it into the CascadeRouter's model selection feedback loop
3. After each agent turn, publish a calibration event to the Bus
4. CalibrationPolicy subscribes and updates the router's confidence estimates

**Wire target**: `roko plan run` → agent turn → calibration event → router update.

**Verification**: Run a plan, check that `.roko/learn/cascade-router.json` shows
updated confidence from calibration (not just raw episode counts).

---

## Priority Order

| # | Item | Time | Impact | Risk |
|---|------|------|--------|------|
| QW-4 | Feature-gate orchestrate.rs cleanup | 30m | Hygiene | None |
| QW-5 | Delete roko-calc | 15m | Hygiene | None |
| QW-7 | Audit floating code | 2-3h | Visibility | None |
| QW-3 | Add balance field | 1-2h | v2 alignment | Low |
| QW-2 | TopicFilter combinators | 1-2h | v2 alignment | Low |
| QW-1 | Engram → Signal rename | 2-4h | v2 alignment | Low |
| QW-6 | Cell execute() stub | 1-2h | v2 alignment | Low |
| QW-8 | Wire calibration_policy | 3-4h | Closes gap | Low-Med |
