# Demo Redesign — Master Index

Plan for a unified demo page that showcases roko's capabilities with the
ROSEDUST visual identity. Replaces the current three separate HTML files
(`index.html`, `terminal.html`, `builder.html`) with a single cohesive
experience.

## Documents

| Doc | What |
|---|---|
| [01-ARCHITECTURE.md](01-ARCHITECTURE.md) | Page structure, component tree, data flow |
| [02-VISUAL-SYSTEM.md](02-VISUAL-SYSTEM.md) | ROSEDUST CSS tokens, glass panels, typography |
| [03-DEMO-SCENARIOS.md](03-DEMO-SCENARIOS.md) | All demo scenarios with exact commands |
| [04-CHECKLIST.md](04-CHECKLIST.md) | Implementation checklist with status tracking |
| [05-CHAIN-KNOWLEDGE-DEMO.md](05-CHAIN-KNOWLEDGE-DEMO.md) | Two new demo scenarios: Code Knowledge Transfer (no chain) + DeFi Chain Intelligence (forked mainnet). Full implementation specs, UI layouts, agent prompts, wiring gaps. |
| [06-JOB-MARKET-DEMO.md](06-JOB-MARKET-DEMO.md) | Trustless job market demo: agents post/claim/execute bounties via BountyMarket (ERC-8183) on mirage. Two examples (research report + API build), validator voting, reputation progression. |

## Design Sources

| Source | Path | What |
|---|---|---|
| ROSEDUST design system | `bardo/prd/18-interfaces/rendering/00-design-system.md` | Color palette, 7 rendering laws |
| Widget catalog | `bardo/prd/18-interfaces/screens/02-widget-catalog.md` | 33 TUI widgets |
| Demo concepts | `tmp/demo-req/DEMO-CONCEPTS.md` | 6 demo concepts (Race, Fleet, Compounding, etc.) |
| xterm spec | `tmp/demo-req/XTERM-TERMINAL.md` | PTY architecture, xterm.js integration |
| Demo resources | `demo/demo-resources/OVERVIEW.md` | Existing shell script demos |

## Current State

Three separate HTML files with inconsistent UX:

| File | Purpose | Issues |
|---|---|---|
| `index.html` | Scripted animation | No live server, fake output |
| `terminal.html` | Multi-pane PTY | No context, just raw terminals |
| `builder.html` | Agent builds code | Gate detection is fragile heuristic |

## Target State

One page at `/demo/` with:
1. **Top nav** — scenario selector tabs
2. **Main area** — xterm.js terminal(s) with ROSEDUST theme
3. **Side panel** — live metrics, file tree, gate status
4. **Bottom bar** — cost ticker, model info, controls (play/pause/reset)
5. **Multiple scenarios** — each tab is a different demo concept
