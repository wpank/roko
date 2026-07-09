# Cell Trait + Protocol Renames — Implementation Prompt

> **Goal**: Define the `Cell` trait in `roko-core`, rename 6 existing traits to
> match the unified spec, and make all protocol implementations conform to Cell.
> This is the architectural keystone — everything downstream depends on it.
>
> **Spec**: `tmp/unified/02-CELL.md` §1-2

## Context

The unified spec defines 9 protocols, all unified under a `Cell` trait. Currently
roko has 6 traits with old names:

| Current Name | New Name | Crate | Methods |
|---|---|---|---|
| `Substrate` | `Store` | roko-core/src/traits.rs | put, get, query, prune |
| `Scorer` | `Score` | roko-core/src/traits.rs | score |
| `Gate` | `Verify` | roko-core/src/traits.rs | verify |
| `Router` | `Route` | roko-core/src/traits.rs | route |
| `Composer` | `Compose` | roko-core/src/traits.rs | compose |
| `Policy` | `React` | roko-core/src/traits.rs | apply (→ react) |

Three new protocols are defined but not yet implemented:
- `Observe` (read-only Lens projections)
- `Connect` (external I/O lifecycle)
- `Trigger` (event-driven Graph firing)

### Files to read first
```
crates/roko-core/src/traits.rs           — current 6 trait definitions
tmp/unified/02-CELL.md                   — Cell trait + 9 protocol specs
tmp/unified-migration/02-PHASE-1-KERNEL.md §1.1-1.4 — rename checklist
crates/roko-core/src/lib.rs              — re-exports
```

### Strategy
Use Rust `type` aliases and `pub use` re-exports for backwards compatibility during
transition. The old names continue to work but are marked deprecated. New code uses
new names. Full removal of old names happens in a later pass.

---

## Tasks

### CT001 — Define Cell trait

**File**: `crates/roko-core/src/cell.rs` (create new)

**Steps**:
1. Define the Cell trait per spec (02-CELL.md §1):
   ```rust
   use std::time::Duration;

   pub type CellId = String;  // content-addressed in future
   pub type Version = (u32, u32, u32);
   pub type Cost = f64;

   #[async_trait::async_trait]
   pub trait Cell: Send + Sync + 'static {
       fn id(&self) -> &str;
       fn name(&self) -> &str;
       fn version(&self) -> Version;
       fn protocols(&self) -> &[&str];
       fn estimated_cost(&self) -> Option<Cost> { None }
       fn estimated_duration(&self) -> Option<Duration> { None }
   }
   ```
2. Define `CellContext`:
   ```rust
   pub struct CellContext {
       pub budget_remaining: Cost,
       pub cancel: tokio_util::sync::CancellationToken,
       pub trace_id: String,
   }
   ```
3. Add `pub mod cell;` to `crates/roko-core/src/lib.rs`
4. Re-export: `pub use cell::{Cell, CellContext, CellId, Cost, Version};`

**Verification**: `cargo check -p roko-core`

---

### CT002 — Rename Substrate → Store

**Steps**:
1. In `crates/roko-core/src/traits.rs`:
   - Rename `pub trait Substrate` to `pub trait Store`
   - Add backwards compat: `#[deprecated(note = "renamed to Store")] pub use Store as Substrate;`
   - Or create type alias in a compat module
2. In `crates/roko-core/src/lib.rs`: update re-exports
3. Search all crates for `Substrate` usage:
   ```bash
   grep -rn 'Substrate' crates/ --include='*.rs' | grep -v target/ | grep -v '_legacy'
   ```
4. Update each usage to `Store` (or leave with deprecation warning for now)
5. The main implementor is `crates/roko-fs/src/` — rename there too

**Verification**: `cargo check --workspace`

---

### CT003 — Rename Scorer → Score

**Steps**: Same pattern as CT002.
- Rename trait in traits.rs
- Update implementations (likely in roko-learn, roko-compose)
- Backwards compat alias

---

### CT004 — Rename Gate → Verify

**Steps**: Same pattern. This is the most impactful rename — `roko-gate` crate has
11+ Gate implementations.

**Additional**: Add `verify_pre()` and `verify_post()` method stubs to the trait with
default implementations that call the existing `verify()`:
```rust
pub trait Verify: Cell {
    async fn verify(&self, ...) -> Verdict;

    // New methods with defaults for backwards compat
    async fn verify_pre(&self, input: &[Signal], ctx: &VerifyContext) -> Verdict {
        Verdict::pass()  // default: don't veto
    }
    async fn verify_post(&self, input: &[Signal], output: &[Signal], ctx: &VerifyContext) -> Verdict {
        self.verify(/* ... */).await
    }
}
```

---

### CT005 — Rename Router → Route

**Steps**: Same pattern. Main implementor is CascadeRouter in roko-learn.

---

### CT006 — Rename Composer → Compose

**Steps**: Same pattern. Main implementor in roko-compose.

---

### CT007 — Rename Policy → React (breaking change)

**Steps**: This is the only rename that changes method signatures. The spec says React
operates on Pulses (ephemeral), not Signals (durable).

For now: just rename the trait. Method signature change (Engram → Pulse input) happens
in a separate task after Pulse/Bus kernel is ready.

---

### CT008 — Define Observe trait (new)

**File**: `crates/roko-core/src/cell.rs` (or new `observe.rs`)

```rust
#[async_trait]
pub trait Observe: Cell {
    async fn observe(&self, ctx: &CellContext) -> Result<Vec<Signal>>;
}
```

No implementations yet — just the trait definition. Implementations (Lenses) come in
a later phase.

---

### CT009 — Define Connect trait (new)

```rust
#[async_trait]
pub trait Connect: Cell {
    async fn connect(&self) -> Result<()>;
    async fn health(&self) -> HealthStatus;
    async fn disconnect(&self) -> Result<()>;
}
```

No implementations yet.

---

### CT010 — Define Trigger trait (new)

```rust
#[async_trait]
pub trait Trigger: Cell {
    async fn arm(&self, ctx: &CellContext) -> Result<()>;
    async fn disarm(&self) -> Result<()>;
    async fn poll(&self) -> Option<TriggerEvent>;
}
```

No implementations yet.

---

### CT011 — Update all trait implementations to also impl Cell

**Steps**:
1. For each Gate/Verify implementation in roko-gate (11+ structs):
   - Add `impl Cell for CompileGate { ... }` with name/version/protocols
2. For CascadeRouter: `impl Cell for CascadeRouter { ... }`
3. For PromptComposer: `impl Cell for PromptComposer { ... }`
4. For existing Policy implementations
5. For FileSubstrate in roko-fs

This is mechanical: each impl just returns static metadata.

**Verification**:
```bash
cargo check --workspace
cargo test --workspace
```

---

### CT012 — Add deprecation warnings and migration guide

**Steps**:
1. Create `crates/roko-core/src/compat.rs` with deprecated re-exports:
   ```rust
   #[deprecated(since = "0.2.0", note = "renamed to Store")]
   pub type Substrate = super::Store;
   // ... etc for all 6 renames
   ```
2. Add doc comments to new trait names explaining the rename
3. Create `tmp/unified-migration-runner/RENAME-GUIDE.md` documenting all renames

**Verification**:
```bash
cargo check --workspace  # should compile with deprecation warnings only
```
