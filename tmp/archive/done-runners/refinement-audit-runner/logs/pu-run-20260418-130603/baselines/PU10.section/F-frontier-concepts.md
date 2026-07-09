# F — Frontier Concepts (Docs 10, 11, 14, 17)

Parity of the four frontier chapters: hauntology (Derrida spectral
traces), inner worlds (visual rendering for dream phases), oneirography
(dream art / image generation / NFT minting), and advanced dream
concepts (dream sharing, nightmare containment, lucid-dream
monitoring).

These chapters are the most explicitly design-only in topic 10. Each
carries rich academic citations and deep narrative, but the shipping
`roko-dreams` crate implements none of them directly.

Generated 2026-04-16.

---

## F.01 — Hauntology / spectral traces are pure design (Doc 10 §"Derrida Hauntology")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 10 (151 lines) draws on Derrida 1993 *Specters of Marx* to describe "hauntology" — spectral traces of past agents persist across generations. Knowledge transfer as backup/restore. Compound escape from monoculture.
**Reality**: `Grep 'hauntology|spectral|Derrida|ghost_trace' crates/roko-dreams --include=*.rs` returns zero matches. No shipping code for spectral traces or generational knowledge inheritance. The concept is a design rationale for why dream-consolidated insights should persist even when the originating agent terminates.
**Fix sketch**: Doc 10 should carry a `Design — Phase 2+ / Design essay` banner. The closest shipping surface is `roko-neuro`'s persistent KnowledgeStore, which does preserve insights after the producing agent exits — but the explicit "hauntology" framing is a design lens, not a type.

---

## F.02 — Compound escape from monoculture (Doc 10 §"Compound Escape from Monoculture")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 10 §"Compound Escape from Monoculture" argues that cross-agent knowledge diversity prevents the whole mesh from converging on a single (possibly wrong) worldview. Divergent agents carry spectral traces that contradict the mainstream.
**Reality**: No mechanism enforces diversity at the mesh level. The per-agent 15% contrarian retrieval (batch 09 C.06, `CONTRARIAN_FRACTION = 0.15`) is the nearest shipping cousin — it prevents *per-agent* echo chambers but not mesh-level monoculture.

---

## F.03 — Knowledge transfer as backup/restore (Doc 10 §"Knowledge Transfer")

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Doc 10 §"Knowledge Transfer" describes copying / restoring knowledge state across agents as a way to preserve "ghosts" of past agents.
**Reality**: `roko-neuro`'s KnowledgeStore is JSONL-backed + tier-aware (per batch 09 D.04 emotional-provenance transfer). The store can in principle be copied between agents (file-level), but there is no explicit "ghost transfer" protocol. Infrastructure present; mesh-level transfer is frontier.

---

## F.04 — Visual rendering for each dream phase (Doc 11 §"NREM Theater", §"REM Garden", §"Hypnagogia Phosphenes", §"Integration Crystallization")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 11 (192 lines) describes four phase-specific visualizations: NREM theater (episode replay as theatrical scenes), REM garden (counterfactual branches as botanical growth), hypnagogia phosphenes (low-confidence fragments as retinal phosphene patterns), integration crystallization (consolidated insights as crystal lattice).
**Reality**: `Grep 'inner_world|theater|garden|phosphene|crystallization|render_dream' crates/ --include=*.rs` returns zero matches. No visualization infrastructure. This is "dream art" UI work, explicitly Phase 5 in Doc 16's roadmap.
**Fix sketch**: Doc 11 should carry a `Design — Phase 2+` banner.

---

## F.05 — Oneirography dream art pipeline is absent (Doc 14 §"Image Generation Pipeline", Doc 16 §"Phase 5")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 14 (604 lines) describes a dream → image pipeline: dream content → prompt synthesis → image generation (external model) → self-appraisal score → affect-reactive auction → optional NFT minting on Korai chain.
**Reality**: `Grep 'oneirography|dream_image|self_appraisal|dream_auction' crates/roko-dreams --include=*.rs` returns zero matches. Doc 16 §"Phase 5: Oneirography" lists all items "Not started" (E.11 cross-ref).
**Fix sketch**: Doc 14 stays `Design — Phase 2+`. Cross-link to the Korai chain layer (batch 08) — NFT minting is a Tier-6 chain concern.

---

## F.06 — Self-appraisal, affect-reactive auctions, extended art forms (Doc 14 §"Self-Appraisal", §"Affect-Reactive Auctions")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Self-appraisal scores dream outputs against expected properties; affect-reactive auctions use current PAD state to weight bidding on which dream outputs get retained / published.
**Reality**: Cross-ref batch 09 D.06 — VCG auction scaffolding exists in `roko-compose` for prompt composition, but dream-specific image auctions are separate and frontier.

---

## F.07 — Steganographic encoding (Doc 14 §"Steganographic Encoding", Tancik 2020 StegaStamp)

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 14 references StegaStamp (Tancik et al. CVPR 2020) for encoding provenance in generated dream images.
**Reality**: No image generation, so no steganography. Frontier.

---

## F.08 — Dream sharing across mesh is absent (Doc 17 §"Dream Sharing")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 17 (674 lines) describes cross-agent dream insight sharing via the agent mesh. Dreams produced by one agent become available to peers for validation.
**Reality**: No mesh protocol exists for dream insights specifically. The shipping `roko-neuro` KnowledgeStore is per-agent. Cross-ref batch 08 C.08 (local `InsightBus` / `PheromoneBus` as intra-process pub/sub precursor) — but no inter-agent dream transport.

---

## F.09 — Nightmare detection and containment (Doc 17 §"Nightmare Detection")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 17 §"Nightmare Detection" describes detecting and quarantining high-arousal / high-threat dream outputs that should not enter the validated knowledge store. Uses affect state + threat severity to classify.
**Reality**: The shipping `threat.rs::threat_warning_entries` (D.08) applies a `severity >= 0.20` filter before emitting warning entries — this is the simplest "nightmare threshold" mechanism. The more elaborate "quarantine" / "containment" surface (with human review, rollback, governance) is frontier.
**Fix sketch**: Doc 17 §"Nightmare Detection" should cross-link to the shipping threat severity filter as a minimal quarantine mechanism. Full containment flow is Phase 2+.

---

## F.10 — Persistent dream journals (Doc 17 §"Persistent Dream Journals")

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Doc 17 §"Persistent Dream Journals" describes per-agent dream journals — a chronological log of dreams, insights, and validation outcomes.
**Reality**: Dream reports persist to `.roko/dreams/dream-{timestamp_ms}.json` (A.11) and `load_latest_dream_report()` can retrieve them. This is a minimal journal. The richer journal (with cross-linking, themes, recurrent patterns) is frontier.

---

## F.11 — Lucid dream monitoring (Doc 17 §"Lucid Dream Monitoring", Filevich 2015)

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 17 cites Filevich et al. 2015 on metacognitive mechanisms in lucid dreaming. Describes a meta-awareness layer that observes dream content in progress and can intervene if the dream diverges too far from useful territory.
**Reality**: `Grep 'lucid|meta_awareness|dream_monitor' crates/roko-dreams --include=*.rs` returns zero matches. No meta-awareness layer. The dream cycle runs to completion without runtime introspection. The `HomuncularObserver` in hypnagogia (D.01) is a rough post-hoc observer, but not the online metacognitive monitoring Doc 17 describes.

---

## F.12 — EEG microstate analogies, Tononi SHY, prefrontal regulation (Doc 17 §"Recent Neuroscience")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 17 and Doc 04 reference Tononi lab 2024 Synaptic Homeostasis Hypothesis (SHY) with SYNCit-K causal demonstration, Sawada et al. Science 2024 prefrontal synaptic regulation, PP2Acα *Communications Biology* 2025 phosphatase regulation, EEG microstates in lucid REM.
**Reality**: These are academic citations used as design rationale for the depotentiation + replay mechanisms. No direct neuroscience-to-code mapping beyond the arousal depotentiation constants in daimon (batch 09 C.07) and the replay utility formulas (B.02). Informational.

---

## Section Summary

| Status | Count |
|--------|-------|
| DONE | 0 |
| PARTIAL | 2 (F.03 knowledge-transfer infrastructure present, F.10 minimal dream journals via report persistence) |
| NOT DONE | 10 (F.01 hauntology, F.02 monoculture escape, F.04 inner worlds, F.05 oneirography pipeline, F.06 self-appraisal / auctions, F.07 steganography, F.08 dream sharing, F.09 full nightmare containment, F.11 lucid monitoring, F.12 recent neuroscience references) |

Section F is uniformly frontier — these four docs (10, 11, 14, 17)
exist primarily to sketch the full imagined future of the dreams
subsystem: how dreams participate in cross-generational knowledge
preservation, how dreams render visually, how dream outputs become
tradeable art, and how meta-awareness might observe dreams in
progress. None of it ships; none of it needs to ship for the
self-hosting loop.

## Agent Execution Notes

### F.01-F.12 — Frontier banner pass

The simplest action for all of section F is applying uniform
`Design — Phase 2+` banners to Docs 10, 11, 14, 17. Doc 17 alone
is 674 lines of design exploration; each major section should carry
the banner.

### F.03 / F.10 — Acknowledge minimal shipping precursors

Two shipping surfaces deserve mention:
- `roko-neuro` KnowledgeStore as the minimal "generational
  knowledge preservation" surface (F.03).
- `.roko/dreams/dream-*.json` reports as the minimal "dream
  journal" (F.10).

Neither is what the docs fully describe, but both are the natural
extension points if the frontier ever gets implementation pressure.

### F.09 — Threat severity is the shipping nightmare filter

Cross-link Doc 17 §"Nightmare Detection" to the `severity >= 0.20`
filter in `threat.rs::threat_warning_entries` as a minimal
quarantine primitive (see D.08).

Acceptance criteria:

- Docs 10, 11, 14, 17 carry uniform Phase 2+ banners,
- minimal shipping precursors (KnowledgeStore, dream reports, threat
  severity filter) are cross-linked from each doc where applicable,
- no code work triggered by this section.
