# B — Pipeline & Rungs (Docs 02, 03)

Parity analysis of `docs/04-verification/02-6-rung-selector.md` and
`docs/04-verification/03-gate-pipeline.md` vs the actual codebase.

---

## B.01 — `PlanComplexity` 4-tier enum

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 02 §3 — 4 complexity levels (Trivial/Simple/Standard/Complex) with `escalate()` that saturates at Complex.
**Reality**: `crates/roko-gate/src/rung_selector.rs:24-34` defines `PlanComplexity` enum with exactly those 4 variants. `escalate()` at `rung_selector.rs:36-45` matches doc's pseudocode. `escalate_by(n: u32)` helper at `rung_selector.rs:47-55` for escalation over multiple failures.

---

## B.02 — 7-rung enum with `CANONICAL_ORDER`

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 02 §2 — 7 rungs (Compile/Lint/Test/Symbol/GeneratedTest/PropertyTest/Integration) numbered 0–6.
**Reality**: `crates/roko-gate/src/rung_selector.rs:62-80` defines `Rung` enum with exactly those 7 variants and `#[repr(u8)]` discriminants 0–6. `CANONICAL_ORDER: [Rung; 7]` const at `rung_selector.rs:83-91` pins the execution order. `Rung::label()` at lines 93-107 gives TUI/logging labels.

---

## B.03 — `RungCaps` + `select_rungs()` pure function

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 02 §4, §5 — caps narrow the complexity-selected rungs; `select_rungs(complexity, caps, prior_failures) -> Vec<Rung>` is pure.
**Reality**:
- `RungCaps` struct at `rung_selector.rs:117-128` with 5 bools: `has_lint_tool`, `has_symbol_manifest`, `has_generated_tests`, `has_property_tests`, `has_integration_scenario`. Note: `compile` and `test` are not caps because they're always available.
- `select_rungs()` at `rung_selector.rs:207-214`. Pipeline: escalate complexity by `prior_failures` → fetch `base_rungs` → filter by caps → return sorted.
- `base_rungs()` decision table at `rung_selector.rs:168-189` matches doc §3.1 exactly.

---

## B.04 — Hardcoded gate dispatch in `run_gate_rung()` (BIGGEST GAP)

**Status**: PARTIAL (HIGH severity)
**Doc claim**: Docs 02 and 03 claim 7 rungs, each mapped to its own gate (Compile→`CompileGate`, Lint→`ClippyGate`, Test→`TestGate`, Symbol→`SymbolGate`, GeneratedTest→`GeneratedTestGate`, PropertyTest→`PropertyTestGate`, Integration→`IntegrationGate`). Doc 02 §6 claims: "The orchestrator maps each `Rung` to a concrete gate implementation and feeds them into a `GatePipeline`."
**Reality**: `crates/roko-cli/src/orchestrate.rs:11423-11461` is a literal match on rung numbers — and only instantiates `CompileGate`, `TestGate`, `ClippyGate`:

```rust
async fn run_gate_rung(payload_sig: &Engram, rung: u32) -> Vec<Verdict> {
    match rung {
        0 => /* CompileGate */,
        1 => /* TestGate */,   // doc says Lint
        2 => /* ClippyGate */, // doc says Test
        _ => /* all three */,
    }
}
```

**Severe consequences**:
1. `SymbolGate` (1002 LOC, full impl) unreachable from `roko plan run`.
2. `GeneratedTestGate` (820 LOC, full impl) unreachable.
3. `PropertyTestGate` (695 LOC, full impl) unreachable.
4. `IntegrationGate` (803 LOC, full impl) unreachable.
5. `DiffGate` (357 LOC, full impl) unreachable.
6. Rung numbers in orchestrate (1=Test, 2=Clippy) are **swapped** relative to `Rung` enum (1=Lint, 2=Test).
7. `GatePipeline` exists at `gate_pipeline.rs:36-178` with real short-circuit + aggregation logic, but is **not used** — `run_gate_rung` returns a `Vec<Verdict>` directly.
8. `select_rungs()` has no caller that feeds into `run_gate_rung`.

**Fix sketch**: Replace the hardcoded match with a `Vec<Box<dyn Gate>>` built from `RungSelector::select_rungs()` output. Map `Rung::Compile → CompileGate`, `Rung::Lint → ClippyGate`, `Rung::Test → TestGate`, `Rung::Symbol → SymbolGate`, etc. Feed into `GatePipeline::new(name).with_gate(...)`. This is the **single most important wiring gap** in the verification layer.

---

## B.05 — Escalation on failure

**Status**: PARTIAL (HIGH severity)
**Doc claim**: Doc 02 §3.2, §8 — `prior_failures` arg to `select_rungs` escalates complexity; failed attempts accumulate, widening verification on retry.
**Reality**: `escalate_by(n)` and `select_rungs(complexity, caps, prior_failures)` both exist at `rung_selector.rs:47-55, 207-214`. **But**: no call site in `orchestrate.rs` wires prior failure counts into `select_rungs` — the hardcoded dispatch bypasses selection entirely (see B.04). Escalation as doc describes it does not happen at runtime.
**Fix sketch**: When B.04 is fixed, the call site should track prior failures per plan (via `task_trackers` or `plan_state`) and feed into `select_rungs` to produce the gate vec.

---

## B.06 — `GatePipeline` sequential composition

**Status**: DONE (but unused from orchestrator)
**Severity**: HIGH
**Doc claim**: Doc 03 §2, §5 — `GatePipeline` wraps `Vec<Box<dyn Gate>>`, itself implements `Gate`, has builder API.
**Reality**: `crates/roko-gate/src/gate_pipeline.rs:36-96` defines the struct. `new(name)` constructor, `push(gate)`, `with_gate(gate)` chaining, `with_short_circuit()` / `without_short_circuit()`, `len()`, `is_empty()`. `impl Gate for GatePipeline` at `gate_pipeline.rs:145+`. Matches doc §2, §5 exactly. **But**: there are no production callers — it's called only from its own tests. The hardcoded dispatch in orchestrate.rs bypasses it (see B.04).

---

## B.07 — Short-circuit semantics + skipped-step accounting

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 03 §3 — default `short_circuit = true`; on first failure, stop; detail should record remaining gates as skipped.
**Reality**: `gate_pipeline.rs:161-180` — on failure with `short_circuit`, appends `"[skip] {gate} (short-circuit)"` lines and breaks. This behavior is not surfaced in orchestrate because orchestrate bypasses GatePipeline entirely.

---

## B.08 — Verdict aggregation (pass/reason/detail/duration)

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 03 §4 — pipeline passes iff every gate passes; reason lists failed gates; detail joined with headers; duration is sum.
**Reality**: `gate_pipeline.rs:146-210` implements all of this:
- `passed = failed_names.is_empty()` (pass iff all pass)
- `reason = "gate pipeline failed: {names with reasons}"`
- `detail_lines` joined with per-step render at `render_step_line()` (gate_pipeline.rs:129-142)
- `duration_ms = elapsed_ms(started)` — wall-clock sum by construction of sequential execution

Doc §4.2 says detail lines are separated by `--- [name] ---` headers. Code uses `N. [pass|fail] name (ms ms)` format. Same information, different cosmetics. **Minor doc-vs-code drift**.

---

## B.09 — `TestCount` merging across inner gates

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 03 §4.5 — when multiple gates produce `TestCount`, pipeline sums passed/failed/ignored.
**Reality**: `merge_test_count(acc, next)` at `gate_pipeline.rs:115-126` with `saturating_add` guards. Returns `None` iff both sides are `None`. Exactly what doc claims.

---

## B.10 — `PlanComplexity` → rung maximum mapping

**Status**: DONE (but see B.04 — ignored at runtime)
**Severity**: —
**Doc claim**: Doc 02 §3.1 table: Trivial → 0; Simple → 1; Standard → 3; Complex → 6.
**Reality**: Code at `rung_selector.rs:168-189` differs from doc's mapping subtly:
- Trivial → `[Compile, Test]` (doc: "Compile only")
- Simple → `[Compile, Lint, Test, Symbol]` (doc: "Compile + Lint" only)
- Standard → `[Compile, Lint, Test, Symbol, GeneratedTest]` (doc: "Compile + Lint + Test + Symbol")
- Complex → all 7 (matches doc)

So doc 02's rationale text ("Compile only", "Compile + Lint") does not match code's actual mapping ("Compile + Test", "Compile + Lint + Test + Symbol"). The ordering and inclusion differ noticeably.
**Notes**: This is a doc-over-description drift, not a code bug. Code's mapping is defensible (Trivial still runs tests). Worth noting since a reader of doc 02 would implement incorrectly if cargo-culting from the doc.

---

## B.11 — Short-circuit + `GatePipeline::is_empty()` edge case

**Status**: DONE
**Severity**: —
**Doc claim**: Implicit in doc 03 §2 — empty pipeline behavior.
**Reality**: `gate_pipeline.rs:150-154` — empty pipeline returns `Verdict::pass` with `detail = "GatePipeline: no inner gates"`. Tested in `pipeline_empty_passes`.

---

## B.12 — Gate algebra (Sequential/Parallel/Fallback/Voting/Weighted/Threshold) from doc 03 §13

**Status**: NOT DONE (LOW severity)
**Doc claim**: Doc 03 §13 sketches a full combinator algebra: `Parallel`, `Fallback`, `Voting`, `Weighted`, `Threshold`; doc §14 adds `ProbabilisticVerdict` with Wilson intervals; doc §15 adds `ProgressivePipeline`; doc §16 adds `GateMetrics`/`PipelineMetrics`.
**Reality**: Only `GatePipeline` (the sequential combinator) exists. No `Parallel`, `Fallback`, `Voting`, `Weighted`, `Threshold`, `ProbabilisticVerdict`, `SequentialPropertyGate`, `FuzzGate`, `ProgressivePipeline`, `ProgressivePhase`, `GateMetrics`, or `PipelineMetrics`.

Grep verification: `grep -rn 'Parallel\|Fallback\|Voting\|ProgressivePipeline\|ProgressivePhase\|ProbabilisticVerdict\|SequentialPropertyGate\|FuzzGate' crates/roko-gate/src/` returns nothing relevant to gates.

**Fix sketch**: Doc 03 §13–§17 should be explicitly marked "design — not started". This is ~600 lines of speculative design that a reader could mistake for current architecture.

---

## Section Summary

| Status | Count |
|--------|-------|
| DONE | 9 |
| PARTIAL | 2 (B.04 hardcoded dispatch, B.05 escalation bypass) |
| NOT DONE | 1 (B.12 gate algebra) |

The pipeline abstractions (`PlanComplexity`, `Rung`, `CANONICAL_ORDER`, `RungCaps`, `select_rungs`, `GatePipeline`, short-circuit, aggregation, test-count merging) are **fully built and tested in `roko-gate`** — but **not called from orchestrate.rs**. The hardcoded `run_gate_rung()` match (B.04) is the #1 wiring gap in the entire verification layer: it reduces 13 gate implementations to 3 (Compile/Test/Clippy) and reduces 7 rungs to 3. Fixing B.04 unlocks SymbolGate, GeneratedTestGate, PropertyTestGate, IntegrationGate, and DiffGate without any new gate-side work.

## Agent Execution Notes

### B.04 — Canonical Runtime Rung Contract

This should be the first executable batch in `04`.

Recommended slice:

1. eliminate the swapped `Lint` / `Test` runtime numbering,
2. move gate construction behind one explicit helper or registry,
3. make post-merge behavior explicit rather than hiding it in `_ => all three`.

Acceptance criteria:

- one honest runtime contract explains the requested rung,
- later batches can build on `Rung` without translating around hidden numeric quirks,
- tests make the mapping discoverable.

### B.05-B.10 — Selector + Pipeline Activation

Good outcome:

- `select_rungs(...)` has a production caller,
- `GatePipeline` has a production caller,
- failure escalation is explicit,
- runtime behavior lines up with the canonical rung contract.

Do not widen this into gate algebra or probabilistic pipeline research. Batch `04` only needs the sequential selector / pipeline path to be real.
