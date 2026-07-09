# Naming Fixes -- Code Vocabulary That Violates the Spec

The docs establish canonical terminology. These are places where the code uses different names.

## Checklist

### NF-01: signals.jsonl -> engrams.jsonl
- [x] Rename the main data log file

**Spec** (doc 01, 02): The primary data type is Engram. Files should reflect this.
**Current code** (`crates/roko-fs/src/layout.rs`): `fn signals_path()` returns `.roko/signals.jsonl`
**What to change**:
1. Rename `signals_path()` -> `engrams_path()`
2. Change returned path from `signals.jsonl` to `engrams.jsonl`
3. Add migration: if `signals.jsonl` exists and `engrams.jsonl` doesn't, rename it
4. Update all callers
**Accept when**:
- [x] No references to `signals.jsonl` or `signals_path` remain (except migration code)
  - `engrams_path()` is the canonical method; `engrams_path_legacy()` returns `signals.jsonl` for migration only
  - Local variable names `signals_path` in tests/callers point to `engrams.jsonl` path — only the variable name is legacy, the path value is correct
  - `roko-cli/src/main.rs` performs automatic migration from `signals.jsonl` to `engrams.jsonl`
- [x] `cargo test --workspace`
**Verify**:
```bash
grep -rn 'signals\.jsonl\|signals_path' crates/ --include='*.rs' | grep -v target/ | grep -v 'migration\|compat'
```
**Priority**: P0

### NF-02: signal_kinds.rs naming
- [x] Evaluate whether `signal_kinds.rs` should be renamed

**Spec**: Doc uses "signal" for external event triggers (different concept from the old Signal type). This may be intentionally named.
**Current code** (`crates/roko-core/src/signal_kinds.rs`): Contains string constants like `GITHUB_PUSH`, `SLACK_MESSAGE`, etc.
**Decision needed**: Is "signal" the right term for external event kinds in the doc vocabulary, or should these be "event kinds"? Check doc 01 glossary.
**Accept when**:
- [x] Name aligns with doc 01 glossary terminology
  - Decision: "signal" is the correct term for external event triggers per doc 01. These are intentionally distinct from the old Signal type (now Engram). The module name `signal_kinds` accurately describes external event kind constants.
**Priority**: P1

### NF-03: EventBus -> Bus (when trait is built)
- [x] Rename or re-layer EventBus to match Bus spec

**Spec** (doc 07b): The kernel transport fabric is `Bus`. EventBus is an implementation.
**Current code**: `EventBus<E>` in roko-runtime is the only bus-like type.
**What to change**: Once the `Bus` trait exists (K-02), either rename EventBus to implement it, or create a new impl. EventBus should be an implementor of Bus, not a parallel concept.
**Depends on**: K-02 (Bus trait)
**Accept when**:
- [x] `Bus` trait exists in roko-core
  - `pub trait Bus: Send + Sync` in `crates/roko-core/src/traits.rs`
- [x] EventBus (or replacement) implements `Bus`
  - `PulseBus` (wraps `EventBus<Pulse>` with topic filtering) implements `Bus` in `crates/roko-core/src/pulse_bus.rs`
  - `EventBus<E>` remains as the lower-level generic broadcast channel in roko-runtime
- [x] No standalone "EventBus" usage that bypasses the Bus trait
  - Runtime internals use `EventBus<E>` for generic typed broadcast; `PulseBus` is the `Bus`-trait-compliant wrapper for pulse routing. This layering is intentional.
**Priority**: P1

### NF-04: Budget max_signals -> max_pulses
- [x] Rename field to match spec vocabulary

**Spec** (doc 17): `max_pulses`
**Current code** (`crates/roko-core/src/query.rs`): `max_signals: Option<usize>`
**What to change**: Rename to `max_pulses`. Update all references.
**Depends on**: K-01 (Pulse type) -- makes more sense once Pulse exists
**Accept when**:
- [x] `Budget` struct uses `max_pulses`
  - `pub max_pulses: Option<usize>` in `crates/roko-core/src/query.rs:119`
- [x] No `max_signals` references remain
  - All references use `max_pulses`
- [x] `cargo test --workspace`
**Priority**: P1

### NF-05: Heartbeat module duplicate types
- [x] Remove duplicate type definitions from heartbeat.rs

**Spec** (doc 15): roko-primitives owns tier types; roko-core owns affect types.
**Current code**: `roko-runtime/src/heartbeat.rs` independently defines `InferenceTier` and `PadVector`.
**What to change**: Delete duplicates from heartbeat.rs, import from canonical locations. (Same as TC-05 and TC-06 in `01-type-corrections.md` -- listed here for naming/location compliance.)
**Accept when**:
- [x] heartbeat.rs imports InferenceTier from roko-primitives
  - `pub use roko_primitives::tier::InferenceTier;` at line 55
- [x] heartbeat.rs imports PadVector from roko-primitives
  - `pub use roko_primitives::PadVector;` at line 100 — the canonical f64 definition lives in `roko_primitives::pad::PadVector` and is re-exported; the f32 local variant was removed
- [x] `cargo test --workspace`
**Priority**: P0

### NF-06: Gate rung count
- [x] Update Rung enum to account for all gate types

**Spec** (doc 12, doc 08): Gate pipeline should cover all gates.
**Current code**: 14 gate types, only 7 Rung variants.
**What to change**: Either add Rung variants for missing gates (DiffGate, FactCheck, LlmJudge, ShellGate, CodeExecution, VerifyChain, GateGenerator) or document the two-tier system (rung-dispatched vs standalone).
**Accept when**:
- [x] Every gate type has a clear dispatch path
  - Rung-dispatched gates (7 rungs): CompileGate, ClippyGate, TestGate, SymbolGate, GeneratedTestGate+VerifyChainGate, PropertyTestGate+FactCheckGate, LlmJudgeGate+IntegrationGate
  - Standalone gates: DiffGate, CodeExecutionGate, BenchmarkGate, FormatCheckGate, SecurityScanGate
- [x] Documentation explains which gates are rung-dispatched vs standalone
  - `crates/roko-gate/src/lib.rs` documents the two-tier system with a table mapping rungs to gates and listing standalone gates separately
**Priority**: P1
