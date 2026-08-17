# Design Documents Index

Quick reference to all relevant design docs scattered across tmp/. Organized by topic.

## CLI Redesign & Workflow

| Doc | Path | Key Proposal | Size |
|-----|------|--------------|------|
| 5 Verbs Proposal | `tmp/subsystem-audits/05-01/42-workflow-redesign-suggestion.md` | Replace 35 subcommands with do/think/show/tune/undo | ~8KB |
| Workflow Engine | `tmp/mori-diffs/36-WORKFLOW-ENTRYPOINT-ORCHESTRATION-AUDIT.md` | WorkflowRequest→Resolver→Plan→Executor spine | ~12KB |
| Entry Point Convergence | `tmp/workflow/implementation-plans/11-entry-point-convergence.md` | Unify all command entry points, delete roko chat | ~6KB |
| CLI/TUI Rendering | `tmp/workflow/implementation-plans/16-cli-tui-rendering-convergence.md` | ResponseRenderer trait, collapse 5 render modes | ~8KB |
| CLI Chat TUI Audit | `tmp/workflow/10-cli-chat-tui-audit.md` | All 5 user-facing modes documented | ~10KB |
| UX Workflow Vision | `tmp/solutions/roko/09-UX-WORKFLOW-VISION.md` | Aggregate→Funnel→Execute as primary workflow | ~72KB |
| UX Plan (6 phases) | `tmp/solutions/roko/15-UX-PLAN.md` | Implementation plan with type definitions | ~15KB |
| UX Issues | `tmp/solutions/roko/15-UX-ISSUES.md` | I-UX01 through I-UX07 with resolutions | ~8KB |
| UX Goals | `tmp/solutions/roko/15-UX-GOALS.md` | 6 design principles (workflow-first, progressive context) | ~6KB |
| UX Audit | `tmp/solutions/roko/15-UX-AUDIT.md` | 4 critical pain points, 5 anti-patterns | ~10KB |
| UX Subsystem Plan | `tmp/subsystem-audits/ux/PLAN.md` | ingest/funnel/next commands, TaskSpec tiering | ~6KB |
| Anti-Patterns | `tmp/workflow/ANTI-PATTERNS.md` | 10 anti-patterns from CLI proliferation | ~5KB |

## Demo & Frontend

| Doc | Path | Key Proposal | Size |
|-----|------|--------------|------|
| Scenario Redesign v2 | `tmp/solutions/demo-running/SCENARIO-REDESIGN.md` | 5 focused scenarios with custom sidebar panels | ~30KB |
| Scenario Details | `tmp/solutions/demo-running/SCENARIO-DETAILS.md` | Full specs for each new scenario | ~17KB |
| Demo Concepts | `tmp/demo-req/DEMO-CONCEPTS.md` | 6 demo concepts (Race, Fleet, Compounding, etc.) | ~15KB |
| Implementation Plan (Inline) | `tmp/demo-req/IMPLEMENTATION-PLAN.md` | Clack-style output, 18 primitives, ROSEDUST theme | ~20KB |
| Demo Redesign Audit | `tmp/demo-redesign/AUDIT.md` | roko-serve + demo terminal layer issues | ~12KB |
| Demo UI Redesign | `tmp/solutions/demo-running/04-DEMO-UI-REDESIGN.md` | CommandList + ContextPanel pattern | ~12KB |
| Streaming Design | `tmp/solutions/demo-running/06-STREAMING-DESIGN.md` | SSE streaming for plan run | ~7KB |
| Terminal Session Redesign | `tmp/solutions/demo-running/TERMINAL-SESSION-REDESIGN.md` | PTY fixes, marker detection, model gating | ~15KB |
| Scenario Audit | `tmp/solutions/demo-running/SCENARIO-AUDIT.md` | Diagnosis of existing 14 scenarios | ~8KB |

## Architecture & System

| Doc | Path | Key Proposal | Size |
|-----|------|--------------|------|
| Lessons & Approaches | `tmp/solutions/roko/01-LESSONS-AND-APPROACHES.md` | 7 architectures tried, cheapest was correct | ~20KB |
| ACP & Workflow Patterns | `tmp/solutions/roko/02-ACP-AND-WORKFLOW-PATTERNS.md` | Unified convergence via ACP state machine | ~15KB |
| Current State & Gaps | `tmp/solutions/roko/05-CURRENT-STATE-AND-GAPS.md` | 130 tasks: 90 solid, 37 partial, 3 hollow | ~25KB |
| Binary Issues Master | `tmp/binary-issues/MASTER-INDEX.md` | 90+ issues with root cause analysis | ~30KB |
| Config & Errors | `tmp/binary-issues/18-CONFIG-AND-ERRORS.md` | First-run UX, cat default, two init paths | ~8KB |
| Slash Commands | `tmp/binary-issues/11-SLASH-COMMANDS.md` | 55+ proposed slash commands, 8 categories | ~12KB |
| Quality of Life | `tmp/binary-issues/10-QUALITY-OF-LIFE.md` | Welcome banner, smart prompts, session auto-save | ~6KB |
| Command Categories | `tmp/solutions/ide/10-bare-mode-commands.md` | Tag all 47 commands with categories | ~5KB |

## Dogfood & Demo Prep

| Doc | Path | Key Proposal | Size |
|-----|------|--------------|------|
| Dogfood Context | `tmp/dogfood/CONTEXT.md` | April 26 dogfood sessions, 6 critical fixes | ~8KB |
| Dogfood Master | `tmp/dogfood/00-INDEX.md` | 56 items checklist | ~10KB |
| May 6 Demo Build | `tmp/dogfood/09-MAY6-DEMO-BUILD.md` | nunchi CLI wrapper, roko audit command | ~6KB |
| Build Plan | `tmp/learnings3/08-BUILD-PLAN.md` | Formatted output as Tier 0 for demo-readiness | ~5KB |
| Pipeline Run Audit | `tmp/solutions/demo-running/PIPELINE-RUN-AUDIT.md` | Live pipeline test results | ~12KB |

## UX Follow-up (Post PR-13)

| Doc | Path | Key Proposal | Size |
|-----|------|--------------|------|
| UX Followup Index | `tmp/ux/ux-followup/00-INDEX.md` | 112 items, 72 done, 40 open | ~8KB |
| Advanced Agent Backends | `tmp/ux/ux-followup/06-advanced-agent-backends.md` | 6 items, 0 done — Codex/Cursor parity | ~4KB |
| TUI Event Parity | `tmp/ux/ux-followup/12-tui-event-parity.md` | 11 items, 7 open | ~5KB |
| Phase 2 Vision | `tmp/ux/ux-followup/08-phase-2-vision.md` | Chain/dreams/HTTP roadmap | ~4KB |

## Implementation Status Docs (In This Folder)

| Doc | Path | What |
|-----|------|------|
| Current State | `CURRENT-STATE.md` | Definitive truth about what's wired vs dead |
| Next Phase | `NEXT-PHASE.md` | Implementation plan for CLI + demo overhaul |
| CLI Redesign | `CLI-REDESIGN.md` | Synthesized CLI proposal from all sources |
| Wiring Audit | `WIRING-AUDIT.md` | Dead code catalog with wiring instructions |
| Task Checklist | `03-TASK-CHECKLIST.md` | Original 200-item checklist (mostly done) |
| 00-INDEX.md | `00-INDEX.md` | Original batch execution guide (56 batches) |
| Improvements | `IMPROVEMENTS.md` | 111KB of detailed improvement notes |

## Reading Order (Recommended)

For understanding where things are:
1. **CURRENT-STATE.md** — what's actually working right now
2. **WIRING-AUDIT.md** — what's dead code

For planning next steps:
3. **CLI-REDESIGN.md** — the unified CLI proposal
4. **NEXT-PHASE.md** — implementation plan with waves
5. **SCENARIO-REDESIGN.md** — the 5 demo scenarios

For deep dives:
6. Pick specific docs from the tables above based on area of interest

## Key Themes Across All Docs

1. **One command, not six** — `roko do "intent"` replaces the entire PRD→plan→run pipeline
2. **Progressive formality** — system auto-classifies complexity, user never picks the pipeline
3. **Streaming everything** — SSE from serve, inline Clack-style in CLI, live sidebar in demo
4. **Kill duplicates** — one execution engine, one rendering pipeline, one init path
5. **Show the learning** — make cascade router, episodes, efficiency visible to users
6. **Custom sidebar panels** — demo app needs scenario-specific React components, not generic lists
