# E — Web Portal, Onboarding, Generative Interfaces / A2UI (Docs 13, 14, 15)

Parity of three user-layer chapters: web portal (518 lines), agent
onboarding flow (543 lines), generative interfaces / A2UI (570 lines).

All three chapters are **frontier**. No `apps/portal/` or equivalent
web frontend ships (only `apps/mirage-rs/` and `apps/roko-chain-
watcher/` exist, both chain-related). No A2UI / generative-interface
code. Onboarding flow is CLI-level (`roko init`) only.

Generated: 2026-04-16.

---

## E.01 — Web portal at port 3000 (Doc 13 §"Web Portal")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 13 (518 lines) describes a web portal at port 3000 with React/Next.js frontend consuming `roko-serve` HTTP + WS endpoints. WCAG 2.1 AA compliant (Doc 17).
**Reality**: `ls roko/` + `ls roko/apps/` — no web-portal directory. No `frontend/`, `portal/`, `web/`, `ui/`. The only shipping apps are `mirage-rs` (EVM simulator) and `roko-chain-watcher` (chain observer). The `roko-serve` HTTP control plane (B.01) ships, so the backend is ready for a frontend to consume — but no frontend exists in this repo.
**Fix sketch**: Doc 13 should carry `Design — Phase 2+` banner. The route surface (~85 routes, batch 12 B.01-B.13) is ready; the frontend implementation is the open work.

---

## E.02 — React + Rosedust + Recharts stack (Doc 13 §"Tech Stack")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 13 specifies React 18 + Next.js + Rosedust CSS + Recharts for charts + TanStack Query for data.
**Reality**: No JS/TS files in the repo that constitute a portal. Frontier.

---

## E.03 — Real-time Spectre / chart streaming (Doc 13 §"Real-Time Updates")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Portal consumes WS streams for real-time Spectre + chart updates.
**Reality**: Follows from D.04-D.06 (Spectre absent) + E.01 (portal absent).

---

## E.04 — Agent onboarding flow (Doc 14 §"Onboarding Flow")

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Doc 14 (543 lines) describes a 5-step onboarding flow: install → init → first prompt → first plan → first PRD. Progressive disclosure of capabilities.
**Reality**: The **CLI-level baseline** ships: `roko init` creates `.roko/` + `roko.toml` (A.08). `roko run "<prompt>"` runs the first prompt (A.02). `roko prd idea "<text>"` captures a work item (A.04). `roko prd plan <slug>` generates a plan (A.04). So the **functional flow** is live via CLI.

What does NOT ship: the documented "onboarding UI" layer — guided walkthroughs, progressive-help integration (cross-ref A.10), first-run wizard, onboarding telemetry, interactive tutorials.
**Fix sketch**: Doc 14 should split into "CLI-level onboarding (shipping)" + "Onboarding UI (frontier)". Current reality is functional bootstrap without onboarding-specific UX.

---

## E.05 — First-run wizard / interactive tutorial (Doc 14 §"First-Run Wizard")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 14 describes an interactive first-run wizard that guides new users through capabilities.
**Reality**: No wizard ships. `roko init` creates config; the rest is documentation-based onboarding.

---

## E.06 — Onboarding telemetry + retention metrics (Doc 14 §"Telemetry")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Onboarding flow tracks step completion + drop-off metrics.
**Reality**: No opt-in telemetry subsystem. Frontier. (This is also a privacy-sensitive area that deliberately does not ship.)

---

## E.07 — A2UI (Agent-to-UI) generative interfaces (Doc 15 §"A2UI")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 15 (570 lines) describes "A2UI" — agents generating their own UI components at runtime, declarative UI description schema (JSON → web / TUI / voice renderers), accessibility-first generation.
**Reality**: `Grep 'A2UI\|generative_interface\|ui_component_schema' crates/ --include=*.rs` returns zero matches. Pure frontier. The shipping TUI (C.*) uses hand-written widgets, not agent-generated UI.
**Fix sketch**: Doc 15 stays `Design — Phase 2+`.

---

## E.08 — Declarative UI schema (Doc 15 §"UI Schema")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 15 §"UI Schema" describes a declarative UI description format (JSON) that renders to web / TUI / voice.
**Reality**: No UI schema type. Frontier.

---

## E.09 — Multi-interface renderers from one schema (Doc 15 §"Multi-Interface")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: A single UI schema renders to web (React), TUI (ratatui), voice (TTS), portal (SVG) — accessibility-first.
**Reality**: Follows from E.07 / E.08. No shipping.

---

## E.10 — A2UI safety gating (Doc 15 §"Safety")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Agent-generated UI passes through safety gates (taint tracking from batch 11, prompt injection checks).
**Reality**: Safety layer ships (batch 11 A.01) but no A2UI consumer.

---

## E.11 — Web portal auth + bearer token (Doc 13 §"Auth")

**Status**: DONE (backend side)
**Severity**: —
**Doc claim**: Portal uses bearer tokens to authenticate against `roko-serve`.
**Reality**: The backend auth ships (batch 12 B.13 — bearer auth in `roko-agent-server/src/auth/bearer.rs` + roko-serve middleware). The portal-side auth is moot because there is no portal.

---

## Section Summary

| Status | Count |
|--------|-------|
| DONE | 1 (E.11 backend-side auth ships; portal absent) |
| PARTIAL | 1 (E.04 CLI-level onboarding shipping; UI wizard frontier) |
| NOT DONE | 9 (E.01 portal, E.02 stack, E.03 real-time, E.05 wizard, E.06 telemetry, E.07 A2UI, E.08 UI schema, E.09 multi-interface, E.10 A2UI safety) |

Section E is the **second-most-frontier section of topic 12**
(after Spectre). The web portal, onboarding UI, and A2UI are all
design; the underlying data + auth surfaces (HTTP control plane,
bearer auth) are shipping and portal-ready.

## Agent Execution Notes

### E.01-E.03 — Web portal banner pass

Doc 13 should carry `Design — Phase 2+` banner. Cross-link
`roko-serve` (B.01) as the ready-to-consume backend.

### E.04-E.06 — Onboarding split

Doc 14 should split into "CLI onboarding (shipping baseline)" vs
"Onboarding UI (frontier)". The CLI commands work — what's missing
is the tutorial layer.

### E.07-E.10 — A2UI frontier

Doc 15 stays frontier. May depend on LLM-to-code code generation
which is itself in-flight (agent-driven code production is the
core use case of Roko).

Acceptance criteria:

- Doc 13 carries Phase 2+ banner; cites roko-serve as backend ready,
- Doc 14 distinguishes CLI baseline from UI wizard,
- Doc 15 uniformly frontier.
