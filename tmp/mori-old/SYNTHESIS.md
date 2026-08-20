# Mori → Roko: Master Synthesis

> From 17 parallel deep-dive agents, 2026-08-19.

## The Headline

**Roko's infrastructure is genuinely superior to Mori's. Roko's UX is genuinely inferior.**

The rewrite succeeded architecturally — learned routing, adaptive thresholds, 11 providers,
safety layers, prompt experiments, checksummed state, no SQLite dependency, 365 HTTP routes
serving real data. The core CLI commands are genuinely wired end-to-end.

The rewrite failed experientially — the TUI is 4x larger but less functional, there are no
operator recovery actions, learning data is fragmented across surfaces, the queue/wave/milestone
UX that made mori intuitive doesn't exist, and 8 of 12 cybernetic features only work through
HTTP, not through the actual execution path.

## What Roko Got Right (Keep)

1. **Single unified architecture** — merged golem + mori stacks
2. **Learned model routing** (CascadeRouter with LinUCB bandits)
3. **Adaptive gate thresholds** (EMA per rung)
4. **19 gates / 7 rungs** (vs mori's 11 gates)
5. **11 provider kinds** (vs mori's 3)
6. **9-layer SystemPromptBuilder** with bidder auctions
7. **Checksummed state persistence** with fingerprinted resume
8. **Immutable-tip git model** (crash-safe, journaled)
9. **Production-grade HTTP layer** (~365 routes, real auth, SSE/WS)
10. **Safety stack** (trust-origin IFC, immune graph, corrigibility)
11. **Prompt A/B experiments**
12. **No SQLite dependency**
13. **Daimon affect modulation** at dispatch
14. **Dreams consolidation** producing routing advice
15. **HDC fingerprinting** for episode similarity

## What Mori Got Right (Port to Roko)

### Priority 1: Execution UX (highest impact)

| Gap | What Mori Had | Effort | Impact |
|---|---|---|---|
| **Queue manifest** | `queue.toml` with named milestones, maintenance batches, session settings | Medium | Critical — this is how operators organize work |
| **Wave computation** | Kahn's algorithm DAG across plans with parallelism visibility | Medium | Critical — operators can't see execution order without this |
| **Express mode** | Skip strategist/reviews, auto-fix on gate failure, 40-60% time savings | Medium | High — essential for batch runs |
| **Conductor supervisor** | Persistent agent that orchestrates, nudges, force-advances | Medium | High — the "brain" that keeps execution moving |
| **TUI recovery actions** | 5 keybindings: retry, diagnose, repair, clean-slate, reverify | Low | High — operators are currently read-only |
| **LLM failure reflections** | Background Haiku call on gate failure → structured analysis → injected into retry | Low | High — agents retry blind without this |
| **Preflight at plan-run** | Validate config, providers, disk before execution starts | Low | High — fail fast instead of mid-execution |

### Priority 2: TUI Quality

| Gap | What Mori Had | Effort | Impact |
|---|---|---|---|
| **ROSEDUST palette** | Warm rose-tinted greys, 3 semantic colors, no pure white | Low | Medium — visual polish |
| **Information-dense header** | Single row: heartbeat, wave, queue, progress bar, ETA, cost, tokens, MCP | Low | Medium — at-a-glance status |
| **Adaptive frame rate** | 60fps idle → 20fps when agents busy | Low | Medium — CPU savings |
| **Exponential smoothing** | Alpha ~0.12 on all metrics to prevent visual jumps | Low | Medium — visual polish |
| **Context-sensitive keybinds** | Hints change per tab + focus zone | Low | Medium — discoverability |
| **Content-aware badges** | Error counts, agent counts on inactive tabs | Low | Medium — peripheral awareness |
| **VFX system** | Particle physics, data rain, bloom/vignette, panel shadows | Medium | Low — nice-to-have polish |
| **Plan tree widget** | Collapsible Wave→Plan with progress bars, health, wave blockers | Medium | High — the primary view |
| **Error digest widget** | Aggregated gate/pipeline/preflight/runtime errors | Low | High — error visibility |

### Priority 3: Observability

| Gap | What Mori Had | Effort | Impact |
|---|---|---|---|
| **Single-pane F7:inspect** | Episodes, playbook, routing, prompts, fixtures, history in one view | Medium | High — learning visibility |
| **Per-model/provider/strategy cost stats** | Pass rate, avg duration, retry rate, cost/run, total runs | Low | Medium — cost awareness |
| **Full prompt text logging** | Complete prompt text stored per invocation | Low | Medium — debugging |
| **Auto-generated per-worktree MCP configs** | MCP configs generated for all backends in every worktree | Medium | Medium — agent tooling |
| **Fixture lifecycle display** | Live PID, key, uptime in TUI | Low | Low — operational visibility |

### Priority 4: Agent Configuration

| Gap | What Mori Had | Effort | Impact |
|---|---|---|---|
| **Role preset config** | Per-role model overrides as simple HashMap | Low | Medium — easy tuning |
| **Effort/priority per role** | High/Medium/Low per role, visible in TUI | Low | Low — visible in config tab |
| **Agent status panel** | All ~30 roles with model, tokens, effort, status | Low | Medium — agent awareness |
| **Warm process pools** | Pre-spawned agent processes for faster dispatch | Medium | Medium — latency reduction |

## What to Remove or Simplify

1. **CorticalState** (2,717 LOC) — never instantiated in production. Either wire it or remove it.
2. **Inference Gateway bypass** — runner-v2 completely bypasses the 9-stage gateway. Either route
   through it or acknowledge it's HTTP-only.
3. **Two parallel page systems** in TUI (`PageId`/`PageScaffold` + `Tab`/`SubView`) — pick one.
4. **Two parallel data models** in TUI (`DashboardData` + `TuiState`) — unify.
5. **Advanced HDC math** (TDA, tropical algebra, sheaf Laplacian) — exists but unused. Archive.
6. **Agent Groups coordination modes** — unimplemented enum variants. Stub or implement.

## God Objects to Decompose

1. **`event_loop.rs`** (~23K lines) — the runner god object
2. **`app.rs`** (4,576 lines) — the TUI god object with 72-variant TuiAction dispatch
3. **`state.rs`** (5,290 lines) — TUI state god object
4. **`dashboard.rs`** (7,445 lines) — TUI dashboard god object

## Recommended Action Plan

### Phase 1: Make it work (1-2 weeks)
1. Wire queue manifest (`queue.toml`) and wave computation into runner-v2
2. Add TUI recovery keybindings (retry, diagnose, repair, reverify)
3. Wire LLM failure reflections into retry dispatch prompts
4. Run preflight checks at `roko plan run` startup
5. Add `roko diagnose <plan-id>` CLI command
6. Fix the two-data-model problem in TUI (unify DashboardData + TuiState)

### Phase 2: Make it beautiful (1-2 weeks)
7. Port ROSEDUST color palette
8. Port information-dense header bar
9. Port plan tree widget with wave grouping
10. Port error digest widget
11. Add adaptive frame rate and exponential smoothing
12. Port context-sensitive keybind hints
13. Build single-pane inspect/learning view (F7 equivalent)

### Phase 3: Make it smart (1-2 weeks)
14. Wire express mode (auto-fix on gate failure)
15. Wire conductor supervisor pattern
16. Add per-model/provider cost stats to TUI
17. Generate per-worktree MCP configs
18. Wire full prompt text logging
19. Decide on CorticalState: wire or remove
20. Decide on Inference Gateway: route through or document as HTTP-only

### Phase 4: Clean up (1 week)
21. Decompose event_loop.rs
22. Decompose app.rs / state.rs / dashboard.rs
23. Remove unused HDC advanced math
24. Resolve two parallel page systems in TUI
25. Wire or remove Agent Groups coordination modes
26. Fresh dogfood proof run

## Key Metrics

| Metric | Mori | Roko | Verdict |
|---|---|---|---|
| Total LOC | 337K | 893K | Roko 2.65x larger |
| TUI LOC | 10.3K | 44K | Roko 4x larger, less functional |
| Crates | 41 | 35 | Similar |
| Gates | 11 | 19 (7 rungs) | Roko better |
| Provider kinds | 3 | 11 | Roko better |
| HTTP routes | ~0 (gateway only) | ~365 (97% real) | Roko massively ahead |
| Learning LOC | 5.8K | 123K | Roko 21x larger |
| Production episodes | 6,600 | ~0 | Mori proven |
| Agent roles | 28 | 28 (shared enum) | Same |
| TUI recovery actions | 5 keybindings | 0 | Mori far ahead |
| Queue/wave/milestone | Full system | None | Mori far ahead |
| Prompt assembly | Monolithic, practical | 9-layer, sophisticated | Roko architecturally better |
| State persistence | Separate files, no integrity | Checksummed, fingerprinted | Roko better |
| Git model | Basic worktrees | Immutable-tip, journaled | Roko better |
| Error UX | LLM reflections + auto-fix | Read-only dashboard | Mori far ahead |
