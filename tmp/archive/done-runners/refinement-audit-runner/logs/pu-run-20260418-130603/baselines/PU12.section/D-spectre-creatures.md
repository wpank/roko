# D — Spectre Creature Visualization (Docs 10, 11, 12)

Parity of three Spectre-focused chapters: Spectre creature
visualization (928 lines), Spectre rendering per interface (614
lines), Spectre as collective display (650 lines).

Section D is **almost entirely frontier**. `Grep 'Spectre\|
SpectreRenderer\|spectre_creature' crates/` returns **zero matches**
for the Spectre visualization surface (the shipping `rosedust` hits
found earlier were all the color-palette theme, not Spectre). The
2,192 lines of Spectre specification describe future creature-style
agent visualizations, per-interface rendering (TUI ASCII / web canvas
/ portal SVG), and collective mesh displays — none of which ship.

Generated: 2026-04-16.

---

## D.01 — Spectre creature concept (Doc 10 §"Overview")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 10 describes "Spectre" as a creature-style visual representation of each running agent: body color = PAD pleasure, size = knowledge-entry count, breathing rate = arousal, edges to peers, etc.
**Reality**: `Grep 'Spectre\|SpectreRenderer\|spectre_creature\|SpectreDensity' crates/ --include=*.rs` returns zero matches. The concept is entirely design.
**Fix sketch**: Doc 10 §"Overview" should carry `Design — Phase 2+` banner.

---

## D.02 — PAD → creature visual mapping (Doc 10 §"PAD to Visuals")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 10 §"PAD to Visuals" specifies body-color = pleasure, size = dominance, breathing = arousal, edge-glow = confidence.
**Reality**: Batch 09 confirms PAD state ships + `DaimonPolicy` consumed across 10 crates. But no visual renderer turns PAD into creature graphics. The nearest shipping surface is the status-bar + token-sparkline widgets in TUI (C.04) which display numeric PAD / affect state text, not creature graphics.

---

## D.03 — Behavioral state → creature pose (Doc 10 §"Behavioral State")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 10 maps the 6 behavioral states (Engaged / Struggling / Coasting / Exploring / Focused / Resting) to creature poses.
**Reality**: Behavioral state ships in `BehavioralState::classify` (batch 09 B.04) but no pose-renderer. Frontier.

---

## D.04 — TUI ASCII Spectre rendering (Doc 11 §"TUI Rendering")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 11 describes ASCII-art Spectre rendering for the TUI — multi-line creature with breathing animation.
**Reality**: No ASCII-Spectre renderer in `tui/widgets/`. `widgets/rosedust.rs` is 9 LOC of color constants, not a creature renderer.

---

## D.05 — Web canvas Spectre rendering (Doc 11 §"Web Canvas")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 11 §"Web Canvas" describes Canvas/WebGL Spectre rendering with `aria-label` alt text (cross-ref Doc 17 §"Perceivable").
**Reality**: No web portal ships (see E.01). Frontier.

---

## D.06 — Portal SVG Spectre rendering (Doc 11 §"Portal SVG")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 11 describes SVG-based Spectre for the web portal with animated transitions.
**Reality**: Follows from D.05 — no portal, no SVG renderer.

---

## D.07 — Spectre as collective display: C-Factor + mesh topology (Doc 12 §"Collective")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 12 (650 lines) describes using Spectres to visualize the agent mesh: C-Factor (collective intelligence factor), edges between cooperating agents, emotional contagion flows.
**Reality**: Cross-ref batch 09 E.01-E.04 — collective contagion + C-Factor are Tier-2M frontier. Visualization is downstream of the absent underlying mesh. Frontier stacked on frontier.

---

## D.08 — Somatic field / stigmergy visualization (Doc 12 §"Somatic Field Visualization")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 12 describes visualizing the mesh-wide somatic field as a heatmap or isoline overlay on the Spectre display.
**Reality**: Somatic field is Tier-2M (batch 09 E.02) — no field data to visualize. Frontier.

---

## D.09 — Spectre as explanatory interface (Doc 10 §"Explanation", Doc 12 §"Narration")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Docs 10 + 12 describe Spectre as a learnable interface — users build intuition for agent internals by watching creature behavior.
**Reality**: No creature → no explanatory interface. Frontier.

---

## Section Summary

| Status | Count |
|--------|-------|
| DONE | 0 |
| PARTIAL | 0 |
| NOT DONE | 9 (D.01-D.09 all frontier) |

Section D is **uniformly frontier**. The 2,192-line Spectre
specification describes a rich creature-visualization system none of
which ships. The underlying data (PAD state, behavioral state,
agent mesh, C-Factor, somatic field) is partially present in various
states (PAD shipping; mesh frontier; C-Factor frontier). The
rendering layer is absent.

## Agent Execution Notes

### D.01-D.09 — Uniform frontier banner pass

All three Spectre docs (10, 11, 12) should carry prominent
`Design — Phase 2+ / Tier 2M` banners. Doc 12 specifically depends
on collective contagion (batch 09 E.01-E.04) + mesh topology, both
of which are themselves frontier.

The **dependency chain** is important: Spectre visualization → mesh
topology + contagion → collective coordination primitives. If
higher-priority work lands in the collective mesh, Spectre
visualization becomes a natural follow-on. Until then, the 2,192
lines of design are aspirational.

Acceptance criteria:

- Docs 10 / 11 / 12 carry uniform Phase 2+ banners,
- Doc 11 §"TUI Rendering" cross-links to shipping status-bar + token-sparkline as the closest shipping data surfaces (text-only, not creature),
- Doc 12 §"Collective" cross-links to batch 09 E.01-E.04 as the dependency gate.
