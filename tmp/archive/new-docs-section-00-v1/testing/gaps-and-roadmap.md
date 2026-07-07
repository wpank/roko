# Testing Gaps and Roadmap

> Known testing gaps from the 2026-04-17 audit and planned future test work.

**Status**: Living document
**Last reviewed**: 2026-04-19

---

## Current Gaps

### P0 — Critical (block a future release if unaddressed)

| Gap | Crate | Impact |
|---|---|---|
| `roko-runtime` has 0 unit tests | `roko-runtime` | ProcessSupervisor and event bus correctness tested only indirectly |
| `roko-serve` test count unknown | `roko-serve` | 200+ HTTP routes with unknown test coverage |
| No property tests for `SafetyPipeline` steps 4-6 | `roko-agent` | Post-call content check not property-tested |

### P1 — High (should be addressed in the next development cycle)

| Gap | Crate | Impact |
|---|---|---|
| `roko-learn` low test density (101 tests / 35,847 LOC) | `roko-learn` | Pattern miner, regression detector, efficiency events untested |
| No integration tests for the 10-subsystem simultaneous learning update | `roko-learn` | Correctness of the combined feedback loop is unverified |
| No fuzz tests running | `roko-fuzz` | Parser/deserializer bugs may go undetected |
| `roko-neuro` not wired to runtime → no integration tests | `roko-neuro` | Integration errors will only surface after wiring |
| `SemanticGate` has only 5 tests | `roko-gate` | LLM-based verdict logic minimally tested |

### P2 — Medium (planned for Phase 2 or when the crate matures)

| Gap | Crate | Impact |
|---|---|---|
| `roko-dreams` has no tests | `roko-dreams` | Scaffold status; acceptable for now |
| No chaos tests (panic injection, disk full, OOM) | all | Resilience under resource exhaustion unknown |
| No load tests for `roko-serve` (200+ concurrent SSE clients) | `roko-serve` | Scalability unknown |
| `CursorACP` backend less tested than other 4 backends | `roko-agent` | Less documented protocol |
| No multi-agent concurrent E2E test | `roko-e2e` | Two agents competing for same worktree untested |
| Chain settlement on live testnet not tested | `roko-chain` | Blocked by chain deployment |
| Performance regression: benchmark CI is partial | all | Performance regressions may go undetected pre-release |

---

## Coverage Ratios (Areas of Concern)

| Crate | LOC | Tests | Tests/KLOC |
|---|---|---|---|
| `roko-learn` | 35,847 | 101 | 2.8 |
| `roko-serve` | ~10,000 (est.) | unknown | ? |
| `roko-neuro` | ~8,000 (est.) | ~? | ? |
| `roko-runtime` | ~3,000 (est.) | 0 | 0.0 |

The overall workspace ratio is ~12 tests/KLOC (3,761 tests / 322K LOC). `roko-learn` is well below this average.

---

## Roadmap

### Phase 1 (Current Cycle)

- [ ] Add unit tests for `roko-runtime`: ProcessSupervisor, event bus, cancellation tokens. Target: 50 tests.
- [ ] Audit `roko-serve` test count; add route tests to reach 80% coverage floor.
- [ ] Add property tests for `SafetyPipeline` steps 4-6.
- [ ] Add 100 unit tests to `roko-learn` covering pattern miner and regression detector.

### Phase 2 (Post-Chain Deployment)

- [ ] Add live testnet integration tests for `roko-chain` clearing.
- [ ] Add `roko-neuro` integration tests after runtime wiring.
- [ ] Add `roko-dreams` consolidation tests after implementation begins.
- [ ] Enable continuous fuzz testing in CI.

### Phase 3 (Observability and Chaos)

- [ ] Add post-deploy smoke test automation (currently manual).
- [ ] Add chaos tests: disk full, OOM, subprocess panic injection.
- [ ] Add load tests for `roko-serve`.
- [ ] Add multi-agent concurrent E2E test.

---

## Property Coverage Gaps

Properties that are not yet implemented as tests (stubs in [by-property/](by-property/README.md)):

| Property | Status |
|---|---|
| NREM utility selection | Planned (depends on roko-dreams implementation) |
| Dreams consolidation idempotence | Planned |

---

## Test Infrastructure Gaps

| Infrastructure | Status | Plan |
|---|---|---|
| Continuous fuzzing | Targets defined; not running | CI job in Phase 2 |
| Post-deploy smoke tests | Manual only | Automated in Phase 3 |
| Benchmark dashboard | Local only; not published | Phase 2 |
| Multi-machine E2E | Not implemented | Phase 3 |

---

## See also

- [by-subsystem/](by-subsystem/README.md) — per-crate gap notes
- [by-property/README.md](by-property/README.md) — property coverage
- [tiers/06-fuzz-tests.md](tiers/06-fuzz-tests.md) — fuzz testing roadmap
