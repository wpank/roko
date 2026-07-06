# Phase A: Refactoring Sequence

All refactoring steps ordered by dependency. Each step should be a separate PRD → plan → execute cycle. No new features — only alignment with the docs.

## Guiding Principles

1. **Mechanical before structural before semantic** — renames first, then moves, then behavior changes
2. **Compile after every step** — `cargo check --workspace` must pass before the next step
3. **One breaking change at a time** — don't combine a crate rename with a type rename
4. **Multi-plan for cross-crate changes** — use `depends_on_plan` for sequencing

---

## Step 1: Rename `bardo-runtime` → `roko-runtime`

**Type**: Mechanical (find-and-replace)
**Risk**: Low — no semantic change
**Scope**: 4 Cargo.toml + ~10 import statements

### Substeps
1. `mv crates/bardo-runtime crates/roko-runtime`
2. Update `crates/roko-runtime/Cargo.toml`: `name = "roko-runtime"`
3. Update workspace `Cargo.toml`: member path
4. Update all dependent `Cargo.toml` files: dependency name + path
5. Find-replace `bardo_runtime` → `roko_runtime` in all `.rs` files
6. `cargo check --workspace`

### Verification
```bash
! grep -rn 'bardo.runtime\|bardo_runtime' crates/ --include='*.toml' --include='*.rs' | grep -v target/
cargo check --workspace
```

---

## Step 2: Rename `bardo-primitives` → `roko-primitives`

**Type**: Mechanical
**Risk**: Low
**Scope**: 6 Cargo.toml + ~18 import statements
**Depends on**: Step 1 (to avoid merge conflicts in shared Cargo.toml files)

### Substeps
Same pattern as Step 1.

### Verification
```bash
! grep -rn 'bardo.primitives\|bardo_primitives' crates/ --include='*.toml' --include='*.rs' | grep -v target/
cargo check --workspace
```

---

## Step 3: Dissolve `roko-golem`

**Type**: Structural (moving code between crates)
**Risk**: Medium — multiple crates affected
**Depends on**: Steps 1-2

### Substeps

#### 3a: Move hypnagogia into roko-dreams
1. Copy `roko-golem/src/hypnagogia.rs` → `roko-dreams/src/hypnagogia.rs`
2. Add `pub mod hypnagogia;` to `roko-dreams/src/lib.rs`
3. Update any imports

#### 3b: Move chain_witness into roko-chain
1. Copy `roko-golem/src/chain_witness.rs` → `roko-chain/src/witness.rs`
2. Add `pub mod witness;` to `roko-chain/src/lib.rs`

#### 3c: Update roko-dreams to not depend on roko-golem
1. Remove `roko-golem` from `roko-dreams/Cargo.toml`
2. Replace `pub use roko_golem::{DreamsEngine, ...}` with local definitions
3. Verify: `cargo check -p roko-dreams`

#### 3d: Update roko-learn to not depend on roko-golem
1. Remove `roko-golem` from `roko-learn/Cargo.toml`
2. Replace any re-exports with local alternatives
3. Verify: `cargo check -p roko-learn`

#### 3e: Update roko-serve to not depend on roko-golem
1. Remove `roko-golem` from `roko-serve/Cargo.toml`
2. Replace scaffold feature usage
3. Verify: `cargo check -p roko-serve`

#### 3f: Delete roko-golem
1. Remove from workspace `Cargo.toml` members
2. Delete `crates/roko-golem/` directory
3. Verify: `cargo check --workspace`

### Verification
```bash
! grep -rn 'roko.golem\|roko_golem' crates/ --include='*.toml' --include='*.rs' | grep -v target/
cargo check --workspace
cargo test --workspace
```

---

## Step 4: Delete mortality / death concepts from code

**Type**: Semantic (concept removal)
**Risk**: Low — mortality.rs is already deleted with roko-golem
**Depends on**: Step 3

### Substeps
1. Grep for any remaining references to mortality, death, thanatopsis, vitality in code
2. Remove any remaining references
3. If any test fixtures reference these, update them

### Verification
```bash
! grep -rni 'mortality\|thanatopsis\|vitality_gauge\|death_clock\|necrocracy\|katabasis\|bloodstain' crates/ --include='*.rs' | grep -v target/
```

---

## Step 5: Update workspace metadata

**Type**: Mechanical
**Risk**: None
**Depends on**: None (can run in parallel with anything)

### Substeps
1. Update `Cargo.toml` authors, repository, homepage
2. Update `roko-serve/Cargo.toml` authors

---

## Step 6: Signal → Engram rename (COMPLETED)

**Type**: Mechanical but massive scope
**Risk**: Medium — 18 crates affected
**Depends on**: Steps 1-4 (clean codebase)

### Outcome

- `roko-core/src/signal.rs` was renamed to `roko-core/src/engram.rs`
- `Signal` / `SignalBuilder` were renamed to `Engram` / `EngramBuilder`
- Consumers across the Rust workspace now use `Engram`
- The temporary compat alias was removed after the consumer sweep

### Verification

- `cargo check --workspace`
- `cargo test -p roko-core -- --nocapture`

### File rename
- `roko-core/src/signal.rs` → `roko-core/src/engram.rs`
- Update `mod signal;` → `mod engram;` in lib.rs

---

## Step 7: General code cleanup

**Type**: Cleanup
**Depends on**: Steps 1-6

After all renames and restructuring:
1. Remove any `#[allow(dead_code)]` on things that are actually dead
2. Remove unused feature gates from Cargo.toml files
3. Run `cargo clippy --workspace --no-deps -- -D warnings` and fix
4. Ensure all public items in renamed crates have doc comments
