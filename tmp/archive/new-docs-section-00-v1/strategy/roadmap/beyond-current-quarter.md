# Beyond Current Quarter

> Q2, Q3, Q4 outlook and Q5–Q6 optionality. A forward-looking sketch, not a commitment.

**Snapshot date**: 2026-04-19

---

## Q2 — Learning Substrate

**Prerequisite**: Q1 Foundation complete (Phase C exit criteria).

**Headline**: Durable memory becomes semantically indexed (HDC fingerprint) and economically shaped (demurrage) while the runtime starts learning from prediction and falsification loops.

**Key tracks**:
- HDC fingerprint on every durable `Engram`
- Demurrage replacing age-only pruning
- Heuristics and falsifiers as inspectable library objects
- Self-learning loop (prediction/outcome Pulse topics)
- c-factor measurement visible during multi-agent runs
- Research-to-runtime starter flows

**Primary risk**: demurrage rate tuning. The Q2 checkpoint question: "Is demurrage producing useful compounding instead of cold-tier churn?"

**Full spec**: [`milestone-q2-learning-substrate.md`](milestone-q2-learning-substrate.md)

---

## Q3 — Ecosystem and UX

**Prerequisite**: Q2 Learning Substrate complete.

**Headline**: The runtime becomes externally legible and extensible. Plugin SPI, StateHub projection, realtime wire protocol, and first-party UX surfaces (CLI, TUI, web) converge on one shared runtime contract.

**Key tracks**:
- Plugin SPI (staged extension model, WASM boundary)
- StateHub projection (kernel-tier shared data surface)
- Realtime wire protocol freeze
- Four-layer Rust SDK + `roko init` + unified verb set
- First web UI release
- Portable single-machine and single-server deployment shape

**Primary risk**: UX scope creep. Priority order: wire protocol freeze → Plugin SPI → everything else.

**Full spec**: [`milestone-q3-ecosystem-ux.md`](milestone-q3-ecosystem-ux.md)

---

## Q4 — Scale, Safety, and Domains

**Prerequisite**: Q3 Ecosystem and UX complete.

**Headline**: Roko becomes domain-shaped, auditable, and multi-tenant enough for serious team workflows.

**Key tracks**:
- Domain profiles (`TypedContext`, starter heuristics, domain-specific gates)
- Safety spine (custody, taint, provenance, audit tooling)
- Replication ledger expansion
- Multi-tenant deployment hardening
- c-factor actuation (policy responds to degraded collective intelligence)

**Primary risk**: multi-tenant auth and isolation.

**Full spec**: [`milestone-q4-scale-safety-domains.md`](milestone-q4-scale-safety-domains.md)

---

## Q5–Q6 — Phase 2 Optionality

**Prerequisite**: Q4 complete + Phase D prerequisites (Bus frozen, safety spine operational, deployment hardened).

**Status**: Deliberately deferred. Q1–Q4 should produce a coherent product even if Q5–Q6 slips indefinitely.

**Scope**:
- ChainBus and MeshBus backends
- MultiBus composition layer
- Dreams (Delta-speed consolidation loop) online
- Composer rewrite (once HDC retrieval + projection layers stable)
- Cross-deployment witness flows
- Published plugin registry

**Full spec**: [`milestone-q5q6-phase2-optionality.md`](milestone-q5q6-phase2-optionality.md)

---

## Not-doing list (Q1–Q4)

Confirmed out of scope for Q1–Q4 (no timeline assigned):

- Training custom models on accumulated episodes
- A graphical plan editor beyond existing plan views
- Full native SDK parity beyond explicitly planned client surfaces
- A first-party inference server
- A Kubernetes operator beyond Helm-grade packaging
- A native mobile app
- A standalone voice-first workflow

---

## See also

- [`current-quarter.md`](current-quarter.md) — Q1 active work
- [`dependencies.md`](dependencies.md) — milestone dependency graph
- [`00-overview.md`](00-overview.md) — sequencing principles and critical path narrative
