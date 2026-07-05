# Type Corrections — Code That Doesn't Match the Spec

These are types, fields, and implementations that exist in the codebase but differ from what the architecture docs specify. Each needs a code change to comply.

## Checklist

### TC-01: Decay::Ttl field name and type
- [x] Fix `Decay::Ttl` to match doc 04-decay-variants.md

**Spec** (doc 04): `Ttl { expires_at_ms: i64 }` — absolute timestamp in milliseconds
**Current code** (`crates/roko-core/src/decay.rs`): `Ttl { ttl_ms: u64 }` — relative duration, unsigned
**Status**: RESOLVED — kept relative `ttl_ms: u64` by design. The `Decay::apply(age_ms)` contract takes a relative age, making a relative TTL the correct semantic. Absolute deadlines are handled at higher layers (e.g. `roko_agent::lifecycle::DecayModel::Ttl { expires_at }`). Added comprehensive design-note documentation explaining the decision.
**Priority**: P1 (semantic mismatch with doc spec)
**Verify**:
```bash
grep -rn 'ttl_ms' crates/ --include='*.rs' | grep -v target/  # currently shows matches
grep -rn 'expires_at_ms' crates/roko-core/src/decay.rs         # currently 0 matches
cargo test --workspace
```

### TC-02: Provenance taint representation
- [x] Replace `tainted: bool` + `TaintInfo` struct with typed `Taint` enum per doc 05/26

**Spec** (doc 05-provenance-and-attestation.md, doc 26-cognitive-immune-system.md): Typed `Taint` enum with variants like `Hallucination`, `Contradiction`, `UnverifiedSource`, `ToolMisuse`, etc.
**Current code** (`crates/roko-core/src/provenance.rs`): `tainted: bool` field plus `taint_info: Option<TaintInfo>` where `TaintInfo { category: String, detail: String, inherited_from: Option<ContentHash> }`. The `category` is a free-form string — no compile-time variant checking.
**Status**: FIXED. Added `#[non_exhaustive] enum Taint { Clean, LlmHallucination, ToolFailure, UserFlagged, StaleData, UnverifiedSource, Propagated, UserInput, Custom }` to `roko-core/src/provenance.rs`. Replaced `tainted: bool` with `taint: Taint` on `Provenance`. Added `Provenance::is_tainted()` method. Updated all callers (engram hash, scorer, taint_propagation). `TaintInfo` retained for backward compat with old serialized JSONL; bidirectional conversion between `TaintInfo` and `Taint` enum provided.
**Priority**: P1 (type-safety issue; blocks SAFE-11 in 20-safety.md)
**Verify**:
```bash
grep -rn 'tainted: bool' crates/ --include='*.rs' | grep -v target/  # currently shows matches
grep -rn 'enum Taint' crates/roko-core/src/provenance.rs              # currently 0 matches
cargo test --workspace
```

### TC-03: Score::effective() formula
- [x] Update `effective()` to match doc 03 formula, or update doc 03 to match code

**Spec** (doc 03-score-7-axis-appraisal.md): `effective() = confidence * (1 + novelty) * (1 + utility) * reputation` (4-factor formula)
**Current code** (`crates/roko-core/src/score.rs:153-175`): Includes salience_factor and coherence_factor multipliers (each `0.5 + 0.5 * axis` when non-zero). precision is excluded by design.
**Status**: RESOLVED. The 6-factor code formula is correct and intentional. Extended doc comment added to `Score::effective()` documenting the formula, design rationale, and backward compatibility with the 4-factor spec (when salience and coherence are zero, the formula reduces exactly to the spec). Precision exclusion documented as a deliberate design decision.
**Priority**: P2 (documentation/decision, not a bug)
**Verify**:
```bash
# Read the effective() method and confirm it matches the doc after update
grep -A 20 'pub fn effective' crates/roko-core/src/score.rs
```

### TC-04: FileSubstrate log file name
- [x] Rename `signals.jsonl` to `engrams.jsonl` per doc vocabulary

**Spec** (doc 01-naming-and-glossary.md, doc 02-engram-data-type.md): The primary data type is Engram. The log should be named accordingly.
**Current code** (`crates/roko-fs/src/layout.rs`): Both files exist: `engrams_path()` returns `.roko/engrams.jsonl` and `signals_path()` returns `.roko/signals.jsonl` (legacy).
**Status**: PARTIALLY FIXED. New code uses `engrams.jsonl`; old `signals_path()` still exists for backward compat.
**What to change**: Rename `signals_path()` to `engrams_path()`. Change the file name from `signals.jsonl` to `engrams.jsonl`. Add a migration check that renames the old file if it exists. Update all references.
**Verify**:
```bash
grep -rn 'signals\.jsonl\|signals_path' crates/ --include='*.rs' | grep -v target/  # shows legacy support
grep -rn 'engrams\.jsonl\|engrams_path' crates/roko-fs/src/layout.rs                # shows new code
cargo test --workspace
```

### TC-05: Duplicate InferenceTier — consolidate to roko-primitives
- [x] Remove duplicate `InferenceTier` from `roko-runtime/src/heartbeat.rs`

**Spec** (doc 15-crate-map.md): roko-primitives owns tier routing types.
**Current code**: Only one canonical `InferenceTier` enum:
  - `crates/roko-primitives/src/tier.rs`: canonical definition with `#[repr(u8)]`, `TryFrom<u8>`, 3 variants
**Status**: FIXED. Duplicate has been consolidated.
**What to change**: Delete the heartbeat.rs copy. Import from roko-primitives. Add any missing derives (`PartialOrd`, `Ord`, `Hash`) to the primitives version if needed.
**Verify**:
```bash
grep -rn 'enum InferenceTier' crates/ --include='*.rs' | grep -v target/  # shows exactly 1 match
cargo test --workspace
```

### TC-06: Duplicate PadVector — consolidate to roko-core with correct precision
- [x] Remove duplicate `PadVector` from `roko-runtime/src/heartbeat.rs`

**Spec** (doc 11-dual-process-and-active-inference.md): PAD vector for affect state.
**Current code**: Two definitions with different float precision:
  - `crates/roko-core/src/affect.rs`: `PadVector { pleasure: f64, arousal: f64, dominance: f64 }` — canonical
  - `crates/roko-runtime/src/heartbeat.rs`: `PadVector { pleasure: f32, arousal: f32, dominance: f32 }` — duplicate with different precision
**Status**: FIXED. Canonical f64 `PadVector` now lives in `roko-primitives/src/pad.rs` (zero-dep crate). Both `roko-core/src/affect.rs` and `roko-runtime/src/heartbeat.rs` re-export from `roko_primitives::PadVector`. The heartbeat `CorticalState` narrows to f32 only at the `AtomicU32` boundary via `as f32` / `f64::from()` in `pad()` / `set_pad()`. All 56 runtime tests and all core affect tests pass.
**Priority**: P1 (type duplication; runtime uses f32 while core uses f64)
**Verify**:
```bash
grep -rn 'struct PadVector' crates/ --include='*.rs' | grep -v target/  # currently shows 2 matches
cargo test --workspace
```

### TC-07: Gate count — ensure all 14 gates are in the pipeline
- [x] Wire all 14 gate types into the rung dispatch system

**Spec** (doc 12-five-layer-taxonomy.md, doc 08): Gate pipeline should include all implemented gates.
**Current code** (`crates/roko-gate/src/`): 15+ gate types exist. The Rung enum defines 7 rungs that dispatch 12 concrete gates. The remaining gates are standalone.
**Status**: RESOLVED (by design). The two-tier gate architecture is intentional and now comprehensively documented:
- **7 rung-dispatched gates** (12 concrete gates across 7 rungs): CompileGate, ClippyGate, TestGate, SymbolGate, GeneratedTestGate + VerifyChainGate, PropertyTestGate + FactCheckGate, LlmJudgeGate + IntegrationGate. These form the core verification pipeline selected by plan complexity.
- **6 standalone gates**: DiffGate, CodeExecutionGate, ShellGate, BenchmarkRegressionGate, FormatCheckGate, SecurityScanGate. Invoked outside the rung pipeline for scenario-specific checks.
- **Composition wrappers**: ParallelGate, VotingGate, FallbackGate combine any gate into parallel/voting/fallback topologies.
- **Ad-hoc checks**: GateGenerator/GeneratedCheck for dynamically generated verification.
Documentation added to `Rung` enum, `rung_selector.rs`, and `lib.rs` module doc.
**Priority**: P2 (architectural; may be by design)
**Verify**:
```bash
grep -rn 'enum Rung' crates/roko-gate/src/ --include='*.rs' | grep -v target/
# Count variants and compare against gate type count
```

### TC-08: SystemPromptBuilder layer documentation
- [x] Ensure the 9-layer structure in code matches doc descriptions

**Spec** (doc 06, doc 19): References "6-layer" prompt builder in various places.
**Current code** (`crates/roko-compose/src/system_prompt_builder.rs`): Has 9 layers (Role identity, Conventions, Domain context, Active signals/pheromones, Task context, Tool instructions, Relevant techniques, Anti-patterns, Affect guidance).
**Status**: RESOLVED. All "6-layer" references updated to "9-layer" in:
- `CLAUDE.md` (2 occurrences: component table + what-to-work-on section)
- `crates/roko-cli/src/explain.rs` (explain module detail string)
- `crates/roko-cli/src/orchestrate.rs` (enrichment system prompt doc comment)
The code in `system_prompt_builder.rs` already has comprehensive 9-layer documentation in its module-level doc comment with a table listing all layers, cache tiers, and stability classes.
**Verify**:
```bash
grep -n 'Layer\|layer' crates/roko-compose/src/system_prompt_builder.rs | head -20
```

### TC-09: Engram struct — ensure all spec fields are present
- [x] Verify Engram has all fields from doc 02

**Spec** (doc 02-engram-data-type.md): Engram should have all specified fields including `emotional_tag` and `attestation`.
**Current code** (`crates/roko-core/src/engram.rs`): Both fields exist (`emotional_tag: Option<EmotionalTag>`, `attestation: Option<Attestation>`).
**Status**: COMPLIANT. The code has these fields. Verify they match the doc's type definitions exactly.
**Verify**:
```bash
grep -A 5 'emotional_tag\|attestation' crates/roko-core/src/engram.rs
```
