# cascade_router.rs Refactor — Implementation Prompt

> **Goal**: Split `crates/roko-learn/src/cascade_router.rs` (5,197 lines) into 3-4
> focused modules. Prepare for EFE routing (Phase 1 kernel). Zero behavior change.

## Context

`cascade_router.rs` is the largest file in `roko-learn`. It mixes:
- Routing decision logic (select model given context)
- LinUCB bandit arms (multi-armed bandit for model exploration)
- Model history persistence (JSONL read/write)
- Decay computation (time-weighted decay of observations)
- Confidence scoring (per-model confidence from observation count)
- Bias application (conductor bias, knowledge bias, cost pressure)
- Snapshot serialization
- Explanation generation (human-readable routing rationale)

### Files to read first
```
crates/roko-learn/src/cascade_router.rs  — the 5,197-line file
crates/roko-learn/src/model_router.rs    — legacy router (2,323 lines, may duplicate)
crates/roko-learn/src/runtime_feedback.rs — LearningRuntime that wraps CascadeRouter
tmp/unified/02-CELL.md §2.4              — Route protocol spec (EFE target)
```

---

## Tasks

### CR001 — Map cascade_router.rs structure

**Steps**:
1. Identify all structs, enums, impl blocks with line ranges
2. Categorize: routing logic, bandit arms, persistence, decay, bias, explanation
3. Write map to `tmp/refactoring/cascade-router-map.md`
4. Identify which functions `model_router.rs` duplicates

---

### CR002 — Extract bandit arm management

**Objective**: Move LinUCB arm state, update, and selection to `crates/roko-learn/src/cascade/arms.rs`.

**Moves**: LinUCB struct, arm stats, UCB score computation, arm update after observation.
**Keeps**: CascadeRouter struct and its `.route()` method in the main file.

---

### CR003 — Extract decay and confidence computation

**Objective**: Move to `crates/roko-learn/src/cascade/decay.rs`.

**Moves**: Time-weighted decay functions, confidence scoring, observation aging.

---

### CR004 — Extract persistence (snapshot/restore)

**Objective**: Move to `crates/roko-learn/src/cascade/persistence.rs`.

**Moves**: `save()`, `load()`, JSONL reading/writing, snapshot serialization, atomic writes.

---

### CR005 — Extract explanation generation

**Objective**: Move to `crates/roko-learn/src/cascade/explain.rs`.

**Moves**: `explain_route()`, `CascadeRouteExplanation`, human-readable formatting.

---

### CR006 — Audit model_router.rs for duplication

**Objective**: Determine if `model_router.rs` (2,323 lines) duplicates cascade_router logic.

**Steps**:
1. Compare public APIs — do they serve different callers?
2. If model_router.rs is legacy: mark deprecated, add doc comment pointing to cascade_router
3. If they serve different use cases: document the distinction

---

### CR007 — Prepare for EFE routing

**Objective**: Add a trait abstraction that both LinUCB and future EFE can implement.

**Steps**:
1. Define `trait ModelSelector`:
   ```rust
   pub trait ModelSelector: Send + Sync {
       fn select(&self, candidates: &[ModelCandidate], ctx: &RoutingContext) -> ModelSelection;
       fn observe(&mut self, model: &str, reward: f64);
   }
   ```
2. Make LinUCB implement this trait
3. CascadeRouter uses `Box<dyn ModelSelector>` instead of direct LinUCB
4. Future EFE implementation just needs to impl the same trait

**Verification**:
```bash
cargo test -p roko-learn
cargo clippy -p roko-learn --no-deps -- -D warnings
```

---

## Expected Result

```
crates/roko-learn/src/
  cascade_router.rs     — ~1,500 lines (CascadeRouter struct + route() + bias)
  cascade/
    mod.rs              — re-exports
    arms.rs             — LinUCB bandit arms (~800 lines)
    decay.rs            — time decay + confidence (~500 lines)
    persistence.rs      — save/load/snapshot (~600 lines)
    explain.rs          — explanation generation (~400 lines)
    selector.rs         — ModelSelector trait (~100 lines)
```
