# Batch Execution Contract

8 batches ordered for unattended execution. Topic 12 is a
**shipping-core with frontier halo** story, but the biggest immediate
work is status correction and scope control, not new interface code.

The main hotspots are:

- Doc 17 still says `Scaffold`
- `roko new` and standalone `roko explain` do not appear to ship
- serve-port defaults are inconsistent (`9090` vs `6677`)
- TUI docs describe a richer “29 screens” surface than the shipping
  tab/modal organization

---

## Batch Posture

- Default strategy: **make the shipping CLI/TUI/server/sidecar obvious; frontier-tag the visualization and innovation halo**.
- Treat `docs/12-interfaces/17-accessibility-and-current-status.md` as the primary status hotspot.
- Treat `docs/12-interfaces/01-cli-command-reference.md`, `02-roko-new-scaffolders.md`, and `03-progressive-help-and-explain.md` as the primary truth-in-advertising hotspot.
- Treat the `9090` vs `6677` split as a first-class doc seam, not a footnote.
- If a task starts requiring a new portal, renderer, ACP runtime, sonification system, or A2UI runtime, record the seam and stop.

## Required Reads

- `tmp/docs-parity/12/00-INDEX.md`
- `tmp/docs-parity/12/BATCHES.md`
- `tmp/docs-parity/12/SOURCE-INDEX.md`
- `tmp/docs-parity/12/context-pack/agent-runbook.md`
- `tmp/docs-parity/12/context-pack/carry-forward-map.md`
- `tmp/docs-parity/12/context-pack/interfaces-summary.md`
- `tmp/docs-parity/12/context-pack/gaps-summary.md`
- `tmp/docs-parity/12/context-pack/repo-map.md`

---

## Recommended Serial Order

`M1 -> M2 -> M3 -> M4 -> M5 -> M6 -> M7 -> M8`

- M1 resolves the CLI truth surface.
- M2 resolves the server/sidecar truth surface and port split.
- M3 resolves the TUI/Rosedust truth surface.
- M4 regenerates the topic status doc from those truths.
- M5-M7 frontier-tag the remaining major surfaces.
- M8 does the final topic sweep.

---

## Batch Overview

| Batch | Tasks | Purpose | Primary Write Scope | Verify Focus |
|-------|-------|---------|---------------------|--------------|
| M1 | A.09, A.10, A.12, A.13, A.11 | CLI truth pass: verify absent scaffolders/explain, enumerate shipping agent/daemon/model-route surfaces | Docs 01, 02, 03, 04 | `rg -n "roko new|roko explain|model route|agent_|daemon" docs/12-interfaces/01-*.md docs/12-interfaces/02-*.md docs/12-interfaces/03-*.md crates/roko-cli/src/main.rs` |
| M2 | B.01-B.15 | Server + sidecar truth pass: route stack, feature routes, and `9090` vs `6677` drift | Docs 05, 06, 17 | `rg -n "9090|6677|/api/events|/stream|/message|Implementation: Scaffold|Route Groups" docs/12-interfaces/05-*.md docs/12-interfaces/06-*.md docs/12-interfaces/17-*.md crates/roko-cli/src/main.rs crates/roko-serve/README.md crates/roko-agent-server/src` |
| M3 | C.03, C.07, C.08, C.12, C.13 | TUI reality pass: 7 tabs, modal stack, PostFX, palette-only Rosedust, inspect-depth honesty | Docs 07, 08, 09 | `rg -n "29 screens|F1|F7|PostFX|Rosedust|command palette|global search" docs/12-interfaces/07-*.md docs/12-interfaces/08-*.md docs/12-interfaces/09-*.md crates/roko-cli/src/tui` |
| M4 | F.03, F.04 | Regenerate Doc 17 as the canonical mixed-status doc, including port/status truth | Doc 17 | `rg -n "Implementation|Shipping|Partial|Frontier|9090|6677|TUI|roko-serve|agent-server" docs/12-interfaces/17-*.md` |
| M5 | D.01-D.09 | Uniform Spectre frontier pass | Docs 10, 11, 12 | `rg -n "Design — Phase 2\\+|Tier 2M|Spectre|status_bar|token_sparkline" docs/12-interfaces/10-*.md docs/12-interfaces/11-*.md docs/12-interfaces/12-*.md` |
| M6 | E.01-E.11 | Web/onboarding/A2UI split: backend-ready vs frontend frontier | Docs 13, 14, 15 | `rg -n "Design — Phase 2\\+|roko-serve|CLI onboarding|A2UI|frontend|portal" docs/12-interfaces/13-*.md docs/12-interfaces/14-*.md docs/12-interfaces/15-*.md` |
| M7 | F.01-F.02, F.05-F.14 | Sonification / UX innovation / IDE frontier pass with `roko-mcp-code` truth note | Docs 16, 18, 20 | `rg -n "Design — Phase 2\\+|Proposed|roko-mcp-code|ACP|VS Code|sonification|voice|gesture" docs/12-interfaces/16-*.md docs/12-interfaces/18-*.md docs/12-interfaces/20-*.md` |
| M8 | global banners + INDEX | Final banner sweep + INDEX parity pointer + consistency pass | All docs/12-interfaces/*.md + tmp/docs-parity/12/* | `rg -n "^> \\*\\*Implementation\\*\\*:|^> \\*\\*Status\\*\\*:" docs/12-interfaces/*.md` |

---

## Dependency Graph

| Batch | Depends on |
|-------|------------|
| M1 | — |
| M2 | — |
| M3 | — |
| M4 | M1 M2 M3 |
| M5 | M4 |
| M6 | M4 |
| M7 | M4 |
| M8 | M1 M2 M3 M4 M5 M6 M7 |

M1-M3 can run independently, but M4 should consume their settled truth
before rewriting the topic status doc.

---

## Parallel-Safe Groups

| Group | Batches | Notes |
|-------|---------|-------|
| G1 | M1, M2, M3 | distinct doc slices; safe to run in parallel |
| G2 | M5, M6, M7 | all frontier passes, after M4 if you want the status doc settled first |

## Conflict Groups

| Group | Shared Write Scope | Why |
|-------|--------------------|-----|
| C1 | `docs/12-interfaces/17-accessibility-and-current-status.md` | M2 and M4 both touch port/status framing |
| C2 | `tmp/docs-parity/12/00-INDEX.md`, `BATCHES.md`, `SOURCE-INDEX.md` | top-level pack should be owned by one agent |

---

## Batch Details

### M1 — CLI Truth Pass

**Owns**: A.09, A.10, A.12, A.13, A.11

**Problem**: The current parity pack still treats `roko new` and
`roko explain` as “partial/unverified”. Source review of
`crates/roko-cli/src/main.rs` shows a large real command surface, but
not those standalone subcommands.

**Scope**:

1. Doc 01 — enumerate the shipping command families actually present in `main.rs`.
2. Doc 01 — explicitly add agent/daemon/provider/model/subscription/event-source surfaces if missing.
3. Doc 02 — if `roko new` is absent, mark it `Design — Phase 2+` instead of implying it ships.
4. Doc 03 — if no standalone `roko explain` exists, reframe as future/proposed help UX.
5. Doc 04 — keep layered config grounded in the shipping config CLI and schema.

**Out of scope**: adding new CLI commands.

**Acceptance criteria**:

- Doc 01 reflects the actual `main.rs` command tree,
- Doc 02 no longer implies `roko new` ships if it does not,
- Doc 03 no longer implies a standalone `roko explain` command if it does not,
- any `--explain` coverage is described precisely (for example model-route explain).

### M2 — Server / Sidecar Truth Pass

**Owns**: B.01-B.15

**Problem**: The current parity pack is directionally right that the
server layer ships strongly, but it papers over two important issues:
port drift (`9090` vs `6677`) and some endpoint-level claims that were
more detailed than the direct source proof.

**Scope**:

1. Doc 05 — cite the shipping route modules and route-building surface.
2. Doc 06 — cite the shipping SSE, top-level WS, and sidecar `/stream` path.
3. Reconcile or clearly flag the `9090` vs `6677` split across CLI, READMEs, and chat defaults.
4. Downgrade endpoint details from `DONE` to `PARTIAL` where only route-module existence, not exact behavior, was verified.
5. Keep OpenAPI docs as frontier.

**Out of scope**: changing runtime defaults or editing Rust.

**Acceptance criteria**:

- batch notes explicitly call out the port split,
- Docs 05/06 distinguish “route stack ships” from “every described endpoint behavior verified”,
- sidecar `/message` and `/stream` surfaces are cited from source.

### M3 — TUI / Rosedust Reality Pass

**Owns**: C.03, C.07, C.08, C.12, C.13

**Problem**: The shipping TUI is real and deep, but Doc 09 frames it as
29 flat screens and Doc 07 frames Rosedust as a full design language.

**Scope**:

1. Reclassify Doc 09’s screens into tabs, modals, widgets, or frontier.
2. Make PostFX / atmosphere / effects explicit.
3. Scope Rosedust to the shipping theme/palette layer.
4. Keep F7 inspect depth honest if full DAG/episode UI is not verified.
5. Mark command palette/global search as frontier unless source proof appears.

**Acceptance criteria**:

- Doc 09 no longer reads like the shipping TUI has 29 independent views,
- Doc 07 no longer overclaims a full shipping design-language implementation,
- PostFX is surfaced as real runtime scope.

### M4 — Regenerate Doc 17

**Owns**: F.03, F.04

**Problem**: Doc 17 is the canonical status doc but still says
`Implementation: Scaffold`.

**Scope**:

1. Rewrite the top banner to a mixed status.
2. Split status into shipping core, partial UX, and frontier innovation surfaces.
3. Include the port/default drift explicitly until it is resolved elsewhere.
4. Scope accessibility claims to what is meaningful for shipping surfaces now.

**Acceptance criteria**:

- Doc 17 no longer uses a blanket `Scaffold` banner,
- shipping CLI/TUI/server/sidecar are called out distinctly,
- frontier surfaces are clearly separated,
- port/default drift is not silently ignored.

### M5 — Spectre Frontier Pass

**Owns**: D.01-D.09

**Problem**: Spectre docs are rich but unshipped.

**Scope**:

1. Apply uniform frontier banners to Docs 10-12.
2. Cross-link the nearest shipping text-only surfaces where useful.
3. Make dependency on collective/mesh work explicit.

**Acceptance criteria**:

- Docs 10-12 carry strong consistent frontier framing,
- no agent could mistake Spectre for a shipped subsystem.

### M6 — Web / Onboarding / A2UI Split

**Owns**: E.01-E.11

**Problem**: Docs 13-15 blur backend-readiness with missing frontend/UI runtime.

**Scope**:

1. Doc 13 — backend-ready, frontend absent.
2. Doc 14 — CLI bootstrap shipping, onboarding UI frontier.
3. Doc 15 — keep A2UI entirely frontier.

**Acceptance criteria**:

- backend/frontend ownership is explicit,
- CLI onboarding baseline is separated from onboarding UI,
- A2UI stays clearly future.

### M7 — Sonification / UX / IDE Frontier Pass

**Owns**: F.01-F.02, F.05-F.14

**Problem**: Topic 12’s long-tail innovation docs are mostly proposal
content, but Doc 20 should still credit `roko-mcp-code`.

**Scope**:

1. Keep sonification future.
2. Split or prioritize Doc 18 proposal buckets where useful.
3. Keep ACP and VS Code runtime absent.
4. Cite `roko-mcp-code` as shipping coverage of the MCP path.

**Acceptance criteria**:

- Docs 16/18/20 use strong truth-in-advertising language,
- Doc 20 cites shipping MCP coverage without implying full IDE support.

### M8 — Final Banner + Housekeeping

**Owns**: final topic-12 cleanup

**Scope**:

1. Sweep implementation/status banners.
2. Add pointer from `docs/12-interfaces/INDEX.md` to `tmp/docs-parity/12/00-INDEX.md`.
3. Ensure parity pack and section notes align.

**Acceptance criteria**:

- banners are internally consistent,
- parity audit is discoverable,
- no remaining top-level contradiction across the batch pack.
