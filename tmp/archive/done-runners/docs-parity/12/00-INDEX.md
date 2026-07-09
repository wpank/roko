# 12-Interfaces Parity Refresh

Audit-aligned refresh for `docs/12-interfaces/`. The point of this pack is
to describe the shipping interface core honestly, separate it from planned UX
work, and make the remaining gaps small and concrete.

Generated: 2026-04-18

---

## Post-Audit Posture

- Shipping now: the CLI, ratatui TUI, `roko-serve` HTTP surface, SSE/WS
  streaming, and the per-agent sidecar are all live and materially larger than
  the old parity pack implied.
- Ship soon: REF28 CLI parity / muscle memory, REF26 StateHub hardening, and a
  concrete cleanup of the `9090` vs `6677` default-port split.
- Defer: Spectre, SvelteKit web UI, A2UI, sonification, IDE integration,
  rich-UX primitives, and other zero-code interface concepts.

## Headline Corrections

- Treat `roko-serve` as a shipping control plane: 200+ routes and roughly
  30K LOC per the audit-corrected repo facts, not "~85 routes" or "not wired."
- Treat the TUI as shipping: the topic-level TUI/CLI surface is 58K LOC, with
  `F1`-`F7` tabs and ratatui wiring already in place.
- Treat the CLI as fully wired. The two important non-shipping concepts are
  still `roko new` and standalone `roko explain`.
- Flag the port inconsistency directly:
  `roko serve` and daemon defaults are `9090` in code, while chat defaults and
  several READMEs still point at `6677`.
- Treat first-party web UI, A2UI, sonification, Spectre, and full IDE
  integration as explicit future work, not partial current product.

## Gap Picture

### Concrete Near-Term Gaps

- Docs 00-04 need truth-in-advertising language for `roko new` and
  `roko explain`.
- Docs 05, 06, and 17 need the port split called out as unresolved drift.
- Docs 07-09 need to describe the shipping TUI as tabs, views, modals, and
  effects rather than as a flat speculative screen catalog.
- Docs 17, 18, and 20 need to stop blending the shipping core with proposal
  content.

### Deferred, Not Missing

- Spectre and creature rendering
- SvelteKit / first-party browser UI
- A2UI and other generative-interface runtime work
- Sonification
- ACP / VS Code integration
- Rich UX primitives and other speculative multimodal surfaces

## File Guide

| File | Purpose | Audit Stance |
|---|---|---|
| [A-cli-and-config.md](A-cli-and-config.md) | CLI, config, command-truth refresh | keep core, narrow command claims |
| [B-http-and-websocket.md](B-http-and-websocket.md) | `roko-serve`, SSE, WS, sidecar | keep core, flag port drift |
| [C-tui-and-rosedust.md](C-tui-and-rosedust.md) | TUI, tabs, Rosedust, PostFX | keep core, narrow design-language claims |
| [D-spectre-creatures.md](D-spectre-creatures.md) | Spectre / creature docs | defer |
| [E-web-onboarding-generative.md](E-web-onboarding-generative.md) | web UI, onboarding UI, A2UI | split shipping CLI onboarding from deferred UI |
| [F-access-innovation-ide.md](F-access-innovation-ide.md) | status doc, sonification, UX ideas, IDE | narrow status, defer innovation halo |
| [SOURCE-INDEX.md](SOURCE-INDEX.md) | verified anchors and corrected references | refresh line numbers and counts |
| [BATCHES.md](BATCHES.md) | realistic overnight-batch plan | narrow to six bounded batches |
| [context-pack/interfaces-summary.md](context-pack/interfaces-summary.md) | one-page summary | shipping core vs deferred halo |
| [context-pack/gaps-summary.md](context-pack/gaps-summary.md) | main hot spots | post-audit gap view |
| [context-pack/carry-forward-map.md](context-pack/carry-forward-map.md) | defer map | keep implementation scope small |
| [context-pack/repo-map.md](context-pack/repo-map.md) | code anchors | corrected numbers and searches |
| [context-pack/agent-runbook.md](context-pack/agent-runbook.md) | execution posture | docs-calibration, not product design |
| [run-docs-parity.sh](run-docs-parity.sh) | overnight runner | descriptions updated to narrowed scope |

## Success Definition

This batch is successful when:

- the parity pack presents CLI, TUI, HTTP, and sidecar as shipping interfaces,
- `roko new` and standalone `roko explain` are explicitly called non-shipping,
- the `9090` vs `6677` split is documented as a concrete fix item,
- Spectre, SvelteKit UI, A2UI, sonification, IDE integration, and rich UX
  primitives are clearly marked deferred,
- the batch plan is realistic for one agent working inside docs only.
