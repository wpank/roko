# E — Collective Contagion and Current Status (Docs 12, 13)

Parity of the collective-contagion chapter (Doc 12) and the
current-status-and-gaps doc (Doc 13).

Doc 12 is entirely frontier. Doc 13 is the topic-09 self-audit and
matches the code more accurately than any other status doc in the
parity batches so far — one of the few "pre-audited" chapters in the
repo.

Generated 2026-04-16.

---

## E.01 — Inter-agent emotional contagion is unimplemented (Doc 12 §"Emotional Contagion Across Agent Mesh")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 12 describes emotional contagion across the agent mesh: nearby agents synchronize PAD with attenuation. Specific parameters:
- P (pleasure) / A (arousal) attenuation: **0.3** per sync hop
- D (dominance) attenuation: **0.0** (dominance does not propagate)
- Arousal cap: **+0.3** per sync cycle
- Unidirectional propagation enforced
- 6-hour borrowed-emotion decay
**Reality**: `Grep 'contagion\|Contagion\|emotional_sync\|PAD_sync\|borrowed_emotion' crates/ apps/ --include=*.rs` returns zero matches in the active codebase (two hits in `apps/mirage-rs/src/{fork, replay}.rs` are the word used in an unrelated event-contagion context, not emotional). No agent-mesh sync primitive for emotion exists. Doc 13 §"Unimplemented Features — Collective Contagion" lists every sub-item as not done.
**Fix sketch**: Doc 12 stays `Design — Phase 2+ Tier 2M`. A minimal first implementation would be a per-turn "inbox" on `AffectState` that receives P/A deltas from peers and applies them with attenuation + arousal cap. Propagation unidirectionality can piggyback on the `InsightBus` pub/sub (see batch 08 C.08).

---

## E.02 — Somatic field / C-Factor aggregation are unimplemented (Doc 12 §"Somatic Field Formation", §"C-Factor")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 12 §"Somatic Field" describes a mesh-aggregated somatic landscape (collective knowledge of "which strategy regions produced good / bad outcomes" across all agents). The C-Factor (collective intelligence factor, Woolley et al. 2010) quantifies how well the mesh's aggregate emotional + cognitive state predicts group performance.
**Reality**: No shipping code. The shipping somatic landscape (see C.05-C.08) is per-agent only. `Grep 'c_factor\|CFactor\|collective_intelligence\|somatic_field' crates/ apps/ --include=*.rs` returns zero matches.

---

## E.03 — Anti-cascade design is specification-only (Doc 12 §"Anti-Cascade Design")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 12 §"Anti-Cascade Design" specifies mechanisms to prevent runaway emotional cascades across the mesh: arousal cap, per-cycle sync limit, unidirectional hop enforcement, 6-hour decay. Ensures `n` agents don't spontaneously panic together.
**Reality**: No contagion exists (E.01), so no anti-cascade design is needed. If contagion is implemented, the parameters at E.01 provide the full anti-cascade envelope.

---

## E.04 — Stigmergy (Grassé 1959) as mesh-mediated coordination is absent (Doc 12 §"Stigmergy via Somatic Field")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 12 draws on Grassé 1959 stigmergy (indirect coordination through environmental traces) — mesh-wide somatic field acts as the shared environment agents read and write. No direct agent-to-agent messaging; all coordination flows through the field.
**Reality**: The local `InsightBus` / `PheromoneBus` primitive at `apps/mirage-rs/src/roko_bridge/subscription/` (see batch 08 C.08) is the nearest shipping ancestor of a stigmergy-style pub/sub, but it is **intra-process**, not cross-agent. Cross-link: batch 08 Doc 07 §"Stigmergy" and topic 10-dreams §"Consolidation Cycles" share the same theme.

---

## E.05 — Doc 13 is the most accurate status doc in this parity batch (Doc 13 §"Implemented Components", §"Scaffolded", §"Specified but Not Implemented")

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 13 catalogs every daimon component with accurate status: Complete / Partial / Scaffolded / Specified-but-unimplemented.
**Reality**: Doc 13's tables at `:26-47` map to the actual shipping code at `roko-daimon/src/lib.rs:*` accurately at the entry level. Spot checks confirming the doc is trustworthy:
- "F1 `PadVector` struct — Done" → `roko-core/src/affect.rs:7-14` ✓
- "F2 8 octant states — Done (roko-golem)" — correctly flagged as legacy (see A.05)
- "F3 `AffectEvent` and `AffectEngine::appraise()` — Done" → `roko-daimon/src/lib.rs:1392-1439, 1635+` ✓
- "F4 Temporal decay (exponential, 4h half-life) — Done" → `roko-daimon/src/lib.rs:71-85` ✓ (see A.07)
- "F5 Behavior modulation — Done" → `DispatchStrategy` + `DispatchParams` + `modulate` ✓ (see B.05)
- "F6 Affect signatures on episodes — Partial" — matches D.02-D.03 findings ✓
- "F7 Affect → SystemPromptBuilder — Done" → `roko-compose/src/system_prompt_builder.rs` ✓ (see D.11)
- "F8 Affect → CascadeRouter — Done" → `DaimonPolicy` in `roko-learn/src/cascade_router.rs` ✓ (see B.06)
- "F9 Persistence — Done" → `load_or_new` + atomic write ✓
- "Somatic query + modulation — Partial" — matches C.06 ✓
- "Collective Contagion — Not started" — matches E.01-E.04 ✓
- "VCG Auction Integration — Partial" — matches D.06 ✓
- "Per-crate confidence — Not done" — matches D.08 ✓
- "Fatigue detection — Not done" — matches D.10 ✓

Doc 13 is the canonical status source for topic 09 and does not need major regeneration. Minor refinement: the "2D+ ... the remaining gap is somatic-landscape-backed retrieval plus broader cross-subsystem weighting" sentence could reference the specific D entries (D.02 four-factor cross-subsystem, D.06 VCG externality ledger).

---

## E.06 — Tier 0C (roko-golem dissolution) is complete (Doc 13 §"Implementation Priority Path — Tier 0C")

**Status**: DONE
**Severity**: —
**Doc claim**: "Tier 0C: Dissolve roko-golem, consolidate affect logic into roko-daimon — Complete."
**Reality**: `ls crates/ | grep -i golem` returns empty (no `roko-golem` crate). All affect logic lives in `roko-daimon` + `roko-core/src/affect.rs`. The previous `roko-golem/src/daimon.rs::AffectOctant` 8-variant enum did not migrate (see A.05); that is the only observable effect of the dissolution. Doc 13 accurately flags this.

---

## E.07 — Priority tier path alignment (Doc 13 §"Implementation Priority Path")

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 13 §"Implementation Priority Path" tables Tier 0C through Tier 2M with status labels. Recommended next steps are listed at the end.
**Reality**: The tier table at Doc 13 `:152-162` is accurate against parity sections A-D:
- Tier 0C: Complete (E.06)
- Tier 2D (F1-F5, F9): Complete (A-B)
- Tier 2E (F5): Complete (B.05)
- Tier 2D+ (F6, F7, F8): Mostly complete (B.06, D.02, D.11)
- Tier 2G (somatic, 8D, k-d tree): Partial (C.01-C.11 — mostly done; C.03/C.09 the remaining gaps)
- Tier 2H (emotional memory): Partial (D.02-D.04)
- Tier 2I (dream-daimon bridge): Partial (C.07, D.04)
- Tier 2M (collective contagion): Not started (E.01-E.04)

The "Recommended Next Steps" at Doc 13 `:163-171` are actionable: deepen somatic semantics, finish emotional-memory scoring, deepen VCG affect bidding, layer in collective contagion + frontier appraisal triggers. These align with the PARTIAL entries in B, C, D sections.

---

## E.08 — Skipped legacy mortality files are correctly documented (Doc 13 §"Skipped Legacy Files")

**Status**: DONE
**Severity**: —
**Doc claim**: Two legacy files skipped: `bardo-backup/prd/03-daimon/04-mortality-daimon.md` (mortality-specific emotional mapping: Economic Anxiety, Epistemic Vertigo, Stochastic Dread) and `05-death-daimon.md` (death protocol, thanatopsis, emotional life review). Only preserved principle: resource scarcity creates emotional pressure.
**Reality**: Matches A.01 (mortality framing removed). The `Grep` for 'mortality' / 'death' / 'dying' in `crates/` / `apps/` returns zero hits in active `.rs` code, confirming both files' concepts did not leak into the implementation. MEMORY.md also records "Death concepts removed" as a user-pinned invariant.

---

## Section Summary

| Status | Count |
|--------|-------|
| DONE | 4 (E.05 Doc 13 accuracy, E.06 Tier 0C complete, E.07 priority tier alignment, E.08 skipped legacy files) |
| PARTIAL | 0 |
| NOT DONE | 4 (E.01 contagion, E.02 somatic field / C-Factor, E.03 anti-cascade, E.04 stigmergy) |

Section E is the simplest part of topic 09: Doc 12 is entirely
Tier 2M frontier (collective contagion has not started), and Doc 13
is the most accurate self-audit in any topic this batch survey has
reviewed.

## Agent Execution Notes

### E.01-E.04 — Collective contagion stays frontier

No code work is warranted until the single-agent affect path is fully
exploited (per Doc 13 §"Recommended Next Steps" #4). Banner both Doc
12 and related sub-sections as `Design — Phase 2+ Tier 2M`.

### E.05-E.08 — Doc 13 is already trustworthy

Doc 13 does NOT need regeneration the way Doc 08-chain Doc 24 does.
The main polishing is to cross-link specific PARTIAL entries in B /
C / D to the "Recommended Next Steps" list in Doc 13.

Keep legacy `roko-golem` mentions in Doc 13 only as migration history.
They should not read like live source dependencies for the active
runtime.

Acceptance criteria for this section:

- Doc 12 is banner-tagged Phase 2+ Tier 2M,
- Doc 13's "Recommended Next Steps" reference the specific parity
  entries (D.02 four-factor, D.06 VCG externality, C.03 domain-native
  extractors) where applicable.
