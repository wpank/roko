# Milestone: Q3 — Ecosystem and UX

> The runtime becomes externally legible and extensible: plugin SPI, StateHub projection, stable realtime transport, and first-party UX surfaces all converge.

**Target**: Q3 (full-team estimate)
**Status**: Planned
**Owner**: UX engineer (primary); Platform engineer (Plugin SPI, StateHub, deployment)
**Prerequisites**: [Q2 — Learning Substrate](milestone-q2-learning-substrate.md) complete
**Unlocks**: [Q4 — Scale, Safety, and Domains](milestone-q4-scale-safety-domains.md)
**Roadmap quarter risk**: UX scope creep
**Last reviewed**: 2026-04-19

---

## Headline

The runtime becomes externally legible and extensible: plugin SPI, StateHub projection, stable realtime transport, and first-party UX surfaces (CLI, TUI, web) all converge on one shared runtime contract.

---

## Quarter demo

A third party installs a plugin and sees the same runtime surface reflected in CLI, TUI, and web clients via one shared projection and transport contract.

---

## Tracks

| Track | Scope | REFs |
|---|---|---|
| Plugin SPI | Land the staged extension model from prompt/profile layers through native and WASM boundaries | REF17 |
| StateHub projection | Promote StateHub projection into a kernel-tier shared data surface | REF26 |
| Realtime wire protocol | Freeze the shared wire protocol; support multiple client surfaces | REF27 |
| Developer UX | Four-layer Rust SDK, interactive `roko init`, unified verbs, CLI parity | REF22 |
| User UX | First web UI release; rich UX primitives | REF23, REF28, REF29, REF30 |
| Deployment shape | Single-machine and single-server deployment portable and reproducible | REF24 |

---

## Deliverables

- [ ] Plugin SPI: staged extension model operational from prompt layer through native boundary
- [ ] StateHub projection: promoted to kernel-tier shared data surface
- [ ] Realtime wire protocol: frozen spec; CLI, TUI, and web client surfaces all consuming it
- [ ] Developer UX: four-layer Rust SDK, `roko init`, unified verb set
- [ ] First web UI release
- [ ] Portable single-machine and single-server deployment shape

---

## Exit criteria

- [ ] A third party can install a plugin without modifying Roko's source code
- [ ] Plugin SPI checkpoint: "Are external plugins actually installing and surviving onboarding?" → Go
- [ ] CLI, TUI, and web clients show the same runtime state via the shared wire protocol
- [ ] `roko deploy` produces a reproducible deployment on a clean machine

---

## Current status

Not started. Awaiting Q2 completion.

---

## Risk

**UX scope creep**: Q3 has the highest breadth of any quarter. Mitigation: prioritize the wire protocol freeze and Plugin SPI first (they gate the rest of the surface work). Push anything that doesn't require a shared wire protocol to Q4 without ceremony.

---

## REF alignment

| REF | Scope |
|---|---|
| REF17 | Plugin extension architecture (SPI) |
| REF22 | Developer UX (Rust SDK, CLI) |
| REF23 | User UX (running agents) |
| REF24 | Deployment UX |
| REF25 | Domain-specific agents (first profiles land in Q4; groundwork here) |
| REF26 | StateHub rearchitecture |
| REF27 | Realtime wire surface |
| REF28 | CLI parity |
| REF29 | Web UI |
| REF30 | Rich UX primitives |

---

## See also

- [`strategy/roadmap/milestone-q2-learning-substrate.md`](milestone-q2-learning-substrate.md) — prerequisite
- [`strategy/roadmap/milestone-q4-scale-safety-domains.md`](milestone-q4-scale-safety-domains.md) — what this unlocks
- [`strategy/roadmap/dependencies.md`](dependencies.md)
