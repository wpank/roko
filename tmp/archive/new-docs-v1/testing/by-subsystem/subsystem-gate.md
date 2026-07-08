# roko-gate — Test Coverage

> 200 tests for the 11-gate, 7-rung verification pipeline: gate semantics, verdict logic, and adaptive thresholds.

**Status**: Shipping
**Crate**: `roko-gate`
**Section**: 04 — Verification
**Last reviewed**: 2026-04-19

---

## Test Count: 200

Source: implementation status audit, 2026-04-17.

| Module | Approx. tests | Focus |
|---|---|---|
| `gates` | ~110 | Per-gate verdict logic (11 gates) |
| `pipeline` | ~40 | 7-rung pipeline orchestration, short-circuit |
| `thresholds` | ~25 | EMA-based adaptive thresholds |
| `verdict` | ~15 | Verdict type, serialization, comparison |
| `process_reward` | ~10 | PRM scoring, forensic causal replay |

---

## The 11 Gates

Each gate has its own test module with unit tests covering:
- Pass verdict on a known-good input.
- Fail verdict on a known-bad input.
- Verdict at the threshold boundary (pass ↔ fail).
- Verdict determinism: same input → same verdict.
- Verdict with adaptive threshold below vs. above current baseline.

| Gate | Rung | What it checks |
|---|---|---|
| `CompileGate` | 1 | `cargo build` exits 0; no compilation errors |
| `LintGate` | 2 | `cargo clippy` passes at configured severity level |
| `TestGate` | 3 | `cargo test` exits 0; no test failures |
| `SymbolGate` | 4 | Exported symbols match expected signatures |
| `GeneratedTestGate` | 5 | Agent-generated tests compile and pass |
| `PropertyTestGate` | 6 | `cargo test --features proptest` passes |
| `IntegrationGate` | 7 | Integration test suite passes |
| `BenchmarkGate` | — | No performance regressions > threshold |
| `SemanticGate` | — | LLM semantic review returns pass verdict |
| `SecurityGate` | — | `cargo audit` passes; no known CVEs |
| `FormatGate` | — | `cargo fmt --check` passes |

---

## 7-Rung Pipeline Tests

Tests for the pipeline orchestration layer:

- **Rung ordering**: rungs execute in order 1 → 2 → 3 → 4 → 5 → 6 → 7.
- **Short-circuit**: a failure at rung N stops the pipeline; rungs N+1…7 are not executed.
- **Verdict aggregation**: the pipeline verdict is the minimum verdict across all executed rungs.
- **Rung skip**: a configured pipeline that skips rung 5 never evaluates `GeneratedTestGate`.
- **Monotonic ratcheting**: a pipeline that passed rung N at threshold `t` cannot regress to a lower rung threshold without an explicit override.

Key properties:
- [../by-property/gate-verdict-monotonicity.md](../by-property/gate-verdict-monotonicity.md)
- [../by-property/pipeline-rung-ordering.md](../by-property/pipeline-rung-ordering.md)

---

## Adaptive Threshold Tests

EMA (Exponential Moving Average) thresholds adjust based on recent verdicts:

- After 10 consecutive passes, the threshold tightens by a configurable factor.
- After 3 consecutive fails, the threshold loosens by a configurable factor.
- The threshold never exceeds 1.0 or drops below a configured floor.
- Threshold changes are persisted in the Engram substrate.

Key property: [../by-property/gate-adaptive-threshold-bounds.md](../by-property/gate-adaptive-threshold-bounds.md).

---

## Regression Golden Tests

Each gate has a set of golden files in `tests/golden/gate/<gate_name>/`:
- `pass_canonical.verdict.json` — known-good verdict for a canonical passing input.
- `fail_canonical.verdict.json` — known-bad verdict for a canonical failing input.

See [../tiers/04-regression-tests.md](../tiers/04-regression-tests.md).

---

## The "Failure is a Verdict" Principle

`roko-gate` tests embody the system principle that gate failure is a verdict, not an error. This is tested via:
- `Gate::evaluate()` returns `Ok(Verdict::Fail)` for bad inputs, not `Err(SomeError)`.
- Only infrastructure failures (missing binary, I/O error, timeout) return `Err`.
- A `Verdict::Fail` must always include a `reason` field with a human-readable explanation.
- A `Verdict::Pass` must include the metrics that led to the pass.

---

## Known Gaps

- `SemanticGate` (LLM-review gate) has only 5 tests — the smallest of any gate — because the verdict depends on LLM output quality, which is hard to test deterministically.
- `BenchmarkGate` performance regression thresholds are static in tests; the dynamic EMA behavior is not tested.
- No chaos/adversarial tests: a gate that panics instead of returning `Err` would be hard to detect.

## See also

- [../by-property/gate-verdict-monotonicity.md](../by-property/gate-verdict-monotonicity.md)
- [../by-property/gate-verdict-idempotence.md](../by-property/gate-verdict-idempotence.md)
- [../by-property/gate-adaptive-threshold-bounds.md](../by-property/gate-adaptive-threshold-bounds.md)
- [../tiers/04-regression-tests.md](../tiers/04-regression-tests.md) — golden verdict files
