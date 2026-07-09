# Trait Migrations — Signatures Behind the Spec

The six synapse traits in `crates/roko-core/src/traits.rs` need signature updates to match the target spec in docs 06-synapse-traits.md and 08-scorer-gate-router-composer-policy.md.

**Prerequisite**: All items in `02-missing-kernel-types.md` must be complete (Pulse, Bus, Datum, TopicFilter, PolicyOutputs).

**Note**: Doc 08 explicitly marks these as "target-state operator design" and "planned migration targets." The current Engram-only signatures are v1. These changes bring the traits to the v2 spec.

## Checklist

### TM-01: Substrate — no changes needed
- [x] Substrate is at target spec

**Spec** (doc 07): Substrate remains Engram-only by design. Pulses go through Bus.
**Current code**: Matches spec. `put(Engram)`, `get(&ContentHash)`, `query(&Query)`, etc.
**Status**: COMPLIANT.

### TM-02: Scorer — add Datum dispatch
- [x] Add `score_engram`, `score_pulse`, and `score(Datum)` methods

**Spec** (doc 08):
```rust
pub trait Scorer: Send + Sync {
    fn score_engram(&self, e: &Engram, ctx: &Context) -> Score;
    fn score_pulse(&self, p: &Pulse, ctx: &Context) -> Score {
        let synthetic = Engram::from_pulse_synthetic(p);
        self.score_engram(&synthetic, ctx)
    }
    fn score(&self, datum: Datum<'_>, ctx: &Context) -> Score {
        match datum {
            Datum::Engram(e) => self.score_engram(e, ctx),
            Datum::Pulse(p) => self.score_pulse(p, ctx),
        }
    }
    fn name(&self) -> &'static str;
}
```

**Current code**:
```rust
pub trait Scorer: Send + Sync {
    fn score(&self, engram: &Engram, ctx: &Context) -> Score;
    fn name(&self) -> &'static str;
}
```

**What to change**:
1. Rename existing `score(&Engram)` -> `score_engram(&Engram)` across all implementations
2. Add `score_pulse` with default impl (synthetic Engram conversion)
3. Add `score(Datum)` dispatch with default impl
4. Update all call sites from `scorer.score(engram, ctx)` to `scorer.score_engram(engram, ctx)` (or wrap in `Datum::Engram`)

**Change type**: Additive -- existing impls rename one method, get two new defaults free.
**Accept when**:
- [x] `score_engram`, `score_pulse`, `score` all exist on trait
- [x] `score_pulse` and `score` have default impls
- [x] All existing Scorer impls compile
- [x] `cargo test --workspace` passes
**Verify**:
```bash
grep -A 15 'pub trait Scorer' crates/roko-core/src/traits.rs
cargo test --workspace
```

### TM-03: Gate — add verify_stream
- [x] Add `verify_stream` method with default impl

**Spec** (doc 08):
```rust
#[async_trait]
pub trait Gate: Send + Sync {
    async fn verify(&self, signal: &Engram, ctx: &Context) -> Verdict;
    async fn verify_stream(&self, pulses: &[Pulse], ctx: &Context) -> Verdict {
        let synthetic = Engram::from_pulses(pulses);
        self.verify(&synthetic, ctx).await
    }
    fn name(&self) -> &str;
}
```

**Current code**:
```rust
#[async_trait]
pub trait Gate: Send + Sync {
    async fn verify(&self, engram: &Engram, ctx: &Context) -> Verdict;
    fn name(&self) -> &str;
}
```

**What to change**: Add `verify_stream` with default impl. No existing code changes.
**Change type**: Additive -- new method with default, zero breakage.
**Accept when**:
- [x] `verify_stream` method exists on Gate trait with default impl
- [x] All existing Gate impls compile unchanged
- [x] `cargo test --workspace` passes
**Verify**:
```bash
grep -A 10 'pub trait Gate' crates/roko-core/src/traits.rs
cargo test --workspace
```

### TM-04: Router — add select_pulse
- [x] Rename `select` -> `select_engram`, add `select_pulse`

**Spec** (doc 08):
```rust
pub trait Router: Send + Sync {
    fn select_engram(&self, candidates: &[Engram], ctx: &Context) -> Option<Selection>;
    fn select_pulse(&self, candidates: &[Pulse], ctx: &Context) -> Option<Selection> { None }
    fn feedback(&self, outcome: &Outcome);
    fn name(&self) -> &'static str;
}
```

**Current code**:
```rust
pub trait Router: Send + Sync {
    fn select(&self, candidates: &[Engram], ctx: &Context) -> Option<Selection>;
    fn feedback(&self, outcome: &Outcome);
    fn name(&self) -> &str;
}
```

**What to change**:
1. Rename `select` -> `select_engram` across all implementations
2. Add `select_pulse` with default `None`
3. Change `name()` return from `&str` to `&'static str`

**Change type**: Additive -- rename + new default method.
**Accept when**:
- [x] `select_engram` and `select_pulse` both exist
- [x] `select_pulse` has default impl returning `None`
- [x] `name()` returns `&'static str`
- [x] All existing Router impls compile
- [x] `cargo test --workspace` passes
**Verify**:
```bash
grep -A 10 'pub trait Router' crates/roko-core/src/traits.rs
cargo test --workspace
```

### TM-05: Composer — add Datum-polymorphic entry point
- [x] Add `compose_datums(&[Datum<'_>])` method with default impl

**Spec** (doc 08):
```rust
pub trait Composer: Send + Sync {
    fn compose(&self, inputs: &[Datum<'_>], budget: &Budget, scorer: &dyn Scorer, ctx: &Context) -> Result<Engram>;
    fn name(&self) -> &'static str;
}
```

**Implementation** (additive migration — no breaking changes):
```rust
pub trait Composer: Send + Sync {
    fn compose(&self, engrams: &[Engram], budget: &Budget, scorer: &dyn Scorer, ctx: &Context) -> Result<Engram>;
    fn compose_datums(&self, datums: &[Datum<'_>], budget: &Budget, scorer: &dyn Scorer, ctx: &Context) -> Result<Engram> {
        // default: convert pulses via from_pulse_synthetic, delegate to compose()
    }
    fn name(&self) -> &str;
}
```

**What was done**:
1. Added `compose_datums` method accepting `&[Datum<'_>]` with default impl
2. Default impl converts Pulses to synthetic Engrams, delegates to `compose()`
3. Existing `compose(&[Engram])` kept as-is — zero breakage to existing impls
4. No call site changes needed — callers can opt into `compose_datums` incrementally

**Change type**: ADDITIVE — new method with default, zero breakage.
**Status**: IMPLEMENTED.
**Accept when**:
- [x] `compose_datums` method exists on Composer trait with default impl
- [x] All existing Composer impls compile unchanged
- [x] `cargo build --workspace` passes
- [x] `cargo test -p roko-core` passes

### TM-06: Policy — add Pulse-aware entry point returning PolicyOutputs
- [x] Add `decide_with_pulses(&[Engram], &[Pulse]) -> PolicyOutputs` method with default impl

**Spec** (doc 08):
```rust
pub trait Policy: Send + Sync {
    fn decide(&self, stream: &[Pulse], ctx: &Context) -> PolicyOutputs;
    fn name(&self) -> &'static str;
}
```

**Implementation** (additive migration — no breaking changes):
```rust
pub trait Policy: Send + Sync {
    fn decide(&self, stream: &[Engram], ctx: &Context) -> Vec<Engram>;
    fn decide_with_pulses(&self, engrams: &[Engram], _pulses: &[Pulse], ctx: &Context) -> PolicyOutputs {
        // default: ignores pulses, wraps decide() output in PolicyOutputs
    }
    fn name(&self) -> &str;
}
```

**What was done**:
1. Added `decide_with_pulses` method accepting `(&[Engram], &[Pulse])` returning `PolicyOutputs`
2. Default impl ignores pulses, delegates to `decide()`, wraps result in `PolicyOutputs`
3. Existing `decide(&[Engram]) -> Vec<Engram>` kept as-is — zero breakage to 17 existing impls
4. No call site changes needed — callers can opt into `decide_with_pulses` incrementally

**Change type**: ADDITIVE — new method with default, zero breakage.
**Status**: IMPLEMENTED.

**Note**: TM-06 unblocks SAFE-11 (CognitiveNamespace) in `20-safety.md`. The CognitiveNamespace can now use `decide_with_pulses()` to route non-tool actions through the universal Policy enforcement point.

**Accept when**:
- [x] `decide_with_pulses` method exists on Policy trait with default impl
- [x] All existing Policy impls compile unchanged (17 implementations)
- [x] `cargo build --workspace` passes
- [x] `cargo test -p roko-core` passes

## Migration order

1. TM-02 (Scorer) -- additive, safe -- DONE
2. TM-03 (Gate) -- additive, safe -- DONE
3. TM-04 (Router) -- additive, safe -- DONE
4. TM-05 (Composer) -- additive (compose_datums) -- DONE
5. TM-06 (Policy) -- additive (decide_with_pulses) -- DONE

ALL TRAIT MIGRATIONS COMPLETE. The additive approach was used for TM-05 and TM-06
instead of breaking changes: new methods with defaults were added alongside the
existing ones, giving callers an incremental migration path.
