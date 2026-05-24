# Post-Refactor Roadmap — What's Left After Runner v2 + Cleanup

> Status: **RESEARCH** — not yet actionable. Read after runner v2 + refactoring complete.

## Where We'll Be After Current Sessions

Assuming runner v2 + parallel refactoring both succeed:

| Done | What |
|------|------|
| ✅ | Clean plan runner (~2,500 lines, streaming, persistence per-task) |
| ✅ | main.rs decomposed (12K → ~500 + commands/) |
| ✅ | config/schema.rs split (6K → ~800 + sections) |
| ✅ | cascade_router.rs split (5K → ~2K + cascade/) |
| ✅ | serve routes split into directories |
| ✅ | Cell trait defined, 6 traits renamed |

## What's Still Broken (Priority Order)

### Tier 1: Actually Blocks Self-Hosting

These prevent `roko plan run` from being reliable enough to develop roko with roko.

**1. Learning loop is one-way (40% functional)**
- Episodes log to disk ✓
- But: no prediction Pulses emitted → no calibration feedback
- CascadeRouter doesn't consult knowledge store
- Post-gate reflection not wired (agents don't learn from failures)
- Error pattern sharing between runs doesn't happen
- **Effect**: System doesn't improve over time. Each run starts from scratch.
- **Fix**: Wire predict-publish-correct loop, knowledge-informed routing, reflection dispatch
- **Effort**: ~8 days

**2. Agent dispatch reliability gaps**
- No health check before dispatch (invalid API key → silent timeout)
- MCP stdio transport has no auto-restart (server crash → hung dispatch)
- Streaming failures are fatal (network drop → lost turn, no retry)
- Provider semaphore has no timeout (starvation possible)
- Safety contracts are permissive (violations warn, don't block)
- **Effect**: Agent failures are opaque and unrecoverable
- **Fix**: Health checks, MCP respawn, streaming checkpoint, semaphore timeout
- **Effort**: ~6 days

**3. No lifecycle Pulses during execution**
- Bus exists, Pulse type exists, but orchestration emits zero Pulses
- TUI gets DashboardEvents (hardcoded), not Bus Pulses
- Learning systems can't subscribe to execution events
- **Effect**: No reactive feedback, no event-driven automation
- **Fix**: Emit Pulses for flow.started, node.started/completed/failed, gate.result
- **Effort**: ~3 days

### Tier 2: Degrades Quality Significantly

**4. Demurrage not wired into Store**
- Fields exist on Engram (balance, demurrage_paid, tier)
- But Store.get() doesn't reinforce, Store.query() doesn't filter, prune doesn't archive
- Knowledge grows unbounded, stale entries never decay
- **Fix**: Wire demurrage into FileSubstrate/FileStore operations
- **Effort**: ~3 days

**5. 6,815 unwrap/expect panic sites**
- Highest risk: roko-fs (6.5% ratio), roko-orchestrator (334 unwraps), roko-agent (247 unwraps)
- Production crashes on malformed JSON, missing files, null values
- **Fix**: Systematic audit of critical paths, replace with `?` + Result
- **Effort**: ~5 days (focus on top 200 sites in I/O paths)

**6. 4 atomic write migration TODOs in roko-learn**
- `prompt_experiment.rs`, `provider_health.rs`, `error_pattern_store.rs`, `cascade_router.rs`
- Old blocking_write pattern instead of `roko_fs::atomic_write_json`
- Risk: corrupted state files on crash
- **Fix**: Mechanical — replace 4 call sites
- **Effort**: ~0.5 day

**7. Event bus double-delivery (roko-serve)**
- REST-originated events appear twice on EventBus
- Location: `crates/roko-serve/src/lib.rs:1009` (has FIXME)
- **Fix**: Deduplicate or gate by origin
- **Effort**: ~0.5 day

### Tier 3: Missing Features for Full Self-Hosting

**8. EFE routing (replace LinUCB)**
- CascadeRouter uses LinUCB — no regime conditioning (Calm/Crisis)
- Doesn't adapt exploration/exploitation based on system state
- **Fix**: Implement EFE formula, integrate regime signals from Conductor
- **Effort**: ~4 days

**9. Post-gate reflection loop**
- Struct exists (`post_gate_reflection.rs`, 21KB) but never called
- On gate failure: no lightweight agent analyzes what went wrong
- Reflection not injected into retry prompt, not stored in Episode
- **Fix**: Wire reflection dispatch on failure, store results, inject on retry
- **Effort**: ~3 days

**10. Dream cycle not auto-triggered**
- `roko-dreams` fully built but only runs on manual `roko knowledge dream run`
- No cron/interval trigger configured
- **Fix**: Add tokio_cron_scheduler, config section `[learning.dreams]`
- **Effort**: ~2 days

**11. Context injection scoping**
- All agents get same context regardless of role
- Implementer should get full code, Reviewer gets summary, Strategist gets high-level
- **Fix**: Role-filtered context in dispatch, KnowledgeConfig with toggles
- **Effort**: ~3 days

**12. Warm agent pool**
- Phase transitions lose 5-15s to agent spawn
- No pre-spawning during gate execution
- **Fix**: WarmPool struct, pre_spawn_warm/promote_warm lifecycle
- **Effort**: ~3 days

### Tier 4: Protocol Completion (Phase 1 Kernel)

**13. Observe protocol + Lenses**
- Trait not in roko-core, Lenses built in conductor but not exposed
- No HTTP routes, TUI doesn't consume Lens data
- **Effort**: ~4 days

**14. Trigger protocol**
- Not implemented at all (CronTrigger, BusTrigger, FileWatchTrigger)
- Blocks event-driven Graph firing
- **Effort**: ~4 days

**15. Connect protocol**
- MCP exists ad-hoc, Chain exists ad-hoc, no unified trait
- **Effort**: ~3 days

**16. TypeSchema validation**
- No type contracts on Cell inputs/outputs
- No edge compatibility checking at Graph load time
- **Effort**: ~3 days

### Tier 5: Quality & Polish

**17. Test coverage gaps**
- roko-cli: 0.81% test/LOC ratio (sparse for 112K LOC)
- roko-serve: 0.86% ratio
- MCP modules: <1% each
- **Fix**: Add unit tests for decision logic, endpoint tests for routes

**18. 82 crate-level lint suppressions**
- Some legitimate (template code), some hiding real issues
- `clippy::too_many_lines` in openai_agent.rs = code smell
- **Fix**: Audit each, fix root cause or document why suppressed

**19. 49 dead_code suppressions**
- Mostly Phase 2 stubs (roko-dreams, roko-daimon) — acceptable
- Some in orchestrate.rs — may be resolved by runner v2

**20. println! calls in production paths**
- 111 in roko-cli, should use tracing instead
- **Fix**: Replace with `tracing::info!` / `tracing::debug!`

**21. Agent backend consolidation**
- 7 backends follow similar patterns (spawn, parse, collect)
- Claude, Codex, Cursor each have own tool loops
- **Fix**: Consolidate around ToolLoopAgent + LlmBackend trait (already started)

**22. Gemini tool-loop not implemented**
- Returns `AgentCreationError` at runtime — no graceful fallback
- **Fix**: Implement GeminiNativeBackend in tool_loop/backends/

---

## Suggested Execution Waves

### Wave 1: Quick Wins (1-2 days, do immediately after merge)

| Item | Effort | Impact |
|------|--------|--------|
| #6 Atomic write migration (4 sites) | 0.5d | Crash safety |
| #7 Event bus double-delivery fix | 0.5d | Data integrity |
| #20 println → tracing in roko-cli | 0.5d | Observability |

### Wave 2: Learning Loop (5-8 days)

| Item | Effort | Impact |
|------|--------|--------|
| #3 Lifecycle Pulses during execution | 3d | Unlocks reactive learning |
| #1 Knowledge-informed routing | 2d | Router uses institutional knowledge |
| #9 Post-gate reflection dispatch | 3d | Agents learn from failures |

### Wave 3: Reliability Hardening (5-6 days)

| Item | Effort | Impact |
|------|--------|--------|
| #2 Agent dispatch health checks | 2d | Fail-fast on bad config |
| #2 MCP auto-restart | 1d | Resilience |
| #2 Safety contracts fail-closed | 1d | Security posture |
| #5 Top 200 unwrap → Result conversions | 2d | Crash prevention |

### Wave 4: Demurrage + Memory (3-5 days)

| Item | Effort | Impact |
|------|--------|--------|
| #4 Wire demurrage into Store ops | 3d | Knowledge lifecycle |
| #10 Dream cycle auto-trigger | 2d | Autonomous consolidation |

### Wave 5: Advanced Features (8-10 days)

| Item | Effort | Impact |
|------|--------|--------|
| #8 EFE routing | 4d | Adaptive model selection |
| #11 Context injection scoping | 3d | Role specialization |
| #12 Warm agent pool | 3d | Latency reduction |

### Wave 6: Protocol Completion (10-14 days)

| Item | Effort | Impact |
|------|--------|--------|
| #13 Observe protocol + Lenses | 4d | Live observability |
| #14 Trigger protocol | 4d | Event-driven automation |
| #15 Connect protocol | 3d | Unified connectors |
| #16 TypeSchema validation | 3d | Type safety |

### Wave 7: Quality (ongoing)

| Item | Effort | Impact |
|------|--------|--------|
| #17 Test coverage | ongoing | Regression prevention |
| #18 Lint suppression audit | 2d | Code quality |
| #21 Backend consolidation | 3d | Maintainability |
| #22 Gemini tool-loop | 2d | Backend completeness |

---

## When to Try Running Roko Again

**After runner v2 merges** — try immediately. The runner should:
- Load tasks.toml cleanly (1 plan, no phantoms)
- Skip enrichment
- Dispatch first task to claude-sonnet-4-6
- Stream output in TUI
- Persist executor.json after task completes
- Handle Ctrl+C

**After Wave 1 merges** — try with learning feedback:
- Atomic writes prevent corruption
- Event bus deduplication is clean
- Tracing works instead of println

**After Wave 2 merges** — self-hosting gets real:
- Router consults knowledge from previous runs
- Failed tasks get reflection before retry
- Lifecycle events flow through Bus for downstream consumers

**After Wave 3** — production-grade reliability:
- Bad configs fail fast instead of hanging
- MCP servers restart on crash
- Safety violations block instead of warn
- Critical paths don't panic on bad input

---

## Total Remaining Effort

| Wave | Days | Cumulative |
|------|------|-----------|
| Quick wins | 1-2 | 1-2 |
| Learning loop | 5-8 | 6-10 |
| Reliability | 5-6 | 11-16 |
| Demurrage | 3-5 | 14-21 |
| Advanced features | 8-10 | 22-31 |
| Protocol completion | 10-14 | 32-45 |
| Quality | ongoing | — |

**To basic self-hosting**: Waves 1-3 = ~16 days
**To full unified spec compliance**: Waves 1-6 = ~45 days
