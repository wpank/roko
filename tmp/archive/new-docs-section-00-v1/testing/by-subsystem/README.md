# By Subsystem

> Per-crate test coverage: exact counts, test focus, and coverage notes.

**Status**: Shipping
**Last reviewed**: 2026-04-19

---

## Total: 3,761 test functions (audit: 2026-04-17)

This folder maps each Roko crate to its test coverage. Every file follows the same structure: crate overview, test count breakdown by tier, key test focus areas, key property invariants, and known gaps.

---

## Contents

| # | Page | Crate | Tests | Section |
|---|---|---|---|---|
| — | [subsystem-core.md](subsystem-core.md) | `roko-core` | 376 | 00 — Architecture |
| — | [subsystem-orchestrator.md](subsystem-orchestrator.md) | `roko-orchestrator` | 158 | 01 — Orchestration |
| — | [subsystem-agent.md](subsystem-agent.md) | `roko-agent` | 346 | 02 — Agents |
| — | [subsystem-compose.md](subsystem-compose.md) | `roko-compose` | 23+ | 03 — Composition |
| — | [subsystem-gate.md](subsystem-gate.md) | `roko-gate` | 200 | 04 — Verification |
| — | [subsystem-fs.md](subsystem-fs.md) | `roko-fs` | 37 | 04 — Storage |
| — | [subsystem-learn.md](subsystem-learn.md) | `roko-learn` | 101 | 05 — Learning |
| — | [subsystem-neuro.md](subsystem-neuro.md) | `roko-neuro` | (see notes) | 06 — Neuro |
| — | [subsystem-daimon.md](subsystem-daimon.md) | `roko-daimon` | (see notes) | 09 — Daimon |
| — | [subsystem-dreams.md](subsystem-dreams.md) | `roko-dreams` | (see notes) | 10 — Dreams |
| — | [subsystem-chain.md](subsystem-chain.md) | `roko-chain` | 52 | 08 — Chain |
| — | [subsystem-std.md](subsystem-std.md) | `roko-std` | 96 | 18 — Tools |
| — | [subsystem-runtime.md](subsystem-runtime.md) | `roko-runtime` | — | Infrastructure |
| — | [subsystem-serve.md](subsystem-serve.md) | `roko-serve` | — | 12 — Interfaces |
| — | [subsystem-cli.md](subsystem-cli.md) | `roko-cli` | 38 | 12 — Interfaces |

---

## Coverage Distribution

```
roko-core          ████████████████████████████████  376
roko-agent         █████████████████████████████     346
roko-gate          █████████████████             200
roko-orchestrator  █████████████                 158
roko-learn         ████████                      101
roko-std           ████████                       96
roko-chain         ████                           52
roko-cli           ███                            38
roko-fs            ███                            37
roko-compose       ██                             23+
(other crates)     ████████████████████████      ~2,334
──────────────────────────────────────────────────────
Total                                            3,761
```

---

## Subsystem → Invariants Mapping

For each subsystem, the key invariants tested are catalogued in [../by-property/](../by-property/README.md):

| Subsystem | Key properties tested |
|---|---|
| core | content-addressing-determinism, score-axis-independence, lineage-acyclicity, decay-monotonicity |
| gate | gate-verdict-monotonicity, gate-verdict-idempotence, pipeline-rung-ordering |
| fs | substrate-idempotence, substrate-read-after-write |
| orchestrator | plan-dag-acyclicity, crash-recovery-consistency |
| agent | cascade-router-fallback, safety-pipeline-ordering |
| learn | bandit-score-monotonicity, episode-completeness |
| neuro | hdc-bundling-commutativity, hdc-binding-bijectivity |

---

## See also

- [../by-property/README.md](../by-property/README.md) — invariant catalog
- [../tiers/01-unit-tests.md](../tiers/01-unit-tests.md) — unit test conventions
