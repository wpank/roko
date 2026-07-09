# Milestone: Q2 — Learning Substrate

> Durable memory becomes semantically indexed and economically shaped while the runtime starts learning from prediction and falsification loops.

**Target**: Q2 (full-team estimate)
**Status**: Planned
**Owner**: Learning engineer (primary)
**Prerequisites**: [Q1 — Foundation](milestone-q1-foundation.md) complete (Phase C exit criteria passed)
**Unlocks**: [Q3 — Ecosystem and UX](milestone-q3-ecosystem-ux.md)
**Roadmap quarter risk**: Demurrage rate tuning
**Last reviewed**: 2026-04-19

---

## Headline

Durable memory becomes semantically indexed (HDC fingerprint) and economically shaped (demurrage) while the runtime starts learning from prediction and falsification loops (heuristics + c-factor).

---

## Quarter demo

Calibrated heuristics are visible in the product, HDC-backed retrieval is live, and c-factor can be inspected during a multi-agent run.

---

## Tracks

| Track | Scope | Primary docs | REFs |
|---|---|---|---|
| HDC fingerprint | Add first-class HDC fingerprint to every durable `Engram`; expose similarity queries | `reference/10-types/`, `research/perspectives/temporal-topology/` | REF11 |
| Demurrage | Replace age-only pruning with balance, reinforcement, and cold-tier durable memory management | `reference/10-types/decay.md`, `reference/08-layers/` | REF12 |
| Heuristics | Promote heuristics, falsifiers, and calibration into inspectable library objects | `subsystems/` (learning section) | REF14 |
| Self-learning and c-factor | Wire prediction/outcome topics, calibration policies, and visible c-factor measurement | `reference/06-loop/`, `research/foundations/c-factor.md` | REF10, REF13 |
| Research-to-runtime | Land paper, claim, and replication-ledger starter flows | `research/references/` | REF16 |

---

## Deliverables

- [ ] HDC fingerprint added to `Engram` in `roko-core`; similarity queries exposed via `Substrate`
- [ ] Demurrage policy replaces age-only pruning; cold-tier durable memory management operational
- [ ] Heuristics and falsifiers are inspectable library objects with calibration state visible
- [ ] Prediction/outcome Pulse topics wired; calibration policy operational
- [ ] c-factor measurement inspectable during a multi-agent run
- [ ] Paper + claim + replication-ledger starter flows operational

---

## Exit criteria

- [ ] HDC-backed retrieval demonstrates higher precision than pure recency-based retrieval on a benchmark corpus
- [ ] Demurrage checkpoint: "Is demurrage producing useful compounding instead of cold-tier churn?" → Go
- [ ] Heuristics with falsification records visible in at least one plan flow
- [ ] c-factor visible in the TUI or CLI during a multi-agent run

---

## Current status

Not started. Awaiting Q1 completion.

---

## Risk

**Demurrage rate tuning**: the balance/reinforcement model may produce cold-tier churn rather than useful compounding if the rate parameters are miscalibrated. This is the primary Q2 risk. Checkpoint at Q2 end: assess compounding vs. churn signal before proceeding.

---

## REF alignment

| REF | Scope |
|---|---|
| REF10 | Self-learning cybernetic loops |
| REF11 | Hyperdimensional substrate (HDC fingerprint) |
| REF12 | Knowledge demurrage |
| REF13 | Collective intelligence c-factor |
| REF14 | Worldview validation / heuristics |
| REF16 | Research-to-runtime |

---

## See also

- [`strategy/roadmap/milestone-q1-foundation.md`](milestone-q1-foundation.md) — prerequisite
- [`strategy/roadmap/milestone-q3-ecosystem-ux.md`](milestone-q3-ecosystem-ux.md) — what this unlocks
- [`strategy/roadmap/dependencies.md`](dependencies.md)
- [`research/perspectives/`](../../research/perspectives/README.md) — HDC, demurrage, c-factor foundations
