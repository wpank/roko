# Batch AUD04: Mark moat/plugin overscoping as aspirational (REF17-21)

**Audit refs**: 03-moat-audit.md (full file), 05-refinement-matrix.md (REF17-21 rows).
Applies the audit's "defer" and "skeptical" verdicts to `docs/20-technical-analysis/`
and `docs/18-tools/`.

Read these files first:

- `tmp/refinement-audit-runner/context-pack/00-AUDIT-RULES.md`
- `tmp/refinements-audit/03-moat-audit.md` (full file -- all 5 REFs audited)
- `tmp/refinements-audit/05-refinement-matrix.md` (REF17-21 rows)
- `tmp/refinements-audit/00-MASTER-SUMMARY.md` ("The 10 Things to Defer" section)
- `docs/18-tools/14-plugin-sdk.md`
- `docs/18-tools/16-plugin-loading.md`
- `docs/18-tools/INDEX.md`
- `docs/20-technical-analysis/00-vision-ta-generalized.md`
- `docs/20-technical-analysis/INDEX.md`
- `docs/00-architecture/17-design-principles-and-frontier-summary.md`
- `docs/00-architecture/30-cross-pollination-innovations.md`
- `docs/00-architecture/23-architectural-analysis-improvements.md`

## Task

The refinements-runner wrote a five-tier plugin SPI, WASM sandboxing, a plugin
registry, moat claims based on interaction density, and net-new innovation
catalogs into the tools and technical-analysis docs. The audit found: zero
plugin authors exist, the moat components are mostly aspirational (2 of 10
exist fully), and the innovation claims oversell speculative pieces. Mark
these as aspirational/deferred.

## Current state (evidence)

The audit found these specific issues:

1. **Plugin SPI (REF17)**: Tiers 1-3 (TOML manifests, prompt packs, declarative
   tools) are reasonable but unbuilt. Tiers 4-5 (C-FFI ABI bridge, WASM
   runtime with 7 host imports) are premature -- no third-party code execution
   need exists. The WASM host surface references types (`Pulse`, `Engram`
   graduation, `substrate_query_similar`) that do not exist in code.

2. **Plugin registry (`plugins.roko.dev`)**: Aspirational infrastructure for a
   community that does not exist. The doc acknowledges Phase 2+ but still
   specifies in detail.

3. **`roko-plugin` crate**: Already exists (~200 lines) as a narrow SDK for
   event sources and feedback loops. It is NOT the SPI described in the docs.

4. **Competitive moat (REF18)**: Of 10 claimed moat components, the audit
   found: HDC fully exists, c-factor partially exists, Bus exists in simpler
   form, demurrage does not exist (0 lines), replication ledger does not exist
   (0 lines), plugin ecosystem has zero participants, heuristic commons has
   zero deployments. The switching-cost table projects to day-720 based on
   features that are not built.

5. **Net-new innovations (REF19)**: The catalog format oversells speculative
   pieces. Audit verdict: **REWRITE** -- convert to research hypotheses.

6. **Modularity (REF20)**: The target dep graph adds `roko-bus`, `roko-hdc`,
   `roko-spi` -- none of which exist. The cleanup direction is right but the
   new crates are premature.

7. **From-scratch redesigns (REF21)**: Useful as a pressure test, dangerous
   as the default implementation mindset. Existing code works.

## Implementation

### 1. Mark plugin tiers 4-5 and registry as aspirational

In `docs/18-tools/14-plugin-sdk.md`:
- Add an implementation-status callout at the top:
  `> **Implementation status**: `roko-plugin` exists (~200 lines) as a narrow
  > SDK for event sources and feedback loops. Tiers 1-3 (prompt packs, profile
  > bundles, declarative tool manifests) are a reasonable near-term target.
  > Tiers 4-5 (C-FFI ABI, WASM sandboxed extensions) and the plugin registry
  > are **aspirational** -- zero plugin authors exist today.`
- Where WASM host imports are described, add a note that the referenced types
  (`Pulse`, `substrate_query_similar`) do not exist in code

In `docs/18-tools/16-plugin-loading.md`:
- Add a similar callout about the gap between current tool registration
  (Rust `ToolRegistry`) and the proposed manifest-driven discovery

### 2. Mark moat framing as aspirational

In `docs/20-technical-analysis/00-vision-ta-generalized.md`:
- If this doc makes moat claims based on the interaction density of 10
  primitives, add a callout:
  `> **Reality check**: Of the 10 primitives cited as moat components, 2 exist
  > fully (Engram, Substrate), 2 partially (HDC, c-factor), and 6 are
  > unimplemented (Pulse, Bus trait, Demurrage, Heuristic commons, Replication
  > ledger, Plugin SPI). The moat framing is aspirational.`

In `docs/00-architecture/30-cross-pollination-innovations.md`:
- If innovation claims cite unbuilt primitives, qualify them

In `docs/00-architecture/17-design-principles-and-frontier-summary.md`:
- If frontier claims cite unbuilt primitives, add appropriate qualifiers

### 3. Mark target crates as proposed in modularity docs

In `docs/00-architecture/23-architectural-analysis-improvements.md`:
- If it describes `roko-bus`, `roko-hdc`, `roko-spi` as existing, mark them as
  "proposed target crates"
- Note that the cleanup direction is correct but the new crates are not yet
  created

### 4. Qualify innovation catalog

In `docs/20-technical-analysis/INDEX.md` and relevant sub-docs:
- Where net-new innovation claims are made, add a qualifier distinguishing:
  - "Shipping" innovations (things that are actually built and novel)
  - "Research hypotheses" (interesting ideas not yet validated)
  - "Prior art integrations" (things that integrate existing research)

### 5. Acknowledge what actually exists as the real moat

Where moat language appears, add a grounding note:
`The actual competitive edge today is: a working Rust agent orchestrator with
multi-backend LLM dispatch, a 7-rung gate pipeline, HDC episode fingerprinting,
episode logging with feedback loops, and an interactive TUI. That is already
more than most agent frameworks have.`

## Write scope

- `docs/18-tools/14-plugin-sdk.md`
- `docs/18-tools/16-plugin-loading.md`
- `docs/18-tools/INDEX.md` (if it overstates plugin system status)
- `docs/20-technical-analysis/00-vision-ta-generalized.md`
- `docs/20-technical-analysis/INDEX.md`
- `docs/00-architecture/17-design-principles-and-frontier-summary.md`
- `docs/00-architecture/30-cross-pollination-innovations.md`
- `docs/00-architecture/23-architectural-analysis-improvements.md`

## Rules

1. **Mark, do not delete.** Aspirational designs are valuable as future specs.
   Add implementation-status callouts; do not remove design content.
2. **Be specific about what exists.** The `roko-plugin` crate is real but
   narrow. The 6 kernel traits are real extension surfaces. HDC is real. Name
   the real things.
3. **Do not claim nothing works.** The working product IS the moat. Qualify
   aspirational claims without denigrating what is built.
4. **Use "aspirational" not "wrong."** The moat/innovation framing is a vision
   doc, not a lie. Frame it as forward-looking, not as fiction.
5. **Do not touch learning docs** -- that is AUD03's scope.
6. **Do not touch safety docs** -- that is AUD06's scope.
7. **Do not touch the glossary** -- that is AUD06's scope.

## Done when

- Plugin SDK docs distinguish tiers 1-3 (reasonable near-term) from tiers 4-5
  (aspirational)
- WASM host surface is marked as referencing types that do not exist
- Moat claims are qualified with "X of 10 primitives currently exist"
- Innovation catalog distinguishes shipping vs. research hypotheses
- Target crates are marked as proposed, not existing
- The real working product is acknowledged as the actual competitive edge
- No design content was deleted
- Final message lists every doc edited and the key qualifier added
