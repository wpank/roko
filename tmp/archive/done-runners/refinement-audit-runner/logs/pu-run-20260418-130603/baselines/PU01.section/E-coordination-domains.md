# E — Coordination & Domains (Docs 12-13)

Covers: stigmergy and cross-domain orchestration.

The audit verdict here is not "narrow the batch."

It is "defer the batch."

Docs `12-13` contain useful theory and future-state design language, but they are not current batch-01 implementation targets.

---

## E.01 — Stigmergy & Niche Construction (Doc 12) — DEFERRED

What exists today:

- indirect coordination through the shared repo, worktrees, merge flow, logging, and persisted traces
- `Kind::Pheromone` in `roko-core`
- `Decay::THREAT`, `Decay::OPPORTUNITY`, and `Decay::WISDOM`
- `FleetCFactor` in orchestration reporting

What does not exist as a live orchestration subsystem:

- formal stigmergy API
- orchestrator-owned pheromone model
- runtime pheromone economy
- batch-01 implementation surface for doc `12`

This doc should be treated as conceptual framing plus small existing primitives, not as evidence of a shipped stigmergy layer.

That means present tense is only safe for the primitives above. Anything stronger should be marked target-state.

## E.02 — Cross-Domain Orchestration (Doc 13) — DEFERRED

What exists today:

- generic orchestration machinery
- code-centric runtime execution
- some research and documentation support
- background hypothesis generation in `roko-dreams`

What does not exist as batch-01 runtime:

- chain-domain execution
- template system
- saga coordinator
- semantic merge
- plan repair engine
- domain-specific orchestration layer

This file should stop feeding an `O6` implementation story.

The runtime is still centered on code plans. Reusing generic orchestration machinery for later domains is plausible, but it is not current batch-01 runtime behavior.

---

## Post-Audit Summary

| Item | Status | What matters now |
|------|--------|------------------|
| Stigmergy framing | Useful theory | keep as future-state framing only |
| Cross-domain orchestration | Phase 2+ | do not schedule as batch-01 work |
| Existing primitives | Small | mention them without inflating them |
| Executable batch | None | `O6` is the deferral lane, not a code batch |

## Minimal Present-Tense Facts

- indirect coordination channels already influence execution
- pheromone-adjacent primitives exist in `roko-core`
- `roko-dreams` exists as background/offline work
- none of that adds up to a live stigmergy subsystem or cross-domain runtime

---

## Keep vs Defer

Keep in present tense:

- indirect coordination channels already used by the runtime
- the few live pheromone-adjacent primitives
- the fact that the runtime is already multi-plan

Defer explicitly:

- formal stigmergy
- cross-domain chain work
- templates, sagas, semantic merge, repair engines
- any claim that docs `12-13` describe a fully shipped orchestration layer
