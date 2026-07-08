# Prompt: 12-interfaces

You are a fresh Claude Opus agent. Zero prior context. Read every file this prompt references before writing.

## Your mission

Generate `/Users/will/dev/nunchi/roko/roko/docs/12-interfaces/`. Covers CLI, HTTP API (roko-serve), TUI design (29 screens, ROSEDUST palette, Spectre viewport), Web Portal, MCP servers, port allocation, agent onboarding, Generative Interfaces (A2UI), accessibility, and the **sonification reframe** (behavioral states, not mortality phases). **KEEP**: ROSEDUST, Spectre, 29-screen TUI. **REMOVE**: terminal requiem, death animations, vitality phase presets.

## Step 1 — Context pack (MANDATORY)

Read all 7 files in `/Users/will/dev/nunchi/roko/roko/tmp/prd-migration/context-pack/` in order.

## Step 2 — refactoring-prd canonical sources

1. `/Users/will/dev/nunchi/roko/refactoring-prd/06-interfaces.md` — **full interface spec** (CLI, HTTP API, 29 TUI screens, ROSEDUST palette, Spectre details, Web Portal, port allocation)
2. `/Users/will/dev/nunchi/roko/refactoring-prd/09-innovations.md` §XIV Generative Interfaces (A2UI)
3. `/Users/will/dev/nunchi/roko/refactoring-prd/08-translation-guide.md` §INCOMPATIBLE: Death Phases as UX, §NEEDS REDESIGN: Sonification
4. `/Users/will/dev/nunchi/roko/refactoring-prd/10-developer-guide.md` §1 Quick Start, §11 CLI UX (progressive help, error-as-teacher, interactive config)
5. `/Users/will/dev/nunchi/roko/refactoring-prd/07-implementation-priorities.md` §Tier 4 Interfaces

## Step 3 — SOURCE-INDEX entry `## 12-interfaces.md`

Read every file. Key legacy:
- All of `bardo-backup/prd/18-interfaces/` (Portal, CLI, UI system, TUI, spatial grammar, bardo-terminal foundation, creature system, plus perspective/, protocol/, rendering/, screens/ subdirectories)
- `bardo-backup/prd/25-mori/mori-interfaces.md`
- `bardo-backup/prd/15-dev/03-debug-ui.md`
- `bardo-backup/prd/shared/branding.md`
- `bardo-backup/prd/shared/port-allocation.md`
- `bardo-backup/prd/24-sonification/05-musical-language.md` — full music spec for the sonification sub-doc
- `bardo-backup/prd/24-sonification/06-preset-catalog.md` — 8 presets (to remap to behavioral states)
- `bardo-backup/tmp/mori-refactor/17-human-agent-interface.md`
- `bardo-backup/tmp/mori-refactor-plan/16-tui-and-support-cleanup.md`
- `bardo-backup/tmp/mori-agents/13-cli-and-deployment.md`

## Step 4 — implementation-plans

- `09-tui-dashboard.md`
- `11-agent-dogfooding.md` §Phase 0-1 (roko-serve extraction, HTTP API, WebSocket)
- `11-sections/phase-0-1.md` — roko-serve creation, webhook routes, dispatch loop

## Step 5 — active code

- Glob `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/**/*.rs`
- Read: `main.rs`, `run.rs`, `orchestrate.rs`, all subcommands
- Read `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/` (if scaffold exists)

## Step 6 — Output and sub-doc plan

```bash
mkdir -p /Users/will/dev/nunchi/roko/roko/docs/12-interfaces
```

Write **18 sub-docs** plus `INDEX.md`:

| # | Filename | Content |
|---|---|---|
| 00 | `00-cli-overview.md` | `roko` CLI as primary interface. 5 CLI modes (REPL, oneshot, pipe, daemon, serve). Top-level command groups. Zero-to-agent in 60 seconds. |
| 01 | `01-cli-command-reference.md` | Full command list with descriptions. roko init, run, status, config wizard, explain, new, plan, prd, research, neuro, episode, daemon, serve, mesh, provider, replay, inject, dashboard, repl. Per-command syntax. |
| 02 | `02-roko-new-scaffolders.md` | `roko new` subcommands: domain, gate, scorer, router, policy, substrate, probe, event-source, template. Every scaffold compiles immediately with working boilerplate. Example generated gate from 10-developer-guide.md §3. |
| 03 | `03-progressive-help-and-explain.md` | Progressive disclosure. `roko status`. `roko explain gates/routing/cognitive`. Error-as-teacher format (What happened / Why this matters / How to fix / Context). Interactive config wizard. |
| 04 | `04-configuration-layered-resolution.md` | roko.toml format. Layered resolution: CLI flags → env vars (ROKO_*) → roko.toml → defaults. Minimal config + override-only-what-you-need pattern. Auto-detection (language, build system, gates). |
| 05 | `05-http-api-roko-serve.md` | `roko-serve` HTTP server. Core endpoints (run, orchestrate, status, engrams, episodes). Agent management (GET/POST/DELETE /api/agents). Neuro endpoints. Provider endpoints. Mesh endpoints. |
| 06 | `06-websocket-streaming.md` | `/ws/events`, `/ws/agent/:id`, `/ws/cfactor`, `/ws/spectre/:id`. SSE for headless operation. Real-time event streaming. |
| 07 | `07-rosedust-design-language.md` | Full color palette (void-black #0a0a0f, twilight #12101a, dusk #1a1726, rose-dim/rose/rose-bright/rose-glow, jade/amber/crimson/violet/sapphire signals, ghost/mist/frost/white text). Typography (monospace throughout — JetBrains Mono, Berkeley Mono). Glass morphism (twilight 80% + blur 12px + rose-dim 20% border). Motion (luxury easing cubic-bezier(0.16, 1, 0.3, 1), ambient breathing, data transitions). Dark-only. |
| 08 | `08-tui-main-layout.md` | TUI main layout diagram (agent list, plan list, mesh, health panels + agent detail + gate results + Daimon state + Neuro tier counts + predictions + Spectre viewport). |
| 09 | `09-tui-29-screens.md` | Full screen inventory across 6 window regions: (1) Navigation (6 screens: agent list, plan list, mesh, knowledge browser, episode timeline, settings). (2) Agent Detail (6: output stream, gate results, Daimon state, prediction dashboard, tool trace, cost breakdown). (3) Plan Detail (5: DAG, task detail, merge queue, timeline, worktree status). (4) Knowledge (4: Neuro explorer, tier progression, cross-domain map, knowledge graph). (5) Collective (4: C-Factor dashboard, agent comparison, pheromone landscape, stigmergy map). (6) System (4: provider health, resource monitor, event log, spectre gallery). |
| 10 | `10-spectre-creature-visualization.md` | **Spectre is NOT decoration. It's a dense information display encoding multiple dimensions into organic visual form.** Procedurally generated from agent ID hash (first 64 bits = body shape/symmetry/limb count; next 32 bits = color; domain = texture: coding geometric / chain flowing / research fractal). Reflects Daimon PAD state (Engaged steady rose glow / Struggling rapid pulsing amber-crimson / Coasting relaxed sapphire / Exploring expanded violet tendrils / Focused compact jade / Resting minimal dim rose with Dreams active). **NEVER dies. Never has Terminal state.** Encodes behavioral state, knowledge tier distribution, current activity, health, mesh connections, pheromone emission. |
| 11 | `11-spectre-rendering-per-interface.md` | TUI (ASCII/Unicode + ANSI colors). Web Portal (WebGL 3D + react-three-fiber + full ROSEDUST glow). CLI (optional inline small spectre next to agent status). API (spectre state as JSON for custom renderers). |
| 12 | `12-spectre-as-collective-display.md` | When viewing a mesh/collective: Spectres arranged spatially by connection topology. Mesh connections render as glowing filaments between Spectres. Pheromone fields as ambient color clouds. C-Factor visualized as overall "harmony" of movement between Spectres — high C-Factor = synchronized, low = discordant. |
| 13 | `13-web-portal.md` | P2, not started. Tech: React 19 + Next 15.5+, Tailwind 4 (ROSEDUST theme), Radix UI, recharts, Three.js / react-three-fiber (Spectre WebGL), TanStack Query, WebSocket, Privy (wallet), viem (chain). 9 pages (Home / Agents / Agent Detail / Knowledge / Mesh / Plans / C-Factor Dashboard / Providers / Settings). |
| 14 | `14-agent-onboarding-flow.md` | Choose domain (coding / chain / research / custom). Select template or compose from Synapse traits. Configure model routing (cascade tiers). Set knowledge preferences (connect to mesh? post to Korai?). Name the agent → Spectre is generated → agent is live. |
| 15 | `15-generative-interfaces-a2ui.md` | Google A2UI protocol. Agents describe UI needs as structured JSONL → frameworks render automatically. No pre-designed screens needed for novel agent types. Generated interfaces inherit ROSEDUST. Spectre as persistent visual anchor. Paradigm for future: agents create their own UI. |
| 16 | `16-sonification-reframed.md` | **REFRAMED — keep the music, remap the presets.** Eno mandate preserved ("simultaneously ignorable and interesting"). 5 musical layers preserved. 8 presets **REMAPPED from mortality phases (Thriving → Terminal) to behavioral states (Engaged / Struggling / Coasting / Exploring / Focused / Resting)**. NO terminal requiem. NO death animations. NO degraded ambient music. Music theory and emotion-scale mappings stay valid. Preset catalog rewrite for each of the 6 behavioral states. |
| 17 | `17-accessibility-and-current-status.md` | WCAG 2.1 AA compliance. Keyboard navigable. Screen reader support (Spectre states described textually). Reduced motion mode (disables Spectre animations but preserves state display). Current status: CLI built (38 tests), HTTP API scaffold, TUI text-only, Web portal not started, MCP server not started, Spectre visualization not started. Port allocation: 3000 (web portal dev), 8080 (roko-serve HTTP), 8443 (WebSocket TLS), 8545 (mirage-rs Anvil RPC). |

Plus `INDEX.md`.

## Step 7-9 — Rules, INDEX, self-check

Per context-pack rules. ≥200 lines per sub-doc, ≥4000 total. Citations: Eno "Music for Airports" (1978), WCAG 2.1 AA, A2UI protocol, Karpathy context engineering.

Cross-reference topics 00-architecture, 02-agents (CLI spawns them), 09-daimon (Spectre reflects PAD state), 13-coordination (mesh visualization), 18-tools (MCP servers).

## CRITICAL REMINDERS

- DO NOT SUMMARIZE. DO NOT TRUNCATE.
- **KEEP**: ROSEDUST, glass morphism, Spectre creatures, 29-screen TUI, Portal concept.
- **REMOVE**: terminal requiem, death animations, vitality phases, mortality-mapped sonification presets.
- **Sonification**: keep the music theory, REMAP the preset catalog to behavioral states.
- Spectre **never dies**. Maps to Daimon behavioral states.
- Apply naming map: bardo-terminal → Roko TUI; Bardo Sanctum → Roko Portal; golem → agent; mori → Roko Orchestrator.
- Use Write tool. Don't ask questions.
