# Milestone Dependency Graph

> Which milestone must land before which, and why.

**Last reviewed**: 2026-04-19

---

## The DAG

```
Q1 Foundation
    │
    │  Phase C must be stable before learning substrate can assume Bus + Pulse
    ▼
Q2 Learning Substrate
    │
    │  Stable heuristics + calibration + shared wire contract must exist
    │  before external surfaces can converge on them
    ▼
Q3 Ecosystem and UX
    │
    │  Safety spine + deployment hardening must exist before
    │  multi-tenant isolation and distributed trust
    ▼
Q4 Scale, Safety, and Domains
    │
    │  In-process Bus proven stable + safety spine + deployment hardened
    │  before distributed backends are worth landing
    ▼
Q5–Q6 Phase 2 Optionality     ← optional; architecture stands without it
```

---

## Dependency table

| Milestone | Immediate prerequisite | Reason |
|---|---|---|
| Q1 Foundation | None | Starting point |
| Q2 Learning Substrate | Q1 complete (Phase C exit criteria) | HDC fingerprint, demurrage, and self-learning loops require stable `Pulse` topics and `Bus` subscriptions |
| Q3 Ecosystem and UX | Q2 complete | Plugin SPI, StateHub, and realtime wire all depend on the shared runtime contract established in Q1–Q2 |
| Q4 Scale, Safety, Domains | Q3 complete | Domain profiles and safety spine require the Plugin SPI and realtime wire contract; multi-tenant isolation requires a stable deployment shape |
| Q5–Q6 Phase 2 Optionality | Q4 complete + Phase D prerequisites | Safety spine and proven in-process Bus must exist before distributed trust assumptions are added |

---

## Key dependency edges

### Kernel → Learning

Q2 tracks (HDC fingerprint, demurrage, heuristics, self-learning, c-factor) all require:
- `Bus` and `Pulse` topics stable in `roko-core`
- Subsystem-specific transport enums gone (Phase C complete)
- No active compatibility shim debt

If Phase C slips, these Q2 tracks must wait.

### Learning → Ecosystem

Q3 tracks (Plugin SPI, StateHub, realtime wire, developer/user UX) require:
- Calibration-bearing heuristics available (so the plugin model can expose them)
- Stable c-factor measurement (so the realtime wire protocol has useful payload to carry)
- HDC-backed retrieval operational (so the Composer rewrite is informed)

### Ecosystem → Scale

Q4 tracks (domain profiles, safety spine, multi-tenant hardening) require:
- Plugin SPI stable (domain profiles are implemented as plugins)
- Realtime wire protocol frozen (safety spine uses it for audit events)
- Deployment shape portable (multi-tenant hardening extends it)

---

## Parallel work within quarters

Within each quarter, several tracks can run in parallel once the kernel critical path is stable:

| Quarter | Serial (kernel critical path) | Can run in parallel once serial track is stable |
|---|---|---|
| Q1 | Phase A → Phase B → Phase C | Naming, Modularity, Observability baseline |
| Q2 | HDC fingerprint → Demurrage → Heuristics | Self-learning + c-factor (once HDC + demurrage are stable) |
| Q3 | Wire protocol freeze → Plugin SPI | Developer UX, User UX, Deployment shape |
| Q4 | Safety spine → Multi-tenant hardening | Domain profiles, Replication ledger expansion |

---

## Cross-plan alignment

This roadmap aligns with but does not require:

| Adjacent plan | Alignment |
|---|---|
| `tmp/ux-followup/` | Q2 and Q3 absorb most observability, state portability, and surface consistency items |
| `tmp/MASTER-PLAN.md` | Adds architectural homes and dependency order to the flat inventory |

If adjacent plans and this roadmap disagree, prefer the dependency order here and update the adjacent inventory.

---

## See also

- [`strategy/refactor-phases/dependencies.md`](../refactor-phases/dependencies.md) — phase-level (A–D) dependency DAG
- [`strategy/roadmap/00-overview.md`](00-overview.md) — critical path narrative
- [`analysis/synergy-map/`](../../analysis/synergy-map/README.md) — why the dependency order compounds
