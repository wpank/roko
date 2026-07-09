# Batch Execution Contract

8 batches ordered for unattended execution. The goal is not just to "cover the verification docs", but to let an agent turn parity findings into bounded work that can run overnight without guessing what the live runtime contract is.

---

## Batch Posture

- Default strategy: **make the shipped verification runtime honest before expanding it**.
- Treat `crates/roko-cli/src/orchestrate.rs` as the primary conflict hotspot.
- Treat `crates/roko-gate/src/rung_selector.rs`, `gate_pipeline.rs`, `adaptive_threshold.rs`, `feedback.rs`, `ratchet.rs`, and `artifact_store.rs` as the supporting contract modules.
- If a task starts requiring reward-model design, autonomous evaluator-agent architecture, or replay-analysis product design, record the seam and stop.
- Every completed batch should leave behind:
  - code changes,
  - verification command output,
  - explicit deferrals,
  - and any newly clarified runtime contract.

Required reads for every batch:

- [00-INDEX.md](00-INDEX.md)
- the owning section file(s) named below
- [SOURCE-INDEX.md](SOURCE-INDEX.md)
- [context-pack/agent-runbook.md](context-pack/agent-runbook.md)
- [context-pack/carry-forward-map.md](context-pack/carry-forward-map.md)

---

## Recommended Serial Order

For a single long-running agent run, prefer:

`V1 -> V2 -> V3 -> V4 -> V5 -> V6 -> V7 -> V8`

This order first makes the runtime contract explicit, then turns selector / threshold / feedback policy into live behavior, then persists long-running state, and only then widens the higher-rung or signal-contract surface.

---

## Batch Overview

| Batch | Tasks | Purpose | Primary Write Scope | Verify Focus | Est. LOC |
|-------|-------|---------|---------------------|--------------|----------|
| V1 | B.04, A.04 | Replace ad-hoc rung numbering with one canonical runtime dispatch contract | `roko-cli` orchestrator gate helpers | `cargo test -p roko-cli -p roko-gate` | 180 |
| V2 | B.05-B.10 | Activate `select_rungs` + `GatePipeline` on a production path | `roko-cli`, `roko-gate` selector/pipeline seam | `cargo test -p roko-cli -p roko-gate` | 260 |
| V3 | D.03, D.04 | Make adaptive thresholds affect retries and skip decisions | `roko-cli` retry / gate-run path | `cargo test -p roko-cli -p roko-gate` | 140 |
| V4 | D.08-D.10 | Feed structured gate feedback into AutoFix | `roko-cli`, `roko-gate::feedback` callsites | `cargo test -p roko-cli -p roko-gate` | 180 |
| V5 | C.05, C.10 | Persist verification artifacts to disk with explicit content-addressing | `roko-gate`, `roko-cli` artifact write path | `cargo test -p roko-cli -p roko-gate` | 220 |
| V6 | C.08, C.09 | Activate and persist `GateRatchet` for long-running convergence | `roko-gate::ratchet`, `roko-cli` | `cargo test -p roko-cli -p roko-gate` | 180 |
| V7 | remaining B.04 reachability, F.01, F.05 | Make high-rung gates reachable only when runtime inputs actually exist | `roko-cli`, `roko-gate`, capability discovery | `cargo test -p roko-cli -p roko-gate` | 260 |
| V8 | G.08, G.10 | Harden verdict-signal decay / tags and add chain-integrity verification | `roko-core`, `roko-cli` signal path | `cargo test -p roko-core -p roko-cli` | 160 |

---

## Dependency Graph

| Batch | Depends on |
|-------|------------|
| V1 | — |
| V2 | V1 |
| V3 | V2 |
| V4 | V2 |
| V5 | V2 |
| V6 | V2 |
| V7 | V2, V5 |
| V8 | V2 |

Why `V1 -> V2`:

- the runtime needs one honest notion of “rung” before selector-driven activation can be trusted.

Why `V2 -> V3`:

- adaptive skip / retry policy should act on the real selector path, not on the current hardcoded one.

Why `V2 -> V4`:

- structured feedback is more useful once the live gate path and rung semantics are less misleading.

Why `V5 -> V7`:

- generated or artifact-backed verification paths are easier to activate after artifacts have a persistent runtime home.

Parallel-safe groups:

- `{V1}` should land first.
- `{V3, V4, V5, V6}` can proceed after `V2`.
- `V7` should wait for `V5`.
- `V8` should wait for `V2`.

Conflict groups:

| Group | Crates / Files | Batches |
|-------|----------------|---------|
| orchestrate-core | `crates/roko-cli/src/orchestrate.rs` | V1, V2, V3, V4, V5, V6, V7, V8 |
| gate-selection | `crates/roko-gate/src/rung_selector.rs`, `gate_pipeline.rs` | V1, V2, V3, V7 |
| gate-feedback | `crates/roko-gate/src/feedback.rs`, autofix prompt path | V4 |
| artifacts-ratchet | `crates/roko-gate/src/artifact_store.rs`, `ratchet.rs`, persistence helpers | V5, V6, V7 |
| signal-contract | `crates/roko-core/src/engram.rs`, `decay.rs`, `crates/roko-cli/src/orchestrate.rs`, CLI command surfaces | V8 |

---

## Batch Details

### V1 — Canonical Runtime Rung Contract

**Owns**: `B.04`, `A.04`

**Read first**:
- [B-pipeline-rungs.md](B-pipeline-rungs.md)
- [A-gate-foundation.md](A-gate-foundation.md)

**Problem**: the live orchestrator path still treats `rung: u32` as an ad-hoc numeric convention with a swapped `Test` / `Lint` mapping and an `_ => all three` fallback. Later agents cannot safely reason about verification behavior from the docs or from `Rung`.

**Scope**:
1. Replace the raw numeric `match` with one explicit translation layer between runtime inputs and canonical rung semantics.
2. Eliminate the current `1 => Test`, `2 => Clippy` mismatch relative to `Rung`.
3. Centralize gate construction in one discoverable helper or registry.
4. Preserve current post-merge behavior only if it is made explicit rather than hidden behind `_`.
5. Add tests proving the runtime mapping is no longer ambiguous.

**Out of scope**:
- `select_rungs(...)` production activation,
- adaptive threshold policy,
- artifact persistence,
- reward-model work.

**Files**:
- `crates/roko-cli/src/orchestrate.rs`
- new helper module if the registry should live outside `orchestrate.rs`

**Verify**:
```bash
cargo test -p roko-cli -p roko-gate
rg -n "run_gate_rung|Rung::|ClippyGate|TestGate|CompileGate" crates/roko-cli crates/roko-gate
```

**Acceptance criteria**:
- one obvious runtime contract explains what a requested verification rung means,
- swapped numeric semantics are gone,
- later agents do not need to reverse-engineer `_ => all three`.

---

### V2 — Production Selector + GatePipeline Activation

**Owns**: `B.05`, `B.06`, `B.07`, `B.08`, `B.09`, `B.10`

**Read first**:
- [B-pipeline-rungs.md](B-pipeline-rungs.md)

**Problem**: `select_rungs`, `prior_failures`, and `GatePipeline` are real library code with tests, but the production path still bypasses them.

**Scope**:
1. Give `select_rungs(...)` a real orchestrator caller.
2. Thread one production notion of plan complexity into that call.
3. Use `GatePipeline` rather than hand-assembling verdict vectors in the main path.
4. Make short-circuiting, aggregation, and test-count merging observable from tests or dry-run behavior.
5. Make failure-escalation inputs explicit and inspectable.

**Out of scope**:
- adaptive-threshold skip decisions,
- ratchet persistence,
- autonomous test generation,
- gate algebra from doc 03 §13.

**Files**:
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-gate/src/rung_selector.rs` if runtime helpers need exposing
- `crates/roko-gate/src/gate_pipeline.rs` only if contract changes are required

**Verify**:
```bash
cargo test -p roko-cli -p roko-gate
rg -n "select_rungs\\(|GatePipeline::new|with_short_circuit|merge_test_count" crates/roko-cli crates/roko-gate
```

**Acceptance criteria**:
- `select_rungs` has a non-test production caller,
- `GatePipeline` has a production caller,
- escalation inputs are explicit rather than implicit,
- runtime behavior lines up with the canonical rung contract from `V1`.

---

### V3 — Adaptive Threshold Read-Side Activation

**Owns**: `D.03`, `D.04`

**Read first**:
- [D-feedback-thresholds.md](D-feedback-thresholds.md)
- [B-pipeline-rungs.md](B-pipeline-rungs.md)

**Problem**: `AdaptiveThresholds` currently updates itself from real gate runs, but runtime retry and skip logic ignore the stored policy.

**Scope**:
1. Use `suggested_max_retries(rung)` in one live retry path.
2. Use `should_skip_rung(rung)` when assembling or filtering production gate runs.
3. Keep cold-start behavior explicit so low-observation plans do not become unstable.
4. Add enough logging or test evidence that later agents can see why a rung was skipped or a retry cap changed.

**Out of scope**:
- SPC detector stack,
- dashboard-only presentation work,
- reward-model gating.

**Files**:
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-gate/src/adaptive_threshold.rs` only if small API helpers are needed

**Verify**:
```bash
cargo test -p roko-cli -p roko-gate
rg -n "suggested_max_retries|should_skip_rung" crates/roko-cli crates/roko-gate
```

**Acceptance criteria**:
- adaptive thresholds affect runtime, not just dashboards,
- skip and retry decisions are inspectable,
- cold-start fallback remains deterministic.

---

### V4 — Structured Gate Feedback In AutoFix

**Owns**: `D.08`, `D.09`, `D.10`

**Read first**:
- [D-feedback-thresholds.md](D-feedback-thresholds.md)

**Problem**: the highest-signal retry-feedback module in the verification stack is not used by the actual autofix prompt path.

**Scope**:
1. Replace or wrap raw gate-output injection with `feedback_for_agent(...)`.
2. Preserve structured `errors`, `warnings`, and `suggestions` in the autofix prompt.
3. Keep raw verbose output available elsewhere if needed for debugging or artifacts.
4. Add tests or fixtures proving the prompt path now uses classified feedback.

**Out of scope**:
- multi-line diagnostic grouping,
- SARIF / JSON parsing,
- per-build-system classifier redesign.

**Files**:
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-gate/src/feedback.rs` only if formatting helpers should live there

**Verify**:
```bash
cargo test -p roko-cli -p roko-gate
rg -n "feedback_for_agent|last_gate_failure|handle_autofix" crates/roko-cli crates/roko-gate
```

**Acceptance criteria**:
- AutoFix consumes structured feedback on a production path,
- errors / warnings / suggestions remain distinguishable,
- the patch does not merely move raw detail around under a new name.

---

### V5 — Persistent Artifact Store

**Owns**: `C.05`, `C.10`

**Read first**:
- [C-artifacts-ratcheting.md](C-artifacts-ratcheting.md)
- [G-forensic-verdict-signals.md](G-forensic-verdict-signals.md)

**Problem**: content-addressed artifact storage exists only in-memory, which blocks long-running or resumed verification runs from benefiting from prior artifacts.

**Scope**:
1. Add a disk-backed artifact layout under `.roko/artifacts/`.
2. Keep content-addressing authoritative; do not introduce duplicate mutable IDs.
3. Persist enough metadata that stored artifacts are attributable to plan / gate / time.
4. Wire at least one real runtime artifact producer into that store.
5. Make the read path discoverable enough for later high-rung activation.

**Out of scope**:
- sophisticated garbage collection,
- replay analytics UI,
- full artifact browser tooling.

**Files**:
- `crates/roko-gate/src/artifact_store.rs` or adjacent persistence helper
- `crates/roko-cli/src/orchestrate.rs`

**Verify**:
```bash
cargo test -p roko-cli -p roko-gate
rg -n "\\.roko/artifacts|manifest|ContentHash::of|ArtifactStore" crates/roko-cli crates/roko-gate
```

**Acceptance criteria**:
- artifacts survive process boundaries,
- disk layout remains content-addressed,
- at least one production artifact writer exists,
- later agents can find the stored artifact by hash or metadata.

---

### V6 — GateRatchet Runtime Activation + Persistence

**Owns**: `C.08`, `C.09`

**Read first**:
- [C-artifacts-ratcheting.md](C-artifacts-ratcheting.md)

**Problem**: `GateRatchet` is meant to prevent convergence thrashing, but no runtime path records or consults it, and its state dies on exit.

**Scope**:
1. Load and save ratchet state on the orchestrator path.
2. Call `record_pass(plan_id, rung)` on successful verification progress.
3. Consult `can_regress(plan_id, rung)` where a lower-rung rerun or fallback would otherwise happen.
4. Leave behind telemetry or logs that explain when ratchet state changed.

**Out of scope**:
- redesigning the overall retry state machine,
- deleting `GateRatchet` as dead code unless runtime activation proves impossible,
- reward-model escalation logic.

**Files**:
- `crates/roko-gate/src/ratchet.rs`
- `crates/roko-cli/src/orchestrate.rs`

**Verify**:
```bash
cargo test -p roko-cli -p roko-gate
rg -n "GateRatchet|record_pass|can_regress|gate-ratchet" crates/roko-cli crates/roko-gate
```

**Acceptance criteria**:
- ratchet state changes on real runtime progress,
- ratchet state survives restart / resume,
- regressions are explicitly prevented or explained.

---

### V7 — High-Rung Gate Reachability

**Owns**: remaining `B.04` reachability, `F.01`, `F.05`

**Read first**:
- [B-pipeline-rungs.md](B-pipeline-rungs.md)
- [F-autonomous-evoskills.md](F-autonomous-evoskills.md)
- [C-artifacts-ratcheting.md](C-artifacts-ratcheting.md)

**Problem**: even after selector activation, the higher-rung gates are only useful if runtime capability discovery can tell when their required inputs actually exist.

**Scope**:
1. Make capability detection (`RungCaps` or equivalent) reflect real runtime inputs.
2. Activate higher-rung gates only when their constructors can be satisfied honestly.
3. Prefer explicit “cap unavailable” behavior over silent reachability claims.
4. Add tests or dry-run evidence for at least one non-basic higher-rung path.

**Out of scope**:
- building a test-generator agent,
- adversarial generation / implementation separation,
- full EvoSkills architecture.

**Files**:
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-gate/src/generated_test_gate.rs`
- `crates/roko-gate/src/property_test_gate.rs`
- `crates/roko-gate/src/integration_gate.rs`
- any capability-discovery helper

**Verify**:
```bash
cargo test -p roko-cli -p roko-gate
rg -n "GeneratedTestGate|PropertyTestGate|IntegrationGate|has_generated_tests|has_property_tests|has_integration_scenario" crates/roko-cli crates/roko-gate
```

**Acceptance criteria**:
- higher-rung reachability is real rather than implied,
- unavailable capabilities are explicitly reported,
- the batch does not claim “autonomous eval generation” exists when only the gate consumer exists.

---

### V8 — Verdict Signal Contract Hardening

**Owns**: `G.08`, `G.10`

**Read first**:
- [G-forensic-verdict-signals.md](G-forensic-verdict-signals.md)
- [D-feedback-thresholds.md](D-feedback-thresholds.md)

**Problem**: `Kind::GateVerdict` signals are emitted, but the emitted engrams do not yet match the contract the docs describe for decay, tags, and integrity checking.

**Scope**:
1. Make verdict-engram decay explicit if the runtime contract intends a 24h half-life.
2. Propagate the critical tags (`plan_id`, `task_id`, `gate`, `passed`, rung or equivalent) explicitly rather than relying on assumptions.
3. Add a chain-integrity check path for stored signals.
4. Keep the result narrow: contract hardening, not a full replay engine.

**Out of scope**:
- causal replay assembly,
- root-cause or what-if analysis,
- predictive gate selection,
- scorer/router/composer/dreams consumption design.

**Files**:
- `crates/roko-core/src/engram.rs`
- `crates/roko-core/src/decay.rs`
- `crates/roko-cli/src/orchestrate.rs`
- CLI command surface if a `verify-chain` entrypoint is added

**Verify**:
```bash
cargo test -p roko-core -p roko-cli
rg -n "Kind::GateVerdict|Decay::HalfLife|lineage|signals.jsonl|verify-chain" crates/roko-core crates/roko-cli
```

**Acceptance criteria**:
- verdict engrams have an explicit, inspectable contract,
- chain integrity can be checked offline,
- the batch stops short of inventing a full replay-analysis product.
