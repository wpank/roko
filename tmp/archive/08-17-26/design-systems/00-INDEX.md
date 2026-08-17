# Design Systems -- Consolidated Prompt Documents

> **What is this?** ROSEDUST is roko's design system -- color palette, typography,
> component library, and atmospheric effects used in the demo web app (`demo/demo-app`)
> and referenced by the ratatui TUI (`crates/roko-cli/src/tui/`). These documents are
> prompt-engineering references: feed them to an LLM when generating ROSEDUST-themed
> HTML sites, dashboards, or interactive visualizations.
>
> **Adoption status (2026-08-13):** ROSEDUST is heavily adopted in the demo-app
> (~1,570 occurrences of ROSEDUST tokens). TUI theme colors are slightly off from
> the spec defined here -- alignment is a known gap. No MoriTheme alias remains
> in these docs (the project was renamed from mori to roko; see note on file 05).

Documents for generating Three.js / WebGL interactive documentation sites using the ROSEDUST design system.

## Documents

| # | File | Purpose | Use When |
|---|------|---------|----------|
| 01 | [ROSEDUST-DESIGN-SYSTEM.md](01-ROSEDUST-DESIGN-SYSTEM.md) | Complete design token reference — colors, typography, spacing, shadows, glass, motion, atmospheric layers, component patterns | **Always include.** This is the design system bible. |
| 02 | [THREEJS-WEBGL-PATTERNS.md](02-THREEJS-WEBGL-PATTERNS.md) | Three.js scene recipes — particle swarms, orrery, wireframe grids, canvas wrappers, HUD overlays, performance budget | Include when building pages with Three.js backgrounds or interactive 3D scenes |
| 03 | [ATMOSPHERE-AND-EFFECTS.md](03-ATMOSPHERE-AND-EFFECTS.md) | Advanced effects — Spectre avatars, crystallization animations, glitch overlays, phosphor decay, slot machines, progress rails, event-driven animation | Include when building richer interactive experiences with data-driven animation |
| 04 | [SITE-GENERATION-PROMPT.md](04-SITE-GENERATION-PROMPT.md) | **The mega-prompt.** Full instructions for generating a complete single-file HTML site. Includes page structure, quality checklist, and topic-specific addons | Use as the primary prompt for site generation tasks |
| 05 | [BARDO-DEEP-AESTHETICS.md](05-BARDO-DEEP-AESTHETICS.md) | Deep aesthetic source -- consciousness states, emotional modulation, lifecycle degradation, hauntological rendering, demoscene algorithms, philosophy engine. **Note:** "bardo" is legacy naming from before the project was renamed to roko. | Include for the most expressive/experiential sites. Not needed for standard product pages. |
| 06 | [SITE-CATALOGUE.md](06-SITE-CATALOGUE.md) | Catalogue of all 14 HTML iterations — look/feel per file, evolution timeline, visual paradigm shifts, interactive element lineage | Reference when choosing which era/style to draw from |
| 07 | [THREEJS-COMPONENT-LIBRARY.md](07-THREEJS-COMPONENT-LIBRARY.md) | **Modular component library.** 20+ reusable Three.js scenes, interactive controls, 2D canvas patterns, UI components — each with full code, parameters, and integration notes | **Always include for site generation.** This is the implementation cookbook. |

## Usage

**For a standard product/doc site:**
→ Use `04` as the prompt, with `01` + `02` + `07` as context.

**For a rich interactive showcase:**
→ Use `04` as the prompt, with `01` + `02` + `03` + `07` as context.

**For a deeply atmospheric/experiential piece:**
→ Use `04` + `01` + `02` + `03` + `05` + `07` as context.

**For choosing a visual era/style:**
→ Consult `06` (catalogue) to pick the paradigm (chassis vs glass/orbit, drag-orbit vs lerp, etc.), then use the appropriate component patterns from `07`.

## Sources Ingested

These documents were consolidated from:
- `tmp/solutions/demo-ui/` — 16 files (ROSEDUST spec, game UX design system, visual audits, redesign proposals, solutions)
- `tmp/solutions/demo-ui2/` — 13 files (refined ROSEDUST v2, WebGL backgrounds, Spectre system, crystallization, terminal aesthetic)
- `prd/18-interfaces/` — 28 files (rendering spec (originally "bardo" -- legacy name, now roko), creature system, spatial grammar, hauntology, transitions, demoscene, NERV aesthetic)

---

*Last updated: 2026-08-13*
- **14 HTML site iterations** (7 nunchi* + 7 rosedust*) — full Three.js code, interactive elements, CSS patterns, evolution across versions
