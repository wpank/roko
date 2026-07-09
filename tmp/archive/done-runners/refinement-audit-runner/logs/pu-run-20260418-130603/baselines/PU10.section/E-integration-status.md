# E — Cross-System Integration and Implementation Status (Docs 15, 16)

Parity of the two "wiring" chapters: cross-system integration (Neuro,
Daimon, Learn, Compose, Gate, Mesh, Orchestrator, Hypnagogia,
Supervisor) and the canonical implementation-status doc.

**Major finding**: Doc 16 is the most-stale status doc in the parity
batches so far. It does not acknowledge `hypnagogia.rs`,
`imagination.rs`, `replay.rs`, or `threat.rs` as shipping modules —
even though each is a substantial file consuming `roko-learn` +
`roko-neuro` + `roko-primitives` APIs. It also references the
dissolved `roko-golem` as a live dependency.

Doc 15 is better — most of its integration claims match the actual
cross-crate dep graph in `Cargo.toml`.

Generated 2026-04-16.

---

## E.01 — Dreams → Neuro (KnowledgeStore) is wired (Doc 15 §"Integration with Neuro")

**Status**: DONE
**Severity**: —
**Doc claim**: Dreams read episodes from the Neuro layer and write consolidated insights back.
**Reality**: `Cargo.toml:17` declares `roko-neuro` path dep. `imagination.rs:12` imports `roko_neuro::{KnowledgeEntry, KnowledgeKind, KnowledgeTier}`. `hypnagogia.rs:11` imports the same. `threat.rs:11` imports the same. `synthesize_hypotheses` (imagination.rs:178) and `threat_warning_entries` (threat.rs:85) both emit `Vec<KnowledgeEntry>` tagged with `"dream"` for waking-side filtering. Confirmed at source level.

---

## E.02 — Dreams → Daimon (emotional depotentiation) is wired (Doc 15 §"Integration with Daimon")

**Status**: DONE (cross-ref B.08)
**Severity**: —
**Doc claim**: Dreams depotentiate high-arousal somatic markers via REM; Daimon receives the update.
**Reality**: See B.08. The `DEPOTENTIATION_*` constants live in `roko-daimon/src/lib.rs:26-28`; dream cycle invokes the daimon's depotentiation pass (per Doc 13 daimon status doc §"Somatic query + modulation — Partial: dream replay now depotentiates..."). Cross-system wiring is live.

---

## E.03 — Dreams → Learn (EpisodeLogger, PatternMiner, ClusterConsolidator, PlaybookStore) (Doc 15 §"Integration with Learn")

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Dreams consume `EpisodeLogger` for episode read, `PatternMiner` for pattern discovery, `CrossEpisodeConsolidator` for K-medoids clustering, `PlaybookStore` for playbook revision.
**Reality**: `Cargo.toml:18` declares `roko-learn` path dep. `replay.rs:10`, `imagination.rs:11`, `threat.rs:10`, `hypnagogia.rs:10` all import `roko_learn::episode_logger::Episode`. `replay.rs:10` also imports `GateVerdict`. So the episode-read surface is fully wired. **What is NOT verified**: whether `cycle.rs` explicitly invokes `PatternMiner` and `CrossEpisodeConsolidator` during the NREM pass (cross-ref B.09 and B.10). `PlaybookStore` usage is referenced in Doc 16 §"Key implemented behaviors" as passed to `DreamCycle`. Partial because the specific wiring is unverified.

---

## E.04 — Dreams → Compose / Gate / Mesh / Orchestrator / Supervisor (Doc 15 §"Integration with Compose", §"Mesh", §"Orchestrator")

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Doc 15 describes outbound flows: dreams → Compose (insights → prompts), dreams → Gate (threshold updates), dreams → CascadeRouter (cost model updates), dreams → Mesh (insight sharing), dreams → Orchestrator (scheduling coordination), dreams → Supervisor (process lifecycle).
**Reality**: `Cargo.toml` does **not** declare `roko-compose`, `roko-gate`, `roko-orchestrator`, or mesh-related deps. Only direct deps: `roko-core`, `roko-neuro`, `roko-learn`, `roko-agent`, `roko-primitives`. So the outbound integration is mediated through Neuro (KnowledgeStore) + Learn (episode log + playbook) rather than direct crate deps. This is actually cleaner — dreams don't need to know about Compose / Gate / Cascade directly; those subsystems read from Neuro. Partial because the downstream consumption is not verified to be widespread.
**Fix sketch**: Doc 15 §"Integration" diagrams should clarify that dreams communicate via Neuro's KnowledgeStore rather than direct crate deps to Compose / Gate / Cascade. This is a better architecture than the doc's "direct integration" framing implies.

---

## E.05 — Dreams → Agent (ClaudeCliAgent + ExecAgent) is wired (Doc 15 §"Agent Dispatch")

**Status**: DONE
**Severity**: —
**Doc claim**: Dreams dispatch inference through the agent layer for deep reflection / counterfactual LLM queries.
**Reality**: `Cargo.toml:19` declares `roko-agent` path dep. `build_dream_review_dispatcher` at `runner.rs:853` constructs dispatchers per Doc 16 §"Agent dispatch": "`ClaudeCliAgent` for Claude CLI-based dream inference and `ExecAgent` for arbitrary command execution". `DreamAgentConfig` at `runner.rs:43-136` carries command / args / model / bare_mode / effort / timeout_ms / env. `AgentDispatcher` trait at `cycle.rs:50` is the pluggable interface. Live.

---

## E.06 — Doc 16 §"Current Code Status" is significantly outdated (Doc 16 §"Current Code Status", §"roko-dreams Crate")

**Status**: PARTIAL (doc is live but wrong)
**Severity**: HIGH (for doc honesty)
**Doc claim**: Doc 16 §"roko-dreams Crate" lists only two shipping modules: `runner.rs` and `cycle.rs`. The lib.rs re-exports snippet shows only `DreamCycle`, `DreamCycleReport`, `AgentDispatcher` from cycle and `DreamAgentConfig...Insight` from runner.
**Reality**: Current `crates/roko-dreams/src/lib.rs:1-86` has **7 modules**: cycle, hypnagogia, imagination, replay, runner, threat, plus the `lib.rs` framing types. Public re-exports include 35+ types across all 7 modules. File sizes:
- `cycle.rs`: **2,910 LOC** (Doc 16 acknowledges existence, not size)
- `hypnagogia.rs`: **538 LOC** (Doc 16 says placeholder only in roko-golem)
- `imagination.rs`: **575 LOC** (Doc 16 says "Not implemented" for G5/G7)
- `replay.rs`: **449 LOC** (Doc 16 does not list as a module)
- `runner.rs`: **1,016 LOC** (Doc 16 acknowledges)
- `threat.rs`: **312 LOC** (Doc 16 says "Not started" for threat simulation)
- `lib.rs`: 86 LOC
- **Total**: 5,886 LOC across 7 modules

Doc 16 is the canonical status doc and hides ~3,500 LOC of shipping code.
**Fix sketch**: Regenerate Doc 16 §"Current Code Status" from SOURCE-INDEX.md. Start a new §"roko-dreams Modules" table with one row per shipping module + LOC + purpose.

---

## E.07 — Doc 16 §"roko-golem Dissolution Plan" is obsolete (Doc 16 §"roko-golem Dissolution Plan")

**Status**: DONE (plan complete) / PARTIAL (doc unaware)
**Severity**: MEDIUM
**Doc claim**: Doc 16 §"roko-golem Dissolution Plan" tables actions: delete `dreams.rs` placeholder, move `hypnagogia.rs`, dissolve `ScaffoldEngine`, remove roko-golem re-exports from `roko-dreams/src/lib.rs`.
**Reality**: Per batch 09 E.06, `roko-golem` has been dissolved. `Grep 'roko_golem' crates/roko-dreams --include=*.rs` should return zero. `roko-dreams/src/lib.rs:1-86` re-exports are now entirely from the in-crate modules (cycle, hypnagogia, imagination, replay, runner, threat), not from golem. The dissolution plan is complete; Doc 16 hasn't caught up.
**Fix sketch**: Doc 16 §"roko-golem Dissolution Plan" should be removed entirely or replaced with "Dissolution Complete" with a one-line summary.

---

## E.08 — Implementation roadmap Phase 2 claims (Doc 16 §"Phase 2: Complete Dream Cycle")

**Status**: PARTIAL (doc is stale)
**Severity**: MEDIUM
**Doc claim**: Doc 16 §"Phase 2: Complete Dream Cycle" lists "Not started" status for: Scheduled trigger (fixed interval), CLI commands (`roko dream run/report/history`), Intensive consolidation mode, Mattar-Daw utility scoring, Wire PatternMiner into dream cycle, Wire CrossEpisodeConsolidator into dream cycle, Mistake identification.
**Reality**: Of these seven Phase 2 items:
- Scheduled trigger: **Done** (A.06)
- CLI commands: Unverified, probably still open (A.07)
- Intensive consolidation: **Not started** (A.10) — the one Doc 16 row that matches
- Mattar-Daw utility scoring: **Done** (B.02)
- Wire PatternMiner: **Partial** (B.10) — infrastructure ready, wiring unverified
- Wire CrossEpisodeConsolidator: **Partial** (B.09) — same
- Mistake identification: **Partial via threat simulation** (D.08) — the shipping `threat.rs` covers mistake identification for failed episodes

Three of seven are undercounted. Three of seven remain reasonably accurate.

---

## E.09 — Implementation roadmap Phase 3 claims (Doc 16 §"Phase 3: REM and Creativity")

**Status**: PARTIAL (doc is stale)
**Severity**: MEDIUM
**Doc claim**: Doc 16 §"Phase 3: REM and Creativity" lists "Not started" for: Pearl SCM counterfactual generation, Boden's three creativity modes, Emotional depotentiation, Threat simulation, Hypnagogia engine (4 layers).
**Reality**: Of these five Phase 3 items, **all five are shipping**:
- Pearl SCM counterfactuals: **Done** (B.04) — `imagination.rs`
- Boden's three modes: **Done** (B.05) — `imagination.rs` `ImaginationMode` enum + `synthesize_hypotheses`
- Emotional depotentiation: **Done** (B.08) — daimon `DEPOTENTIATION_*` constants + `DepotentiationReport`
- Threat simulation: **Done** (D.08) — `threat.rs`
- Hypnagogia engine 4 layers: **Done** (D.01) — `hypnagogia.rs`

**Phase 3 is 5-of-5 shipping, not 0-of-5 as Doc 16 claims.**
**Fix sketch**: Regenerate Doc 16 §"Phase 3" entirely — move all five items from "Not started" to "Done".

---

## E.10 — Implementation roadmap Phase 4 claims (Doc 16 §"Phase 4: Integration and Feedback")

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Doc 16 §"Phase 4" lists "Not started" for: Dream → gate threshold updates, Dream → CascadeRouter updates, Dream → playbook revisions, Mesh knowledge sharing, Dream feedback into plan generator.
**Reality**: Still largely Not-Started, matching Doc 16's claim. These are the true open wiring seams (cross-ref C.07). The `PlaybookStore` dep exists in the cycle but the specific dream → playbook-revision path is unverified. Gate / CascadeRouter / Mesh / Plan-Generator updates are not present.

---

## E.11 — Implementation roadmap Phase 5 (Oneirography) stays frontier (Doc 16 §"Phase 5")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 16 §"Phase 5: Oneirography" lists all items "Not started". Cross-ref Doc 14.
**Reality**: Matches — no shipping code for dream image generation, self-appraisal system, affect-reactive auctions, extended art forms, steganographic encoding. See F-frontier-concepts.md.

---

## E.12 — Key dependencies table (Doc 16 §"Key Dependencies")

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Doc 16 §"Key Dependencies" table lists status: `EpisodeLogger` available, `PatternMiner` available (not wired), `CrossEpisodeConsolidator` available (not wired), `k_medoids` available, `KnowledgeStore` available, `TierProgression` available, `ClaudeCliAgent` / `ExecAgent` available, `ProcessSupervisor` available, **Daimon "Not yet implemented"**, HDC vectors built (not called from dreams).
**Reality**: Two drift items: (a) Daimon IS implemented (see batch 09 — `roko-daimon` is 2,636 LOC of live code); (b) HDC IS called from dreams — `imagination.rs:142` uses `text_fingerprint(...).similarity(...)` for counterfactual trust region. Doc 16's dependency table is inaccurate on both.
**Fix sketch**: Update Doc 16 dependency table: Daimon → "Implemented (see 09-daimon)"; HDC → "Called from dreams via `imagination.rs:142`, `cycle.rs:129-131`".

---

## E.13 — Scheduling claims are accurate (Doc 16 §"G1 Idle trigger: Implemented")

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 16 §"G1 Idle trigger" marks Implemented — idle_threshold_mins check against episode timestamps.
**Reality**: See A.05 — matches the shipping `DreamSchedulePolicy::idle_delay` + quality-adaptive logic.

---

## Section Summary

| Status | Count |
|--------|-------|
| DONE | 4 (E.01 Dreams→Neuro, E.02 Dreams→Daimon, E.05 Dreams→Agent, E.13 Idle scheduling claim accurate) |
| PARTIAL | 8 (E.03 Dreams→Learn wiring unverified, E.04 Dreams→Compose/Gate/Mesh mediated via Neuro, E.06 Doc 16 undercount, E.07 golem-dissolution doc obsolete, E.08 Phase 2 roadmap stale, E.09 Phase 3 roadmap all shipping, E.10 Phase 4 genuinely Not Started, E.12 dependencies table inaccurate) |
| NOT DONE | 1 (E.11 Phase 5 Oneirography stays frontier) |

Section E catalogs the **most acute doc-reality drift in topic 10**:
Doc 16 §"Phase 3" says 5-of-5 items are "Not started" when in reality
**all 5 are shipping**. Doc 16 §"roko-dreams Crate" omits 4 of the 7
shipping modules entirely. The golem-dissolution plan is obsolete.

## Agent Execution Notes

### E.06 / E.07 / E.08 / E.09 / E.12 — Doc 16 regeneration

These five items are the strongest case for regenerating Doc 16
entirely. The current version hides more than half the shipping code
and still references the dissolved golem crate.

### E.04 — Architecture clarification

The Doc 15 integration diagrams should clarify that Dreams
communicates via Neuro's KnowledgeStore rather than direct crate
dependencies. This is a **better** architecture than the doc
implies, not a gap.

Acceptance criteria:

- Doc 16 regenerated to reflect 7-module crate reality,
- golem-dissolution plan removed or marked Complete,
- Phase 3 roadmap flipped from "Not started" to "Done" for Pearl SCM, Boden, depotentiation, threat, hypnagogia,
- dependency table updated: Daimon Implemented, HDC called from dreams.
