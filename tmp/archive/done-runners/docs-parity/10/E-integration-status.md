# E - Cross-System Integration and Implementation Status (Docs 15, 16)

This is the main regeneration section. The runtime is ahead of the docs,
and Doc 16 is the place where that drift is most misleading.

Generated: 2026-04-18

---

## Runtime Reality

`roko-dreams` is currently a 7-file runtime surface with 5,964 LOC:

| File | LOC | Current role |
|------|-----|--------------|
| `lib.rs` | 119 | module exports and compatibility types |
| `runner.rs` | 1,054 | public runner facade, controls, trigger/schedule/heartbeat policies |
| `cycle.rs` | 2,917 | core cycle, reporting, persistence, cluster outputs |
| `replay.rs` | 449 | replay planning and utility scoring |
| `imagination.rs` | 575 | REM counterfactual imagination and creativity modes |
| `hypnagogia.rs` | 538 | hypnagogia engine |
| `threat.rs` | 312 | threat simulation |

That is the baseline Doc 16 should be regenerated from.

---

## What Already Ships

### E.01 - Dreams -> Neuro

**Status**: DONE

Dreams emits `KnowledgeEntry` values and persists them via `KnowledgeStore`.

### E.02 - Dreams -> Daimon

**Status**: DONE

The runtime already relies on Daimon-era depotentiation behavior; Daimon
must not be described as missing support infrastructure.

### E.03 - Dreams -> Agent

**Status**: DONE

`build_dream_review_dispatcher()` plus `AgentDispatcher` are live runtime
surfaces, not roadmap placeholders.

### E.04 - CLI, daemon, and orchestrator entry points

**Status**: DONE

- CLI: `main.rs:5609-5704`
- daemon loop: `daemon.rs:239-268`
- orchestrator auto-dream path: `orchestrate.rs:5890-5969`

Docs should treat dreams as a live runtime surface, not just an internal
crate.

---

## Mixed Integration Reality

### E.05 - Dreams -> Learn

**Status**: PARTIAL

Episode ingestion and playbook support are real. Pattern mining and
cross-episode consolidation infrastructure are real. What stays partial
is exact dream-cycle wiring for every learning helper.

### E.06 - Dreams -> Compose / Gate / Mesh

**Status**: PARTIAL

The important clarification is architectural:

- dreams does not directly depend on `roko-compose`, `roko-gate`, or mesh crates,
- the main exchange surface is mediated through `KnowledgeStore` and learning outputs,
- stronger downstream feedback loops remain future work.

This is a narrowing, not a deletion.

---

## Doc 16 Regeneration Priorities

### E.07 - Replace stale `roko-golem` ownership

**Status**: REQUIRED

Any remaining `roko-golem` dissolution-plan framing is obsolete. The
runtime already lives in `roko-dreams`.

### E.08 - Rebuild the module/status table from current code

**Status**: REQUIRED

Doc 16 should explicitly include `replay.rs`, `imagination.rs`,
`hypnagogia.rs`, and `threat.rs`, not just `runner.rs` and `cycle.rs`.

### E.09 - Flip Phase 3 from "not started" to shipped

**Status**: REQUIRED

Pearl-adjacent counterfactuals, creativity modes, depotentiation-adjacent
REM output, threat simulation, and hypnagogia all ship now.

### E.10 - Keep Phase 4 and Phase 5 narrow

**Status**: REQUIRED

The real open seams are still:

- stronger downstream feedback loops,
- dream sharing and richer journal systems,
- oneirography and rendering,
- nightmare/lucid systems.

Those should stay explicitly future-facing.

---

## What To Carry Into The Live Docs

- Doc 15 should present mediated-via-Neuro integration as the main architecture.
- Doc 16 should be regenerated from current module reality, not from legacy crate history.
- The dependency table should show Daimon as implemented and HDC as already used from dreams.
- Frontier systems should move to future-work sections instead of being described as current wiring.
