# Post-Parity Runner

**Purpose**: Full roko maturation — from wiring fixes through architecture convergence to GTM readiness.
**Assumes**: All 195 mega-parity batches landed on `wp-arch2`.
**Approach**: Merged implementation plans (01-19) with existing post-parity wiring tasks, ordered by impact.
**Runner format**: Uses parallel-template (same as mega-parity) — codex, worktrees, cherry-pick, 20 concurrent.

## Summary

| Phase | Prefix | Batches | Focus |
|---|---|---|---|
| Original Post-Parity | PA-PAK | 161 | Wiring fixes users see and feel |
| Phase 0: Critical Fixes | S0_ | 25 | Panics, security, config, learning wiring |
| Phase 1: Architecture | O1_, D1_, G1_, CD_ | 43 | Runtime convergence, dispatch unification, gate pipeline |
| Phase 2-3: UX & Innovation | QA_, LF_, GE_, UX_, RF_, IN_ | 47 | Prompt assembly, learning loops, performance, innovations |
| Phase 4: GTM & Hardening | SF_, OB_, GT_, AC_, RP_, XC_, TV_ | 54 | Safety, observability, integrations, testing |
| Phase 5: ACP Deferred Items | AS_, AF_, AD_, AT_ | 15 | Session concurrency + Mood Ring + Dream Journal + Tournament |
| **Total** | | **345** | |

## Original Runners (PA-PAK, 161 batches)

| Runner | ID | Focus | Batches |
|---|---|---|---|
| A | PA_ | Shared HTTP client — fix 7-43s latency regression | 5 |
| B | PB_ | Chat dispatch enrichment — system prompt, tools, history | 8 |
| C | PC_ | Streaming — wire StreamingState to live token deltas | 6 |
| D | PD_ | Slash commands — /system, /effort, /gate, /config | 6 |
| E | PE_ | Safety fail-closed — permissions, dispatcher safety | 8 |
| F | PF_ | orchestrate.rs freeze — default off, compile warning | 3 |
| G | PG_ | Memory + bug fixes — efficiency_events leak, TUI | 4 |
| H | PH_ | Plan execution — parallel tasks, cargo semaphore, timeout | 5 |
| I | PI_ | Learning loop — episode enrichment, thresholds | 5 |
| J | PJ_ | Persistence & resume — snapshots, atomic JSONL | 5 |
| K | PK_ | Knowledge & routing — neuro→router, affect→routing | 6 |
| L | PL_ | ACP integration — MCP dispatch, API fallback, costs | 4 |
| M | PM_ | Merge & concurrency — merge queue, warm pool | 3 |
| N | PN_ | Demo-serve wiring — endpoints, SSE events | 7 |
| O | PO_ | Observability — cost, tokens, progress events | 6 |
| P | PP_ | CLI end-to-end — learning loop, session resume | 6 |
| Q | PQ_ | Code intelligence — HDC, PageRank, persistent index | 4 |
| R | PR_ | Legacy migration — PRD auto-plan, cloud jobs | 2 |
| S | PS_ | Provider dispatch — unified dispatch, errors | 4 |
| T | PT_ | ACP learning — episodes, efficiency, routing | 3 |
| U | PU_ | Config hot-reload — file watcher | 1 |
| V | PV_ | Runner data quality — feedback, timestamps | 5 |
| W | PW_ | Demo bench + dashboard — endpoints, c-factor | 6 |
| X | PX_ | Critical bugs — AffectPolicy dup, TOCTOU | 4 |
| Y | PY_ | Workflow convergence — PRD/research/plan via WE | 4 |
| Z | PZ_ | Agent process safety — PID race, contracts | 4 |
| AA | PAA_ | Demo workflow + terminal — PTY WS, SSE | 3 |
| AB-AK | PAB-PAK | Extended: safety, parsing, gates, knowledge, UX, config, testing | 40 |

## Phase 0: Critical Fixes (S0_, 25 batches)

| Group | Focus | Batches | Wave |
|---|---|---|---|
| S0A | P0 panics & security | 4 | 1 |
| S0B | Config consistency | 4 | 1 |
| S0C | Learning wiring | 4 | 1 |
| S0D | Env var elimination | 4 | 1 |
| S0E | ServiceFactory completeness | 3 | 2 (deps S0B) |
| S0F | Config validation | 3 | 1 |
| S0G | Feature flags & safety | 3 | 1 |

## Phase 1: Architecture Convergence (43 batches)

| Group | Prefix | Focus | Batches | Wave |
|---|---|---|---|---|
| O1A | O1_ | Worktree integration | 3 | 1 |
| O1B | O1_ | Parallel execution | 3 | 1 (deps O1A) |
| O1C | O1_ | Context handoff | 3 | 2 (deps O1B) |
| D1A | D1_ | CascadeRouter wiring | 5 | 1 |
| D1B | D1_ | Episode logging | 3 | 1 |
| D1C | D1_ | Dispatch unification | 4 | 2 (deps D1B) |
| D1D | D1_ | Provider health | 3 | 2 |
| D1E | D1_ | Budget enforcement | 3 | 2 |
| G1A | G1_ | Gate foundation | 3 | 1 |
| G1B | G1_ | Gate convergence | 5 | 2 (deps G1A) |
| G1C | G1_ | Adaptive intelligence | 3 | 2 |
| CD_A | CD_ | Code debt cleanup | 5 | 1 |

## Phase 2-3: UX, Prompt, Learning, Innovation (47 batches)

| Group | Prefix | Focus | Batches | Wave |
|---|---|---|---|---|
| QA_A | QA_ | Prompt assembly: model-aware windowing | 5 | 3 |
| LF_A | LF_ | Learning: universal wiring | 6 | 2 |
| LF_B | LF_ | Learning: anomaly & intervention | 3 | 3 |
| GE_A | GE_ | Gate evolution: roko-eval foundation | 4 | 3 |
| GE_B | GE_ | Gate evolution: bridge + criteria | 4 | 4 |
| UX_A | UX_ | CLI UX: foundation (next, summary, dry-run) | 3 | 2 |
| UX_B | UX_ | Context packs MVP | 2 | 3 |
| RF_A | RF_ | Performance: low-hanging | 5 | 1 |
| RF_B | RF_ | Performance: gate optimization | 3 | 2 |
| RF_C | RF_ | Performance: warm pool | 3 | 3 |
| IN_A | IN_ | Innovation: agent memory | 3 | 4 |
| IN_B | IN_ | Innovation: self-improving gates | 3 | 4 |
| IN_C | IN_ | Innovation: multi-agent | 3 | 4 |

## Phase 4: GTM, Safety, Observability (54 batches)

| Group | Prefix | Focus | Batches | Wave |
|---|---|---|---|---|
| SF_A | SF_ | Safety: contract enforcement | 5 | 1 |
| SF_B | SF_ | Safety: permission system | 3 | 2 |
| SF_C | SF_ | Safety: MCP & audit | 4 | 2 |
| OB_A | OB_ | Observability: tracing | 4 | 2 |
| OB_B | OB_ | Observability: dashboard | 4 | 3 |
| OB_C | OB_ | Observability: alerting | 3 | 3 |
| GT_A | GT_ | GTM: adapter foundation | 3 | 3 |
| GT_B | GT_ | GTM: GitHub integration | 3 | 4 |
| GT_C | GT_ | GTM: Linear integration | 2 | 4 |
| AC_A | AC_ | ACP: session improvements | 3 | 2 |
| AC_B | AC_ | ACP: parallel agents | 3 | 3 |
| RP_A | RP_ | Runner patterns | 5 | 2 |
| XC_A | XC_ | Cross-cutting: error handling | 3 | 1 |
| XC_B | XC_ | Cross-cutting: lifecycle | 3 | 2 |
| TV_A | TV_ | Testing: infrastructure | 3 | 1 |
| TV_B | TV_ | Testing: smoke tests | 3 | 2 |

## Phase 5: ACP Deferred Items (AS_, AF_, AD_, AT_, 15 batches)

Tracks the items left over after the original mega-parity ACP work landed (R3_F0x / R5_F0x / R7_F0x — see `tmp/solutions/acp/REMAINING.md`). Each prompt embeds its own `Issue Tracker` section with cross-batch checkboxes; the canonical tracker is `tmp/solutions/acp/REMAINING.md`.

| Group | Prefix | Focus | Batches | Wave |
|---|---|---|---|---|
| AS_A | AS_ | Wrap `SessionManager` in `Arc<RwLock>` | 1 | 1 |
| AF_A | AF_ | Mood Ring surface (enable + type + emit) | 3 | 2 |
| AF_B | AF_ | Affect-driven modulation (escalate + ConfigOptionUpdate) | 2 | 3 |
| AD_A | AD_ | Dream Journal at session start (load + render + dedupe + routing) | 4 | 2 |
| AT_A | AT_ | Tournament plumbing (variant + config + data model) | 2 | 3 |
| AT_B | AT_ | Tournament execution (spawn + parallel) | 2 | 4 |
| AT_C | AT_ | Tournament selection (compare card + permission + apply) | 1 | 4 |

## Execution DAG

### Original Post-Parity Waves

```
Wave 1 (16 parallel): PA, PE, PF, PG, PH, PJ, PO, PQ, PS, PU, PV, PW, PX, PZ, PAA, PAB
Wave 2 (6): PB←PA, PI←PH, PM←PH, PR←PF, PN←PA+PO, PY
Wave 3 (6): PC←PB, PD←PB, PK←PI, PL←PA, PP←PB, PT←PI
```

### New Phase Waves (interleaved with above)

```
Wave 1 (parallel with original Wave 1):
  S0A, S0B, S0C, S0D, S0F, S0G     (22 Phase 0 batches)
  O1A, O1B, D1A, D1B, G1A, CD_A    (22 Phase 1 batches)
  RF_A                               (5 Performance batches)
  SF_A, XC_A, TV_A                   (11 Phase 4 batches)

Wave 2 (parallel with original Wave 2):
  S0E                                (3 batches, deps S0B)
  O1C, D1C, D1D, D1E, G1B, G1C     (21 Phase 1 batches)
  LF_A, UX_A, RF_B                  (12 Phase 2-3 batches)
  SF_B, SF_C, OB_A, AC_A, RP_A, XC_B, TV_B  (24 Phase 4 batches)

Wave 3:
  QA_A, LF_B, GE_A, UX_B, RF_C    (17 Phase 2-3 batches)
  OB_B, OB_C, GT_A, AC_B           (13 Phase 4 batches)

Wave 4:
  GE_B, IN_A, IN_B, IN_C           (13 Phase 2-3 batches)
  GT_B, GT_C                        (5 Phase 4 batches)
```

### ACP Deferred Item Waves

```
Wave 1 (parallel, no deps):
  AS_01    AF_01    AD_01    AT_01
Wave 2 (per-group seconds):
  AF_02    AD_02    AT_02
Wave 3 (per-group thirds):
  AF_03    AD_03    AD_04    AT_03
Wave 4 (per-group fourths):
  AF_04    AT_04
Wave 5 (finalizers):
  AF_05    AT_05
```

## Running

```bash
# Full run (20 concurrent)
./run.sh

# Resume interrupted run
./run.sh --continue

# Run specific groups
./run.sh --group S0A
./run.sh --only S0_01,S0_02,S0_03

# Cherry-pick to target branch
./lib/auto-pick.sh --interval 90 --target-branch wp-arch2

# Monitor
./run.sh --watch
./run.sh --status
```
