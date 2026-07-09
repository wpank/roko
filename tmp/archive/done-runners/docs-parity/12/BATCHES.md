# Batch Execution Contract

Six bounded batches for the topic-12 parity pack. This is a docs-calibration
pass, not a feature-build pass.

Generated: 2026-04-18

---

## Default Posture

- Prefer shipping-reality corrections over new design language.
- Use the audit-corrected headlines: CLI/TUI/HTTP are wired, `roko-serve` is
  200+ routes / 30K LOC, and the topic-level TUI surface is 58K LOC.
- Treat `roko new`, standalone `roko explain`, Spectre, SvelteKit web UI,
  A2UI, sonification, and IDE integration as non-shipping unless source proof
  says otherwise.
- Keep implementation asks small: REF28 CLI parity and REF26 StateHub
  hardening are ship-soon items, not excuses to redesign every interface doc.

## Recommended Order

`M1 -> M2 -> M3 -> M4 -> M5 -> M6`

## Batch Overview

| Batch | Purpose | Primary Write Scope | Verify Focus |
|---|---|---|---|
| M1 | CLI and config truth pass | `A-cli-and-config.md` | `roko new`, `roko explain`, `model route --explain`, config layering |
| M2 | HTTP, streaming, and port-drift truth pass | `B-http-and-websocket.md` | 200+ route framing, SSE/WS, sidecar `/message` + `/stream`, `9090` vs `6677` |
| M3 | TUI and Rosedust truth pass | `C-tui-and-rosedust.md` | 58K LOC headline, `F1`-`F7` tabs, ratatui wiring, PostFX, narrowed design language |
| M4 | Defer visualization, web UI, and A2UI | `D-spectre-creatures.md`, `E-web-onboarding-generative.md` | zero-code frontier surfaces vs shipping CLI onboarding |
| M5 | Narrow status, innovation, and IDE material | `F-access-innovation-ide.md` | shipping core vs deferred halo, `roko-mcp-code` truth note, ship-soon carry-forward |
| M6 | Refresh pack scaffolding and final consistency | `00-INDEX.md`, `SOURCE-INDEX.md`, `context-pack/*`, `run-docs-parity.sh` | corrected anchors, counts, runner descriptions, final grep checks |

## Dependencies

| Batch | Depends on |
|---|---|
| M1 | — |
| M2 | — |
| M3 | — |
| M4 | — |
| M5 | M1 M2 M3 M4 |
| M6 | M1 M2 M3 M4 M5 |

## Batch Details

### M1 — CLI and Config Truth

**Owns**: command-truth fixes for docs 00-04.

**Scope**:

1. State plainly that the CLI is already broad and wired.
2. Mark `roko new` as non-shipping.
3. Mark standalone `roko explain` as non-shipping and point to
   `model route --explain` as the live nearby surface.
4. Keep config layering grounded in the existing config commands.
5. Carry REF28 forward as ship-soon work, not present-tense product parity.

**Acceptance criteria**:

- `A-cli-and-config.md` no longer treats `roko new` or `roko explain` as
  partial live features.
- The file distinguishes `prd draft new` from a hypothetical `roko new`.
- CLI parity / muscle memory is framed as next-step work, not already done.

### M2 — HTTP, Streaming, and Port Drift

**Owns**: control-plane and sidecar truth pass.

**Scope**:

1. Replace stale "~85 routes" framing with the audit-corrected
   200+ route / 30K LOC headline.
2. Keep SSE, WebSocket, and sidecar messaging described as shipping.
3. Flag the `9090` vs `6677` split as unresolved docs/runtime drift.
4. Keep OpenAPI, gRPC, and browser UI work deferred.

**Acceptance criteria**:

- `B-http-and-websocket.md` describes `roko-serve` as a shipping surface.
- The port split is explicit and concrete.
- `/api/events`, `/api/routing/explain`, `/message`, and `/stream` are called
  out with source anchors.

### M3 — TUI and Rosedust

**Owns**: TUI status correction and design-language narrowing.

**Scope**:

1. Use the audit-corrected 58K LOC headline for the TUI/CLI surface.
2. Reframe the shipping TUI around `F1`-`F7` tabs, views, modals, and effects.
3. Keep ratatui, PostFX, and the palette/theme layer as shipping.
4. Treat the large 29-screen inventory and full cross-surface design system as
   overscoped docs, not absent product gaps.

**Acceptance criteria**:

- `C-tui-and-rosedust.md` describes a wired TUI instead of a mostly-missing one.
- Rosedust is narrowed to the real theme/palette layer.
- Unverified extras like command palette/global search stay out of the
  shipping headline.

### M4 — Deferred Visualization and Web Halo

**Owns**: explicit deferrals for the zero-code surface area.

**Scope**:

1. Mark Spectre / creature visualization deferred.
2. Mark SvelteKit / first-party web UI deferred.
3. Mark A2UI deferred.
4. Split shipping CLI onboarding from deferred onboarding UI.

**Acceptance criteria**:

- `D-spectre-creatures.md` is uniformly deferred.
- `E-web-onboarding-generative.md` separates CLI bootstrap from absent web UI
  and A2UI runtime work.

### M5 — Status, Innovation, and IDE

**Owns**: status framing and the remaining innovation halo.

**Scope**:

1. Correct the status narrative so shipping CLI/TUI/HTTP/sidecar are obvious.
2. Scope accessibility claims to shipping surfaces.
3. Defer sonification, rich UX primitives, ACP, and VS Code integration.
4. Credit `roko-mcp-code` without overstating it as full IDE support.
5. Carry REF26 and REF28 forward as the near-term interface work that remains.

**Acceptance criteria**:

- `F-access-innovation-ide.md` distinguishes shipping core, ship-soon hardening,
  and deferred innovation.
- The file does not imply IDE integration or sonification ship today.

### M6 — Pack Scaffolding and Final Sweep

**Owns**: the pack-level materials.

**Scope**:

1. Refresh `00-INDEX.md` around shipping reality.
2. Refresh `SOURCE-INDEX.md` with corrected anchors and line numbers.
3. Refresh the context pack with the post-audit gap picture.
4. Update `run-docs-parity.sh` to the narrowed batch descriptions.
5. Run the grep and shell-syntax checks.

**Acceptance criteria**:

- the context pack matches the six-batch plan,
- source anchors point at current files and line ranges,
- `bash -n tmp/docs-parity/12/run-docs-parity.sh` passes.
