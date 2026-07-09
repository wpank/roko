# 12-Interfaces Parity Analysis

Gap analysis of `docs/12-interfaces/` (20 PRDs + INDEX, ~12,715 lines
covering CLI, config, `roko-serve`, WebSocket/SSE streaming, Rosedust,
TUI layout, 29-screen inventory, Spectre visualization, web portal,
onboarding, A2UI, sonification, accessibility/status, UX innovation,
and IDE integration) against the shipping interfaces stack:

- `crates/roko-cli/` — substantial CLI binary + TUI
- `crates/roko-serve/` — HTTP control plane with 17 route modules plus
  route wiring
- `crates/roko-agent-server/` — per-agent sidecar with auth,
  registration, HTTP messaging, and WebSocket streaming
- `crates/roko-mcp-code/` — shipping MCP code-intelligence server

Generated: 2026-04-16

---

## How To Use This Batch

**Topic 12 is a shipping-core plus large-frontier-halo batch.**

The shipped core is real and large:

- CLI command surface in `crates/roko-cli/src/main.rs`
- TUI in `crates/roko-cli/src/tui/`
- HTTP control plane in `crates/roko-serve/src/routes/`
- per-agent sidecar in `crates/roko-agent-server/src/`
- MCP code server in `crates/roko-mcp-code/`

The frontier halo is also real, but mostly design-only:

- Spectre visualization
- web portal frontend
- A2UI / generative interfaces
- sonification
- most UX innovation proposals
- ACP / VS Code extension work

The most important work in this batch is not inventing interface code.
It is making the docs honest and making the parity pack safe for
overnight agents:

1. Correct the CLI truth surface:
   `roko new` does not appear to ship, and Doc 03's `roko explain`
   concept does not map to a standalone CLI subcommand.
2. Correct the server truth surface:
   `roko-serve` and `roko-agent-server` are more shipped than the docs
   admit, but the port/default story is inconsistent (`9090` in CLI,
   `6677` in README/chat defaults).
3. Reframe the TUI docs around shipping tabs/modals/widgets instead of
   the speculative 29-screen flat inventory.
4. Regenerate Doc 17 from `Scaffold` to a mixed status document.
5. Apply stronger frontier banners to Spectre, web portal, A2UI,
   sonification, UX innovation, and IDE-extension work.

If a task starts requiring a new portal, Spectre renderer, audio stack,
A2UI schema, ACP runtime, or VS Code extension, stop and record the
seam.

Recommended serial order: `M1 -> M2 -> M3 -> M4 -> M5 -> M6 -> M7 -> M8`

---

## Document Index

| File | Docs Covered | Items | Status |
|------|--------------|-------|--------|
| [A-cli-and-config.md](A-cli-and-config.md) | 00, 01, 02, 03, 04 | A.01-A.13 | CLI core mostly shipping; scaffolders/explain need truth pass |
| [B-http-and-websocket.md](B-http-and-websocket.md) | 05, 06 | B.01-B.15 | Server + sidecar strong, but port and endpoint-contract notes need tightening |
| [C-tui-and-rosedust.md](C-tui-and-rosedust.md) | 07, 08, 09 | C.01-C.13 | TUI ships strongly; 29-screen and design-language docs drift |
| [D-spectre-creatures.md](D-spectre-creatures.md) | 10, 11, 12 | D.01-D.09 | Frontier-only |
| [E-web-onboarding-generative.md](E-web-onboarding-generative.md) | 13, 14, 15 | E.01-E.11 | Backend-ready + frontend frontier |
| [F-access-innovation-ide.md](F-access-innovation-ide.md) | 16, 17, 18, 20 | F.01-F.14 | Status doc plus mostly frontier innovation surfaces |
| [BATCHES.md](BATCHES.md) | — | 8 batches | Execution contract |
| [SOURCE-INDEX.md](SOURCE-INDEX.md) | — | Verified code anchors + corrections | Reference |
| [run-docs-parity.sh](run-docs-parity.sh) | — | Batch runner | Overnight execution scaffold |
| `context-pack/agent-runbook.md` | — | Execution posture | Agent brief |
| `context-pack/carry-forward-map.md` | — | Deferral map | Scope control |
| `context-pack/interfaces-summary.md` | — | Shipping vs frontier summary | Quick context |
| `context-pack/gaps-summary.md` | — | Main status hotspots | Quick context |
| `context-pack/repo-map.md` | — | High-value paths + searches | Fast verification |

Doc `INDEX.md` is absorbed into this file.

---

## Overall Parity: 35/75 items DONE (47%)

Topic 12 still has one of the highest raw parity scores, but the current
pack overstates a few verified surfaces and understates a few others.

What is clearly shipping:

- CLI with substantial subcommand surface
- TUI with 7 tabs, modal stack, widget library, and PostFX pipeline
- `roko-serve` API/control plane
- `roko-agent-server` sidecar with `/message` and `/stream`
- MCP server path via `roko-mcp-code`

What is clearly frontier:

- Spectre rendering
- web portal frontend
- A2UI
- sonification
- voice / gesture / multimodal ideas
- ACP runtime and VS Code extension work

What needs special handling because it is neither simply shipped nor
simply absent:

- Doc 17 status framing (`Scaffold` is too weak)
- `roko new` and `roko explain` doc claims
- `9090` vs `6677` serve-port drift across code, docs, and READMEs
- specific route/endpoint inventories that are richer in the docs than
  what we have directly verified from source

### Tier 1 — Should Exist Now (runtime-critical)

None. The core interface layer already runs.

### Tier 2 — Should Exist Soon (doc honesty / status clarity)

| ID | Title | Status | Severity |
|----|-------|--------|----------|
| F.04 | Doc 17 still marks the whole topic `Scaffold` | PARTIAL | HIGH |
| B.01 / B.14 | Serve-port story is inconsistent: CLI defaults `9090`, READMEs/chat default `6677` | PARTIAL | HIGH |
| A.09 | `roko new` appears absent from the live CLI command enum | NOT DONE / unverified in prior pack | MEDIUM |
| A.10 | Doc 03 `roko explain` does not map to a standalone subcommand; only model-route explain flags are verified | NOT DONE / doc drift | MEDIUM |
| C.03 | Doc 09's 29-screen framing does not match shipping 7-tab + modal-stack reality | PARTIAL | MEDIUM |
| C.07 | Doc 07 overstates a full design language; shipping Rosedust is much narrower | PARTIAL | LOW |
| B.02-B.07 | Some specific endpoint claims should be treated as partially verified unless source-path confirmed | PARTIAL | LOW |

### Tier 3 — Future / Phase 2+ Frontier

| ID | Title | Status | Severity |
|----|-------|--------|----------|
| D.01-D.09 | Spectre visualization | NOT DONE | LOW |
| E.01-E.10 | Portal, onboarding UI, A2UI | NOT DONE / PARTIAL | LOW |
| F.01-F.02 | Sonification | NOT DONE | LOW |
| F.05-F.09 | UX innovation proposals | NOT DONE | LOW |
| F.10-F.11-F.13-F.14 | ACP / VS Code extension surfaces | NOT DONE | LOW |
| B.15 | OpenAPI / auto-generated API docs | NOT DONE | LOW |
| C.13 | Command palette / global search / per-view filter | NOT DONE | LOW |

### Already Shipped

| ID | Title | Status |
|----|-------|--------|
| A.01-A.08, A.11-A.13 | Core CLI + config + status/replay/dashboard/serve/chat + agent/daemon families | DONE |
| B.01 | `roko-serve` route stack exists and is substantial | DONE |
| B.04-B.07, B.11-B.13 | agent sidecar auth + registration + `/message` + `/stream` + feature routes + SSE/WS surfaces | DONE |
| C.01-C.02, C.04-C.06, C.08-C.11 | TUI tabs, modals, widgets, palette, PostFX, approval IPC, config tab | DONE |
| E.11 | backend-side bearer-auth foundation for future portal | DONE |
| F.12 | MCP-server coverage via `roko-mcp-code` | DONE |

---

## Execution Boundaries

| Item | Better Home | Why |
|------|-------------|-----|
| Spectre renderer or creature-state schema | later visualization pass | no renderer ships today |
| React/Next portal implementation | later frontend build pass | backend exists; frontend absent |
| Sonification subsystem | later audio pass | no audio stack |
| A2UI runtime/schema/renderers | later generative-UI pass | design-only today |
| ACP runtime / VS Code extension | later IDE-integration pass | decision doc exists; runtime absent |
| Large UX proposal prioritization beyond doc splitting | later product-review pass | proposal curation, not parity |

Batch 12 should produce:

- a batch pack that explicitly calls out `roko new` / `roko explain`
  truth status,
- a batch pack that calls out `9090` vs `6677` as a real drift seam,
- a stronger Doc 17 regeneration plan,
- narrower frontier passes for Spectre / portal / A2UI / sonification /
  IDE / UX proposals,
- a runner + context pack so agents can execute batch slices overnight.

---

## Critical Interface Issues

1. **Doc 17 still says `Scaffold`.** That is materially misleading for a topic with a substantial CLI, TUI, server, and sidecar.
2. **The serve-port story is inconsistent.** `roko serve` defaults to `9090` in `main.rs`, while `roko-serve/README.md` and chat defaults still point at `6677`.
3. **`roko new` and `roko explain` are not just “partially verified”.** They need explicit truth-in-advertising treatment unless source proof appears.
4. **Docs 05 and 06 are more scaffold-y than the parity pack admitted.** The framework and many routes are real, but several detailed endpoint claims need narrower verification language.
5. **Doc 09's 29-screen flat inventory is not how the shipping TUI is organized.**
6. **The frontier halo is very large.** Spectre, portal, A2UI, sonification, UX innovation, and full IDE integration need harder boundaries so agents do not widen the batch.

---

## Key Insight

Topic 12 is not a weak interfaces topic. It is a **strong interfaces
topic with weak status framing**.

The most important parity work is:

- correcting status language,
- narrowing or downgrading unverified command/endpoint claims,
- separating backend-shipping from frontend-frontier,
- and making the docs safe for unattended agents to execute without
  assuming that every spec surface already exists.

---

## Batch 12 Success Definition

Batch `12` is successful when:

- the parity pack includes runner + context-pack scaffolding,
- `BATCHES.md` splits the work around real status hotspots instead of
  broad “frontier pass” buckets,
- `SOURCE-INDEX.md` explicitly calls out `roko new`, standalone
  `roko explain`, and the `9090` vs `6677` drift,
- section notes are explicit enough that an overnight agent can execute
  one batch without prior context,
- later agents can pick up `BATCHES.md` and execute `M1` safely.
