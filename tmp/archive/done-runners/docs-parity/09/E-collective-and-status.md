# E — Collective Contagion and Current Status (Docs 12, 13)

Parity of the collective-contagion chapter (Doc 12) and the
current-status self-audit (Doc 13).

Doc 12 is still a frontier design note, not a shipped runtime surface.
Its current `> **Implementation**: Built` banner at
`docs/09-daimon/12-collective-emotional-contagion.md:6` is stale and
should be read against its own status block at `:253-259`, which says
the feature set is not implemented. Doc 13 is the opposite: it is the
most trustworthy status doc in topic 09, and the remaining work there
is mostly polish, cross-linking, and making sure any `roko-golem`
references read as migration history rather than live runtime
ownership.

Generated 2026-04-18.

---

## E.01 — Doc 12 should be treated as Tier 2M frontier, not “Built” (Doc 12 header, §"Current Status and Gaps")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 12 currently opens with `> **Implementation**: Built`
at `docs/09-daimon/12-collective-emotional-contagion.md:6`, then later
states the exact opposite at `:253-259`: no contagion code exists, the
mesh is not built, somatic-field aggregation is absent, and C-Factor is
absent.
**Reality**: The later status block is the honest one. There is no
shipping inter-agent affect path in the active tree. Read the entire
chapter as `Design — Phase 2+ Tier 2M frontier`.
**Fix sketch**: Re-banner Doc 12 and any summaries that quote it. Keep
the mechanism details as design inputs, not present-tense runtime
descriptions.

---

## E.02 — Inter-agent emotional contagion is unimplemented (Doc 12 §"Contagion Rules", §"Contagion Triggers")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 12 specifies cross-agent emotional propagation with
concrete parameters:
- P / A attenuation: **0.3**
- D attenuation: **0.0**
- Arousal cap: **+0.3** per sync cycle
- Unidirectional propagation
- 6-hour borrowed-emotion decay
**Reality**: `rg -n "contagion|Contagion|borrowed_emotion|emotional_sync|PAD_sync" crates apps -g '*.rs'`
finds no active emotional-contagion runtime. The only live affect
engine is single-agent `roko-daimon`; no mesh sync primitive exists for
PAD propagation.
**Fix sketch**: Keep the parameter table, but label the whole mechanism
as deferred Tier 2M work until a real mesh transport and affect inbox
exist.

---

## E.03 — Somatic field and C-Factor remain design-only (Doc 12 §"Somatic Field Formation", §"C-Factor Integration")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 12 describes a mesh-level somatic field and a
collective-intelligence C-Factor built on shared strategy outcomes and
collective vigilance.
**Reality**: The shipped somatic system is still local to a single
`DaimonState`. `crates/roko-daimon/src/lib.rs:1084-1350` defines a
persisted per-agent `SomaticLandscape`; there is no cross-agent
aggregator. `rg -n "c_factor|CFactor|somatic_field|collective_intelligence" crates apps -g '*.rs'`
returns no production implementation for the Doc 12 concepts.

---

## E.04 — Anti-cascade and stigmergy are specification-only because contagion itself does not ship (Doc 12 §"Anti-Cascade Design", cross-ref §"Somatic Field Formation")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 12 defines anti-cascade protections and mesh-level
stigmergy through a shared somatic field.
**Reality**: No contagion implementation means there is nothing yet to
cap, decay, or make unidirectional. The nearest real analogue is the
single-process signal/prompt plumbing around somatic retrieval and
context assembly, not a multi-agent stigmergic field.
**Fix sketch**: Keep the safeguards as future acceptance criteria for
Tier 2M rather than describing them as live behavior.

---

## E.05 — Doc 13 is trustworthy and mostly needs polish, not regeneration (Doc 13 §"Implemented Components", §"Specified but Not Implemented", §"Recommended Next Steps")

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 13 (`docs/09-daimon/13-current-status-and-gaps.md`)
catalogs what is complete, partial, and still deferred.
**Reality**: This is accurate in the places that matter:
- `:20-47` correctly describes the live `roko-daimon` core and partial
  somatic / emotional-memory surfaces.
- `:64-131` correctly keeps collective contagion, C-Factor, per-crate
  confidence, and fatigue in the unimplemented bucket.
- `:148-171` gives the right tier ordering and next-step priorities.

The remaining issues are polish-level:
- The F2 row at `:73` should read as legacy provenance only. The active
  runtime does **not** ship `AffectOctant`; the live runtime ships
  `BehavioralState` in `crates/roko-core/src/affect.rs:87-158`.
- The F6 / Tier 2D+ / “Recommended Next Steps” wording can point more
  explicitly to the real remaining gaps: somatic-backed retrieval,
  broader cross-subsystem weighting, richer VCG externality policy, and
  domain-native strategy extractors.

Doc 13 is therefore the canonical status baseline for topic 09.

---

## E.06 — Tier 0C is complete, and `roko-golem` should be described only as migration history (Doc 13 §"Removed legacy affect implementation", §"Implementation Priority Path")

**Status**: DONE
**Severity**: —
**Doc claim**: Tier 0C consolidation is complete.
**Reality**: Correct. There is no active `roko-golem` crate in
`crates/`, and the live affect contracts are `roko-core` plus
`roko-daimon`. Current runtime ownership sits at:
- `crates/roko-core/src/affect.rs:7-189`
- `crates/roko-daimon/src/lib.rs:57-1797`

The cleanup needed here is rhetorical, not architectural: any mention
of `roko-golem` in Doc 13 should read as “where this idea used to live”
rather than “what the current runtime still depends on.”

---

## E.07 — The priority path is still aligned with the codebase (Doc 13 §"Implementation Priority Path")

**Status**: DONE
**Severity**: —
**Doc claim**: Tier statuses at `docs/09-daimon/13-current-status-and-gaps.md:152-171`
show 2D / 2E complete, 2D+ / 2G / 2H / 2I partial, and 2M not started.
**Reality**: That matches the code and the earlier parity sections:
- Tier 2D / 2E: live PAD appraisal, behavioral-state classification,
  and dispatch modulation ship now.
- Tier 2D+: mostly done through emotional tagging, prompt affect
  guidance, and router policy wiring.
- Tier 2G / 2H / 2I: partial through local somatic memory, retrieval
  biasing, and dream depotentiation.
- Tier 2M: not started beyond design prose.

The recommended sequence is still sensible: finish exploiting the
single-agent path before taking on collective contagion.

---

## E.08 — The skipped mortality files are still correctly quarantined (Doc 13 §"Skipped Legacy Files")

**Status**: DONE
**Severity**: —
**Doc claim**: The mortality-specific legacy files were deliberately
excluded, preserving only the generalized “scarcity creates pressure”
principle.
**Reality**: Correct. This aligns with topic 09’s broader mortality
cleanup and with the absence of mortality/death framing in the active
Rust affect runtime.

---

## Section Summary

| Status | Count |
|--------|-------|
| DONE | 4 (E.05 Doc 13 trustworthiness, E.06 Tier 0C complete, E.07 priority-path alignment, E.08 skipped legacy files) |
| PARTIAL | 0 |
| NOT DONE | 4 (E.01 stale Doc 12 “Built” banner, E.02 contagion runtime, E.03 somatic field / C-Factor, E.04 anti-cascade / stigmergy) |

Section E divides cleanly into two buckets:

- Doc 12 is frontier Tier 2M design material and should be bannered that way.
- Doc 13 is already the honest status document for topic 09; it mostly needs wording polish and sharper cross-links, not a wholesale rewrite.

## Agent Execution Notes

### E.01-E.04 — Keep Doc 12 in the design lane

Do not let the detailed parameter tables in Doc 12 imply shipped
runtime behavior. The design is useful, but it is still contingent on a
future mesh layer plus explicit contagion state handling.

### E.05-E.08 — Use Doc 13 as the canonical status anchor

Treat Doc 13 as the current truth source for topic 09 status work.
Polish it by:

- keeping `roko-golem` references historical only,
- tightening the F2/F6/Tier 2D+ wording,
- and cross-linking the recommended next steps to the concrete partial
  gaps already identified in sections C and D.

Acceptance criteria for this section:

- Doc 12 is called out as frontier / Tier 2M, not “Built”.
- Doc 13 is described as trustworthy and mostly polish-level work.
- `roko-golem` is described as dissolved migration history, not active
  runtime ownership.
