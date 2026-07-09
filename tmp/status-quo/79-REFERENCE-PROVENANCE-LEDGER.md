# Reference Provenance Ledger

> Status-quo audit · re-verified 2026-07-08 · git HEAD `5852c93c05a4` · scope: `docs/v1/21-references/` (27 files: 00–25 numbered + `INDEX.md`).

`docs/v1/21-references` is a 27-file bibliography and research-provenance corpus (files `00-…` through `25-…` plus `INDEX.md`, re-counted this pass). It explains **why** many mechanisms exist, but it should not decide whether a subsystem is current, shipped, or prioritized without a code/status-pack link. A citation justifies a design; it does not prove current behavior. Provenance tags are used today only in this file and `docs/v1/STATUS.md` — maintained docs elsewhere do not yet tag their v1 citations, so the classification below is the authoritative fence.

## Reference inventory

| File | Topic | Current use | Provenance class |
|---|---|---|---|
| `00-lifecycle-and-finite-agency.md` | Lifecycle and finite agents. | Rationale for lifecycle/mortality work; **not** proof of compile-time type-state (that remains doc-only). | rationale |
| `01-memory-consolidation.md` | Memory consolidation. | Provenance for dreams/neuro; map to `39`, `41`. | implemented-descendant (partial) |
| `02-affective-computing.md` | Affect/PAD/OCC/ALMA. | Provenance for Daimon; map to `56`. | implemented-descendant |
| `03-dreams-and-offline-learning.md` | Dream replay and offline learning. | Provenance for roko-dreams; **trigger wiring still absent** (delta_consumer NOT WIRED). | implemented-descendant (trigger 🔌) |
| `04-coordination-and-multi-agent.md` | Multi-agent coordination. | Rationale for relay/groups/mesh; live transport status in `70`. | rationale / partial |
| `05-biological-analogues.md` | Biological analogues. | Research-only unless tied to an implemented loop. | research-only |
| `06-self-learning-systems.md` | Self-learning and feedback. | Rationale for learn/cascade/experiments; map to `40`. | implemented-descendant |
| `07-context-engineering.md` | Context engineering. | Provenance for compose/prompt layers; map to `34`. | implemented-descendant |
| `08-security-and-provenance.md` | Security and provenance. | Rationale for safety/custody; scope matrix is `75`. | implemented-descendant (partial) |
| `09-hdc-vsa.md` | HDC/VSA. | Rationale for fingerprints/retrieval; HDC enablement partial. | implemented-descendant (partial) |
| `10-market-microstructure.md` | DeFi/market microstructure. | Product-risk input for chain/ISFR; live authority in `42`, `58`. | product-risk |
| `11-streaming-algorithms.md` | Online stats. | Rationale for thresholds/metrics; proof in gates/tests. | implemented-descendant |
| `12-signal-processing.md` | Time series and signal processing. | Research-only until wired consumers exist. | research-only |
| `13-philosophy.md` | Philosophy. | Background only. | research-only |
| `14-agent-harnesses-and-tool-use.md` | Agent harnesses/tool use. | Rationale for provider/tool dispatch; map to `38`. | implemented-descendant |
| `15-cybernetics-and-vsm.md` | Cybernetics/VSM. | Rationale for conductor/feedback loops. | rationale |
| `16-active-inference.md` | Active inference/FEP. | Partial code bridge (`roko-learn/src/active_inference.rs`); full POMDP/EFE remains research. | implemented-descendant (partial) |
| `17-process-reward-models.md` | PRMs and verification. | Rationale for gate/process-reward modules (`roko-gate/src/process_reward.rs:162`). | implemented-descendant |
| `18-collective-intelligence.md` | Collective intelligence. | Rationale for c-factor/groups; live proof partial. | implemented-descendant (partial) |
| `19-regulatory-compliance.md` | Compliance. | Product-risk input; not a shipped compliance program. | product-risk |
| `20-cognitive-architectures.md` | Cognitive architecture. | Background for Cell/Graph/dreams/daimon framing. | rationale |
| `21-mechanism-design.md` | Mechanism design/attention economics. | Rationale for VCG/auction; production selection reachable-but-cold (see `18` VCG note). | implemented-descendant (cold) |
| `22-protocol-standards.md` | Protocol standards. | Input for ACP/MCP/relay alignment. | rationale |
| `23-generational-and-evolutionary.md` | Generational/evolutionary systems. | Rationale for lifecycle/evolution; mostly future-state. | rationale / research-only |
| `24-additions-2025.md` | 2025 additions. | Date-sensitive; verify before reuse. | date-sensitive |
| `25-research-to-runtime.md` | Research-to-runtime pipeline. | Target process; **not a live pipeline**. | research-only |
| `INDEX.md` | Master citation index. | Navigation only. | — |

## Provenance classes

| Class | Meaning | Code-verified examples (this pass) |
|---|---|---|
| Implemented descendant | A cited mechanism has identifiable, current code. | Mattar-Daw replay → `roko-dreams/src/replay.rs`; Woolley c-factor → `roko-learn/src/efficiency.rs`; FrugalGPT cascade → `roko-learn` cascade_router (+ `tests/cascade_router_integration.rs`); Ebbinghaus decay → `roko-core/src/decay.rs` + `demurrage.rs`; PAD affect → `roko-daimon/src/policy.rs` + `roko-core::affect::PadVector`; PRM → `roko-gate/src/process_reward.rs:162`. |
| Rationale source | Explains design choices; does not prove current behavior. | Context engineering, cybernetics/VSM, protocol standards, cognitive architectures. |
| Product-risk source | Useful for legal/economic/security caution. | Compliance, DeFi/market microstructure, security/provenance. |
| Research-only | No current implementation contract. | Philosophy, some biological analogues, signal processing, full active inference, research-to-runtime pipeline. |

## Verification notes (this pass)

- **Implemented-descendant anchors re-checked and hold.** Every "implemented descendant" example above resolves to a real, current source file (grep-verified against HEAD). No stale attributions found in the class table.
- **One caveat on `03-dreams`.** The mechanism (Mattar-Daw priority replay) is coded in `roko-dreams`, but its *runtime trigger* is not live: `roko-runtime/src/delta_consumer.rs` is self-labelled `STATUS: NOT WIRED` with stubbed dream phases. So "dreams" is implemented-descendant for the *algorithm* but 🔌 built-not-wired for the *loop* — do not cite `03` as proof the dream cycle runs on a cadence.
- **`16-active-inference` is a partial bridge, not full FEP.** `roko-learn/src/active_inference.rs` exists (`BeliefState`/`select_tier`) but the full factorized-POMDP/EFE story in the reference remains research.
- **`21-mechanism-design` (VCG) is reachable-but-cold, not blocked.** Corrected from earlier "production selection still blocked": `vcg_allocate` is defined and called (`roko-compose/src/{auction.rs:380,prompt.rs:1213}`), gated by a live strategy branch; density-greedy dominates only because bidders are rarely warmed.

## Checklist

- [ ] **[P2]** When a maintained doc cites a v1 reference, add a provenance tag: `implemented-descendant`, `rationale`, `product-risk`, `research-only`, or `date-sensitive`. Today only this file and `docs/v1/STATUS.md` do so.
- [ ] **[P2]** Tie every `implemented-descendant` citation to a code path AND a status-pack file (the class table above is the seed).
- [ ] For date-sensitive references (`24-additions-2025`, market/competitor material), record source year/date before importing into current docs.
- [ ] Keep philosophy/biology/market material out of "done" criteria unless it has a concrete proof gate.
- [ ] Adopt a docs convergence rule: references justify design; **only tests/code/status-pack files justify current-state claims.**
