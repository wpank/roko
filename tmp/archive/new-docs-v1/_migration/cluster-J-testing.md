# Migration Log — Cluster J: Testing

**Source**: `docs/00-architecture/32-comprehensive-test-strategy.md` (1573 lines, 74.3 KB)
**Target base**: `tmp/new-docs/testing/`
**Refactor verb**: split + reorganize
**Completed**: 2026-04-19

---

## Files Written

### Root-level (testing/)

| File | Lines | Source coverage |
|---|---|---|
| `testing/README.md` | 74 | Top-level index; test count table; reader guides |
| `testing/00-test-philosophy.md` | 108 | Test philosophy, what we test/don't, design principles, gate relationship |
| `testing/01-test-pyramid.md` | 166 | Five-tier pyramid, tier definitions, CI execution order |

### testing/tiers/

| File | Lines | Source coverage |
|---|---|---|
| `tiers/README.md` | 37 | Tier index |
| `tiers/01-unit-tests.md` | 145 | Unit test scope, naming, frameworks, TestContext, per-crate counts, coverage |
| `tiers/02-integration-tests.md` | 139 | Multi-crate scope, IntegrationContext, fixture tapes, failure semantics |
| `tiers/03-property-tests.md` | 195 | proptest framework, invariant overview, strategies, counterexample handling |
| `tiers/04-regression-tests.md` | 160 | Golden files, verdict replay, diff output, update workflow |
| `tiers/05-end-to-end-tests.md` | 177 | Full self-hosting loop, hermetic environment, crash/resume test |
| `tiers/06-fuzz-tests.md` | 140 | cargo-fuzz targets, corpus management, sanitizer config |
| `tiers/07-performance-tests.md` | 139 | criterion benchmarks, per-crate catalogue, flakiness control |

### testing/by-subsystem/

| File | Lines | Crate | Test count |
|---|---|---|---|
| `by-subsystem/README.md` | 78 | — | 3,761 total |
| `by-subsystem/subsystem-core.md` | 115 | `roko-core` | 376 |
| `by-subsystem/subsystem-orchestrator.md` | 104 | `roko-orchestrator` | 158 |
| `by-subsystem/subsystem-agent.md` | 101 | `roko-agent` | 346 |
| `by-subsystem/subsystem-compose.md` | 73 | `roko-compose` | 23+ |
| `by-subsystem/subsystem-gate.md` | 112 | `roko-gate` | 200 |
| `by-subsystem/subsystem-fs.md` | 75 | `roko-fs` | 37 |
| `by-subsystem/subsystem-learn.md` | 96 | `roko-learn` | 101 |
| `by-subsystem/subsystem-neuro.md` | 82 | `roko-neuro` | — |
| `by-subsystem/subsystem-daimon.md` | 85 | `roko-daimon` | — |
| `by-subsystem/subsystem-dreams.md` | 57 | `roko-dreams` | 0 (scaffold) |
| `by-subsystem/subsystem-chain.md` | 82 | `roko-chain` | 52 |
| `by-subsystem/subsystem-std.md` | 70 | `roko-std` | 96 |
| `by-subsystem/subsystem-runtime.md` | 57 | `roko-runtime` | 0 |
| `by-subsystem/subsystem-serve.md` | 59 | `roko-serve` | — |
| `by-subsystem/subsystem-cli.md` | 70 | `roko-cli` | 38 |

### testing/by-property/

| File | Property type |
|---|---|
| `by-property/README.md` | Index of 33 properties |
| `by-property/content-addressing-determinism.md` | Determinism |
| `by-property/content-hash-collision-resistance.md` | Collision resistance |
| `by-property/engram-serialization-roundtrip.md` | Round-trip identity |
| `by-property/score-axis-independence.md` | Independence |
| `by-property/score-normalization-range.md` | Boundedness |
| `by-property/score-aggregation-monotonicity.md` | Monotonicity |
| `by-property/decay-monotonicity.md` | Monotonicity |
| `by-property/decay-exponential-asymptote.md` | Asymptotic limit |
| `by-property/decay-linear-terminus.md` | Terminal condition |
| `by-property/lineage-acyclicity.md` | Acyclicity |
| `by-property/provenance-chain-integrity.md` | Integrity |
| `by-property/gate-verdict-monotonicity.md` | Monotonicity |
| `by-property/gate-verdict-idempotence.md` | Idempotence |
| `by-property/pipeline-rung-ordering.md` | Ordering |
| `by-property/gate-adaptive-threshold-bounds.md` | Boundedness |
| `by-property/substrate-idempotence.md` | Idempotence |
| `by-property/substrate-read-after-write.md` | Consistency |
| `by-property/substrate-gc-preserves-living.md` | Safety (GC) |
| `by-property/hdc-bundling-commutativity.md` | Commutativity |
| `by-property/hdc-binding-bijectivity.md` | Bijectivity |
| `by-property/neuro-knowledge-tier-monotonicity.md` | Monotonicity |
| `by-property/plan-dag-acyclicity.md` | Acyclicity |
| `by-property/crash-recovery-consistency.md` | Consistency |
| `by-property/event-log-replay-idempotence.md` | Idempotence |
| `by-property/cascade-router-fallback-ordering.md` | Completeness |
| `by-property/safety-pipeline-ordering.md` | Ordering |
| `by-property/token-count-determinism.md` | Determinism |
| `by-property/bandit-score-monotonicity.md` | Monotonicity |
| `by-property/c-factor-bounds.md` | Boundedness |
| `by-property/pad-vector-bounds.md` | Boundedness |
| `by-property/daimon-no-terminal-state.md` | Liveness |
| `by-property/soulbound-non-transferability.md` | Non-transferability |
| `by-property/reputation-ema-convergence.md` | Convergence |
| `by-property/token-demurrage-rate.md` | Rate invariant |
| `by-property/isfr-clearing-idempotence.md` | Idempotence |
| `by-property/prompt-layer-ordering.md` | Ordering |
| `by-property/cancellation-token-propagation.md` | Propagation |
| `by-property/tool-file-ops-idempotence.md` | Idempotence |
| `by-property/tool-dispatch-determinism.md` | Determinism |

### testing/tools-and-harness/

| File | Lines |
|---|---|
| `tools-and-harness/README.md` | 27 |
| `tools-and-harness/01-test-harness.md` | 130 |
| `tools-and-harness/02-mock-llms.md` | 134 |
| `tools-and-harness/03-fixture-library.md` | 106 |
| `tools-and-harness/04-ci-integration.md` | 133 |
| `tools-and-harness/05-coverage-tooling.md` | 82 |
| `tools-and-harness/06-snapshot-testing.md` | 130 |

### testing/quality-gates/

| File | Lines |
|---|---|
| `quality-gates/README.md` | 36 |
| `quality-gates/01-pre-commit.md` | 71 |
| `quality-gates/02-pr-checks.md` | 74 |
| `quality-gates/03-pre-release.md` | 76 |
| `quality-gates/04-post-deploy.md` | 74 |

### Other

| File | Lines |
|---|---|
| `testing/gaps-and-roadmap.md` | 109 |

---

## File Count Summary

| Folder | Files |
|---|---|
| `testing/` (root) | 3 |
| `testing/tiers/` | 8 |
| `testing/by-subsystem/` | 16 |
| `testing/by-property/` | 40 |
| `testing/tools-and-harness/` | 7 |
| `testing/quality-gates/` | 5 |
| **Total** | **79** |

---

## Coverage Verification

### Test Counts Preserved

All test counts from the source are preserved in `by-subsystem/` files:
- 3,761 total (README.md, by-subsystem/README.md)
- `roko-core`: 376 (subsystem-core.md)
- `roko-agent`: 346 (subsystem-agent.md)
- `roko-gate`: 200 (subsystem-gate.md)
- `roko-orchestrator`: 158 (subsystem-orchestrator.md)
- `roko-learn`: 101 (subsystem-learn.md)
- `roko-std`: 96 (subsystem-std.md)
- `roko-chain`: 52 (subsystem-chain.md)
- `roko-cli`: 38 (subsystem-cli.md)
- `roko-fs`: 37 (subsystem-fs.md)
- `roko-compose`: 23+ (subsystem-compose.md)

### Properties Extracted: 39

All properties described in the source as "must always hold", "invariant", "monotonicity", "idempotence", "determinism", "acyclicity", etc. are catalogued in `by-property/`. The count (39 property files) exceeds the minimum expectation because the source is thorough on invariants.

### Subsystems Covered: 15

core, orchestrator, agent, compose, gate, fs, learn, neuro, daimon, dreams, chain, std, runtime, serve, cli.

### Coverage Gaps Documented

All known testing gaps from the source are captured in `gaps-and-roadmap.md`:
- P0: roko-runtime (0 tests), roko-serve (unknown count), safety pipeline property gaps.
- P1: roko-learn density, fuzz tests not running, neuro/dreams missing integration.
- P2: chain live testnet, chaos tests, load tests, multi-agent E2E.

---

## Source → Destination Mapping

| Source section | Destination file(s) |
|---|---|
| Test philosophy, principles | `testing/00-test-philosophy.md` |
| Test pyramid / tier overview | `testing/01-test-pyramid.md` |
| Unit test conventions | `tiers/01-unit-tests.md` |
| Integration test setup | `tiers/02-integration-tests.md` |
| Property-based testing | `tiers/03-property-tests.md` |
| Regression / golden testing | `tiers/04-regression-tests.md` |
| End-to-end self-hosting loop | `tiers/05-end-to-end-tests.md` |
| Fuzz testing | `tiers/06-fuzz-tests.md` |
| Performance benchmarking | `tiers/07-performance-tests.md` |
| Per-crate test counts + focus | `by-subsystem/subsystem-*.md` |
| Invariants and properties | `by-property/*.md` |
| Test harness (roko-test) | `tools-and-harness/01-test-harness.md` |
| Mock LLM / tape replay | `tools-and-harness/02-mock-llms.md` |
| Fixture library | `tools-and-harness/03-fixture-library.md` |
| CI pipeline | `tools-and-harness/04-ci-integration.md` |
| Coverage tooling | `tools-and-harness/05-coverage-tooling.md` |
| Snapshot / golden infrastructure | `tools-and-harness/06-snapshot-testing.md` |
| Pre-commit gate | `quality-gates/01-pre-commit.md` |
| PR checks | `quality-gates/02-pr-checks.md` |
| Pre-release checks | `quality-gates/03-pre-release.md` |
| Post-deploy observability | `quality-gates/04-post-deploy.md` |
| Known gaps + roadmap | `testing/gaps-and-roadmap.md` |
