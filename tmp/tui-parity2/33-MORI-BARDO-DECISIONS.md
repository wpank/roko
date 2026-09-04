# Mori/Bardo rendering decisions

Mori is the visual baseline; Bardo's PRDs supply intent and component vocabulary. Roko should not
copy every decorative technique. The acceptance criterion is operational legibility during long,
high-volume plan runs.

## Adopt

| Pattern | Roko decision |
|---|---|
| True-black canvas and quiet raised surfaces | Use VOID as the canvas, one-cell gutters, and restrained panel backgrounds so hierarchy comes from spacing before borders. |
| One dominant information surface | Keep the selected agent transcript or active plan output visually dominant; compress metadata/roster/metrics around it. |
| Semantic transcript structure | Preserve tool calls, tool output, assistant text, role/attempt, context, and conductor intervention as visually distinct segments. |
| Stable chrome | Header, view navigation, warnings, and footer must remain readable and must not be overwritten by effects, modals, or toasts. |
| State expressed through color/background | Prefer subtle foreground lift, background tint, progress, and small status glyphs over screen-wide character fields. |
| Responsive disclosure | Wide terminals show full tab/navigation labels and supporting tables; narrow terminals retain the active view and essential status, then drop secondary columns/chrome. |
| Explicit empty/loading/error states | Distinguish “not started,” “warming cache,” “waiting on provider,” “tool running,” “gating,” “completed,” and “data unavailable.” |
| Evidence through the production renderer | Headless and continuous captures must call the same `App::draw` path as the terminal rather than a parallel content-only renderer. |

## Adapt

| Mori/Bardo pattern | Adaptation |
|---|---|
| Atmospheric motion | Minimal is the default and contains no glyph particles. Full is opt-in, background-only, sparse, and protected by a true master switch/reduced-motion mode. |
| Dense multi-panel layouts | Retain Roko's ten operational tabs, but add width/height breakpoints and prioritize the active panel rather than preserving every column. |
| Persistent intervention prompts | Use bounded toasts and a dedicated warning bar; transient notices must not consume most of a 24-row terminal or cover the footer. |
| Rich topology/graph displays | Show only authored/runtime relationships. Do not present synthetic co-plan cliques or a flat Wave 1 as authoritative topology. |

## Reject

- Character rain, braille bands, guide lines, or particles drawn into panel whitespace at normal
  settings. Blank cells are layout, not spare pixels.
- Effects rendered after modal content or over operational glyphs.
- Decorative subtab labels without reachable input and independent state.
- Tables that keep every column at the cost of truncating the identity, status, or active output.
- “Live” labels for values loaded from disk, inferred from capacity, or delivered only at completion.
- UI controls that optimistically change a badge but have no runner acknowledgement or effect.

## Implemented baseline

`88996a418` establishes the visual hierarchy and protects content from the catastrophic Full-preset
overlay. `fced716b6` makes Off a genuine master switch, removes particles from Minimal, preserves
operator TOML comments while cycling presets, compacts the global header, and makes previously
hidden subviews visible. Full remains an explicit diagnostic/flourish mode; the local uncommitted
`roko.toml` currently selects it and therefore does not represent the default experience.

The Phase 2 responsive and toast passes are evidence-driven follow-ups to this baseline, not a
claim that every view now matches Mori's information density.
