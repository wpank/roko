# 74 — Unified Evaluation Framework (EvidenceCollector / Criterion / Profile)

**Priority**: P3 — architecture improvement; current gate pipeline works but has structural
limits that compound as roko scales
**Size**: XL (new crate family, three phases of progressive migration; each phase is M)
**Crates**: `crates/roko-eval/` (new), `crates/roko-gate/src/` (bridge + eventual migration)
**Depends on**: None (additive; the existing gate pipeline continues to work throughout)

---

## Background

The gate pipeline in roko works end-to-end and is fully wired into the runner-v2 event loop.
It is not broken. However, it has three structural limitations that will make it harder to
extend as roko's self-hosting workflows become more sophisticated:

1. **Hardcoded gate dispatch.** Adding a new gate today requires editing four files:
   `gate_service.rs`, `rung_dispatch.rs`, the `Rung` enum, and the rung mapping. There is
   no config-driven or user-extensible way to add custom gates.

2. **Evidence and judgment are fused.** Every gate (e.g., `CompileGate`, `TestGate`) is
   responsible for both spawning the subprocess that produces evidence *and* interpreting the
   result. This coupling means: (a) two gates cannot share the same process output; (b)
   evidence cannot be cached across unchanged artifacts; (c) infrastructure failures (process
   crashed) are indistinguishable from evaluation failures (code has errors); (d) gates
   cannot be tested with synthetic evidence without running real subprocesses.

3. **Five disconnected evaluation mechanisms.** The codebase has five separate evaluation
   systems with different score types, different composition rules, and no shared
   infrastructure. There is no unified way to compose, author, share, or learn from
   evaluations across them.

This spec introduces a new `roko-eval` crate that separates evidence collection from
judgment via `EvidenceCollector` and `Criterion` traits. It provides a backward-compatible
bridge so the existing `gate_dispatch.rs` in runner-v2 continues to call `run_gates()`
unchanged throughout the migration.

## Current State

1. **Gate pipeline entry point:** `GateService::gate_for_name()` at
   `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/gate_service.rs` line 59 uses a
   `match` statement mapping gate names to concrete implementations. The current registered
   names are: `"compile"` / `"compile:cargo"` -> `CompileGate`, `"clippy"` /
   `"clippy:cargo"` -> `ClippyGate`, `"test"` / `"test:cargo"` -> `TestGate`,
   `"diff"` / `"diff:git"` -> shell-based diff, `"fmt"` / `"fmt:cargo"` / `"format"` ->
   `FormatCheckGate`, `"judge"` / `"llm-judge"` -> `StubJudgeGate`.

2. **Composition wrappers (preserve these):** `ParallelGate`, `VotingGate`, `FallbackGate`,
   and `ComposedGatePipeline` with four strategies (Sequential, Parallel, Voting, Fallback)
   are well-designed and must be preserved. They live in `crates/roko-gate/src/composition.rs`.

3. **Adaptive thresholds (preserve these):** `AdaptiveThresholds` with EMA pass rates, CUSUM
   shift detection, SPC alerts, and Hotelling joint anomaly detection lives in
   `crates/roko-gate/src/adaptive_threshold.rs`. The runner-v2 feeds verdicts through these;
   they must continue to receive data after the migration.

4. **Runner-v2 integration point:** The only call site that must keep working is
   `crates/roko-cli/src/runner/gate_dispatch.rs`, which calls `run_gates()` on a
   `GateRunner` trait object. The bridge adapter described in Phase 1 keeps this call site
   unchanged.

5. **Existing evaluation mechanisms (five disconnected systems):**

   | Mechanism | Location | Score type |
   |---|---|---|
   | 7-rung gate pipeline | `roko-gate/gate_service.rs` | `Verdict { passed: bool }` |
   | LLM judge gate | `roko-gate/llm_judge_gate.rs` | `f32` [0..1] via `JudgeOracle` |
   | Process reward model | `roko-gate/process_reward.rs` | `f64` [0..1] |
   | Eval generator | `roko-gate/eval_generator.rs` | `Evaluation` struct |
   | Acceptance contract | `roko-gate/acceptance_contract.rs` | `AcceptanceDecision` enum |

6. **No existing EvidenceCollector or Criterion types.** A workspace-wide search confirms
   that `EvidenceCollector`, `EvidenceBag`, and `CriterionResult` do not exist anywhere
   in the codebase. All implementation work starts fresh in the new `roko-eval` crate.

7. **The `Verify` trait** (`async fn verify(&self, signal: &Signal, ctx: &Context) -> Verdict`)
   is the kernel contract in `roko-core`. It must not be removed or changed. The new
   framework operates alongside it; `LegacyCriterion` will wrap `Verify` impls as `Criterion`
   impls.

## Implementation Plan

### Phase 1: Core types + bridge (new `crates/roko-eval/` crate)

**Step 1.** Create `crates/roko-eval/` as a new workspace member. Add it to the workspace
`Cargo.toml` under `[workspace] members`. The initial `Cargo.toml` for the crate needs:
```toml
[dependencies]
roko-core = { path = "../roko-core" }
async-trait = "0.1"
serde = { version = "1", features = ["derive"] }
```

**Step 2.** Define the three core traits in `crates/roko-eval/src/lib.rs`:

```rust
/// Produces typed evidence from artifacts (e.g., runs `cargo check`, captures output).
#[async_trait::async_trait]
pub trait EvidenceCollector: Send + Sync {
    fn kind(&self) -> EvidenceKind;
    async fn collect(&self, artifact: &ArtifactRef, ctx: &EvalContext)
        -> Result<Evidence, CollectError>;
}

/// Scores one dimension of quality given an evidence bag.
pub trait Criterion: Send + Sync {
    fn name(&self) -> &str;
    fn required_evidence(&self) -> &[EvidenceKind];
    fn evaluate(&self, artifact: &ArtifactRef, evidence: &EvidenceBag, ctx: &EvalContext)
        -> CriterionResult;
}

/// Ordered list of criteria with a composition strategy and per-criterion thresholds.
pub struct Profile {
    pub name: String,
    pub criteria: Vec<Box<dyn Criterion>>,
    pub strategy: CompositionStrategy,  // mirrors GateComposition variants
}
```

**Step 3.** Define the data types needed by the traits:
- `ArtifactRef`: path + metadata for the artifact being evaluated (e.g., a workspace root)
- `EvidenceBag`: `HashMap<EvidenceKind, Evidence>` where `Evidence` wraps `(stdout, stderr,
  exit_code, duration_ms)`
- `CriterionResult`: `{ passed: bool, score: f64, findings: Vec<Finding>, duration_ms: u64 }`
- `Finding`: `{ severity: Severity, message: String, file: Option<PathBuf>, line: Option<u32>,
  rule_id: Option<String>, fix_hint: Option<String> }`
- `EvalVerdict`: the top-level result returned by running a `Profile`

All types must derive `Serialize`, `Deserialize`.

**Step 4.** Implement the two initial collectors:
- `ProcessCollector`: spawns a command, captures stdout/stderr/exit code. Used by compile,
  clippy, test, and fmt criteria.
- `DiffCollector`: runs `git diff --stat`, captures output.

**Step 5.** Implement the bridge adapters:
- `LegacyCriterion<V: Verify>`: wraps any existing `Verify` impl as a `Criterion`.
  `required_evidence()` returns `&[]` (no evidence needed; the wrapped gate spawns its own
  subprocess). `evaluate()` calls `v.verify(signal, ctx)` and converts `Verdict` to
  `CriterionResult`.
- `BridgeGateService`: wraps an `EvalService` (which runs `Profile`s) behind the existing
  `GateRunner` trait so that `gate_dispatch.rs` can continue calling `run_gates()` unchanged.
  Initially, the bridge just delegates everything to the legacy `GateService`; it becomes
  meaningful in Phase 2 when migrated criteria are registered.

**Step 6.** Add `roko-eval` to the workspace and verify `cargo build --workspace` passes.
Add at least one unit test for `ProcessCollector` and one for `LegacyCriterion`.

### Phase 2: Migrate code gates to criteria

For each of the seven existing code gates, create a `Criterion` equivalent inside
`crates/roko-eval/src/criteria/`:

| Criterion struct | Migrates | Key change |
|---|---|---|
| `CompileCriterion` | `CompileGate` | Consumes `ProcessCollector` output; reuses `compile_errors.rs` parsing from roko-gate |
| `LintCriterion` | `ClippyGate` | Configurable strict/graduated mode; structured lint `Finding`s |
| `TestCriterion` | `TestGate` | Extracts test names, failure output as structured `Finding`s |
| `FormatCriterion` | `FormatCheckGate` | Lists unformatted file paths as `Finding`s |
| `DiffCriterion` | `DiffGate` | Consumes `DiffCollector` evidence |
| `SymbolCriterion` | `SymbolGate` | Unchanged for now; wraps existing behavior |
| `SecurityCriterion` | `SecurityScanGate` | Emits `info` finding when audit tool is missing |

Each criterion must pass the same correctness test cases as its gate equivalent. The
`BridgeGateService.migrated` set lists which gate names route through criteria; unmigrated
gate names fall back to the legacy `GateService`.

### Phase 3: Registry-driven dispatch

**Step 1.** Create `CriterionRegistry` in `roko-eval` that maps string gate names to
`Box<dyn Criterion>` instances.

**Step 2.** Populate the registry at startup from:
1. Built-in criteria (the seven from Phase 2, registered under the same names as their gate
   equivalents so existing plan `tasks.toml` files need no changes)
2. Config-declared criteria: TOML files discovered under `.roko/criteria/*.toml` at workspace
   startup

**Step 3.** Wire the `CriterionRegistry` into `BridgeGateService`. When `gate_for_name()`
is called, check the registry first; fall back to the legacy `match` in `gate_service.rs`
for any name not in the registry.

**Step 4.** Update `AdaptiveThresholds` to accept `CriterionStats` per criterion name (same
EMA + CUSUM logic, just keyed by criterion name string rather than rung index). Existing per-
rung stats remain for backward compatibility.

### Phase 4 (aspirational): User-authored criteria

Support TOML-defined custom criteria. Two evaluation backends:
- **Shell**: exit 0 = pass, nonzero = fail. Evidence available via env vars.
- **Judge**: LLM evaluation with a rubric and model specification.

Example TOML:
```toml
# .roko/criteria/no-unwrap.toml
[criterion]
name = "no_unwrap"
kind = "deterministic"
severity = "hard"
[criterion.check]
type = "shell"
command = "grep -rn '.unwrap()' --include='*.rs' ${EVAL_ARTIFACT_PATH}/src/ && exit 1 || exit 0"
```

## Acceptance Criteria

### Phase 1
- [ ] `crates/roko-eval/` compiles as a workspace member (`cargo build -p roko-eval`)
- [ ] `EvidenceCollector`, `Criterion`, `Profile` traits defined with correct Rust signatures
- [ ] `EvidenceBag`, `ArtifactRef`, `CriterionResult`, `Finding`, `EvalVerdict` types
  defined and serializable
- [ ] `ProcessCollector` has a unit test that runs a real command and captures stdout/stderr
- [ ] `LegacyCriterion` wraps a mock `Verify` impl and produces a `CriterionResult`
- [ ] `BridgeGateService` wraps `GateRunner` and passes existing gate tests without
  regression: `cargo test -p roko-gate` passes unchanged
- [ ] `roko plan run plans/demo-hello --fresh` completes identically through the bridge

### Phase 2
- [ ] All seven code criteria produce the same pass/fail outcome as their gate equivalents
  on a known test workspace
- [ ] Each criterion accepts `ProcessCollector` evidence rather than spawning its own subprocess
- [ ] `CriterionResult.findings` for `CompileCriterion` carry file paths and line numbers
  parsed from `cargo check` output
- [ ] `BridgeGateService` routes migrated gate names through criteria; unrecognized names
  fall back to legacy `GateService`

### Phase 3
- [ ] `CriterionRegistry` replaces the `gate_for_name()` `match` in `gate_service.rs` as
  the primary dispatch path
- [ ] Custom criteria in `.roko/criteria/*.toml` are discovered and loaded at runner startup
- [ ] Unknown gate names produce a clear error message instead of silently skipping
- [ ] `AdaptiveThresholds` records per-criterion stats preserving existing per-rung data

## Verification Checklist

- [ ] `cargo build --workspace` after adding `roko-eval` to workspace members
- [ ] `cargo test -p roko-eval` passes (Phase 1 unit tests)
- [ ] `cargo test -p roko-gate` passes without regression after bridge is wired
- [ ] `roko plan run plans/demo-hello --fresh` produces the same verdict as before Phase 1
- [ ] After Phase 2: run a plan that exercises compile + clippy + test gates; verify
  `CriterionResult.findings` contain structured source locations
- [ ] After Phase 3: create `.roko/criteria/no-unwrap.toml` with a shell criterion;
  run `roko plan run` and verify the custom criterion fires

## Files to Modify

| File | Change |
|---|---|
| `/Users/will/dev/nunchi/roko/roko/Cargo.toml` | Add `crates/roko-eval` to `[workspace] members` |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-eval/` (new) | New crate: traits, types, collectors, bridge adapters |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/gate_service.rs` | Wire `CriterionRegistry` into `gate_for_name()` in Phase 3 |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/adaptive_threshold.rs` | Extend with `CriterionStats` in Phase 3 |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/gate_dispatch.rs` | No changes needed; the `GateRunner` trait call site is preserved |

## Not in Scope

- Arena-style evaluations (covered by E40)
- Browser/visual evaluation infrastructure
- LLM judge panel methodology (Bradley-Terry, position swap, disjoint families)
- Community marketplace for criteria
- Dashboard visualization of evaluation results
- Training or fine-tuning evaluation models
- Replacing the `Verify` trait in roko-core (it remains the kernel contract)
- Changes to the runner-v2 event loop beyond the `GateRunner` dispatch point
