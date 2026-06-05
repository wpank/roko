# ACP Batches — Mixed Into Mega-Parity Runners

## Batch Distribution

ACP integration batches distributed across existing runners by domain:

| Runner | New Group | Batches | What |
|--------|-----------|---------|------|
| R3 (agent-session) | 3F | 4 | ACP uses ChatAgentSession + dispatcher + safety + permissions |
| R5 (telemetry-learning) | 5F | 5 | ACP emits episodes + feeds cascade router + knowledge cards |
| R7 (mori-polish) | 7F | 6 | ACP conversation history, file notifications, slash commands, phase badges, narratives, context providers |

**Total: 15 new batches** across 3 runners.

### UX-Critical Batches (for screenshot parity)

| Batch | Screenshot Element |
|-------|-------------------|
| R3_F04 | Permission dialog (Allow/Always Allow/Reject) |
| R5_F05 | Knowledge store card (hits with scores) |
| R7_F04 | Phase badges with iteration tracking (⊕ Strategizing, 🔧 Auto-fixing iter 2) |
| R7_F05 | Narrative text between phases |
| R7_F06 | Context provider resolution (@-mentions) |
| R5_F03 | Token counter in status bar (24,850 / 200k) |

---

## TOML Entries (append to batches.toml)

```toml
# ============================================================
# Runner 3, Group F: ACP Agent Session Integration
# Wire ACP pipeline dispatch through roko-agent instead of raw subprocess
# ============================================================

[[batch]]
id = "R3_F01"
title = "Replace ACP raw subprocess dispatch with roko-agent Dispatcher"
group = "3F"
deps = ["R3_B05", "R3_D03"]
scope = ["crates/roko-acp/src/runner.rs", "crates/roko-acp/Cargo.toml"]
also_read = ["crates/roko-agent/src/dispatcher/mod.rs", "crates/roko-acp/src/bridge_events.rs"]
verify = "quick"

[[batch]]
id = "R3_F02"
title = "Wire SystemPromptBuilder into ACP session dispatch"
group = "3F"
deps = ["R3_F01", "R3_A02"]
scope = ["crates/roko-acp/src/bridge_events.rs", "crates/roko-acp/src/session.rs", "crates/roko-acp/Cargo.toml"]
also_read = ["crates/roko-compose/src/system_prompt_builder.rs", "crates/roko-compose/src/templates/"]
verify = "quick"

[[batch]]
id = "R3_F03"
title = "Wire safety contract enforcement into ACP dispatch"
group = "3F"
deps = ["R3_F01"]
scope = ["crates/roko-acp/src/handler.rs", "crates/roko-acp/src/bridge_events.rs"]
also_read = ["crates/roko-agent/src/safety/mod.rs", "crates/roko-agent/src/safety/contracts.rs"]
verify = "quick"

# ============================================================
# Runner 5, Group F: ACP Telemetry & Learning Integration
# Make ACP sessions participate in the learning feedback loop
# ============================================================

[[batch]]
id = "R5_F01"
title = "Wire ACP episode logging (begin_turn / end_turn / close)"
group = "5F"
deps = ["R5_A05", "R5_B01"]
scope = ["crates/roko-acp/src/bridge_events.rs", "crates/roko-acp/src/runner.rs", "crates/roko-acp/Cargo.toml"]
also_read = ["crates/roko-learn/src/episodes.rs", "crates/roko-cli/src/orchestrate.rs"]
verify = "quick"

[[batch]]
id = "R5_F02"
title = "Feed ACP dispatch outcomes into CascadeRouter"
group = "5F"
deps = ["R5_F01", "R5_C01"]
scope = ["crates/roko-acp/src/bridge_events.rs", "crates/roko-acp/src/runner.rs"]
also_read = ["crates/roko-learn/src/cascade_router.rs"]
verify = "quick"

[[batch]]
id = "R5_F03"
title = "Wire ACP cost/token tracking from provider responses"
group = "5F"
deps = ["R5_F01", "R5_B03"]
scope = ["crates/roko-acp/src/bridge_events.rs", "crates/roko-acp/src/runner.rs", "crates/roko-acp/src/workflow.rs"]
also_read = ["crates/roko-learn/src/efficiency.rs"]
verify = "quick"

[[batch]]
id = "R5_F04"
title = "ACP telemetry integration proof (episode + cost + router)"
group = "5F"
deps = ["R5_F02", "R5_F03"]
scope = ["crates/roko-acp/tests/"]
also_read = []
verify = "full"

[[batch]]
id = "R5_F05"
title = "Query knowledge store and emit knowledge card at dispatch time"
group = "5F"
deps = ["R5_F01"]
scope = ["crates/roko-acp/src/bridge_events.rs", "crates/roko-acp/src/runner.rs", "crates/roko-acp/Cargo.toml"]
also_read = ["crates/roko-neuro/src/lib.rs", "crates/roko-cli/src/orchestrate.rs"]
verify = "quick"

# ============================================================
# Runner 7, Group F: ACP UX Polish & Completeness
# Independent improvements to ACP session behavior
# ============================================================

[[batch]]
id = "R7_F01"
title = "ACP conversation history accumulation + context injection"
group = "7F"
deps = []
scope = ["crates/roko-acp/src/session.rs", "crates/roko-acp/src/bridge_events.rs"]
also_read = []
verify = "quick"

[[batch]]
id = "R7_F02"
title = "ACP file change notifications via git diff after pipeline commit"
group = "7F"
deps = []
scope = ["crates/roko-acp/src/runner.rs", "crates/roko-acp/src/types.rs"]
also_read = []
verify = "quick"

[[batch]]
id = "R7_F03"
title = "ACP missing slash commands + session concurrency (Arc<RwLock>)"
group = "7F"
deps = []
scope = ["crates/roko-acp/src/session.rs"]
also_read = []
verify = "quick"

[[batch]]
id = "R7_F04"
title = "Phase badge inline emission + iteration tracking in pipeline"
group = "7F"
deps = []
scope = ["crates/roko-acp/src/pipeline.rs", "crates/roko-acp/src/runner.rs"]
also_read = []
verify = "quick"

[[batch]]
id = "R7_F05"
title = "Emit narrative text between pipeline phases"
group = "7F"
deps = ["R7_F04"]
scope = ["crates/roko-acp/src/runner.rs"]
also_read = []
verify = "quick"

[[batch]]
id = "R7_F06"
title = "Context provider registry + resolution at prompt time"
group = "7F"
deps = []
scope = ["crates/roko-acp/src/session.rs", "crates/roko-acp/src/bridge_events.rs"]
also_read = ["crates/roko-acp/src/types.rs"]
verify = "quick"
```

---

## Dependency DAG

```
Runner 3 (agent-session):
  R3_B05 (Claude CLI turn tests) ──┐
  R3_D03 (deprecate dispatch_direct)┼──→ R3_F01 (ACP dispatcher) ──→ R3_F02 (system prompts)
  R3_A02 (resolve system prompt) ───┘                              ├──→ R3_F03 (safety contracts)
                                                                   └──→ R3_F04 (permission bridge)

Runner 5 (telemetry-learning):
  R5_A05 (display unknown) ──┐
  R5_B01 (identify emitters) ┼──→ R5_F01 (ACP episodes) ──→ R5_F02 (cascade router)
  R5_C01 (feed outcomes) ────┘         │                └──→ R5_F03 (cost tracking)
  R5_B03 (gate outcome) ──────────────────────────────────────┘         │
                                       └──→ R5_F05 (knowledge cards)    ▼
                                                               R5_F04 (integration proof)

Runner 7 (mori-polish):
  (no deps) ──→ R7_F01 (conversation history)
  (no deps) ──→ R7_F02 (file change notifications)
  (no deps) ──→ R7_F03 (slash commands + concurrency)
  (no deps) ──→ R7_F04 (phase badges + iteration) ──→ R7_F05 (narrative text)
  (no deps) ──→ R7_F06 (context providers)
```

---

## Anti-Patterns (for context-pack)

Add to `context-pack/00-RULES.md` under a new section:

```markdown
## ACP Integration (Groups 3F, 5F, 7F)

- ACP-1: One dispatch path — pipeline phases go through roko-agent Dispatcher,
  never raw `Command::new("claude")` (remove the subprocess calls in runner.rs)
- ACP-2: One prompt assembly — use SystemPromptBuilder from roko-compose,
  remove static `CODE_MODE_SYSTEM_PROMPT` / `PLAN_MODE_SYSTEM_PROMPT` / `RESEARCH_MODE_SYSTEM_PROMPT` strings
- ACP-3: No silent learning gaps — every ACP dispatch MUST emit an episode turn
  and a cascade router observation. If a dispatch completes without logging, it's a bug.
- ACP-4: No dead metrics — `WorkflowRun.total_cost_usd` and `total_tokens` must
  reflect actual provider usage. Zero is not valid after a successful dispatch.
- ACP-5: Session state is shared-nothing — AcpSession owns its own history and
  config, does not read global state except via the shared services (FeedbackService,
  PromptAssemblyService, SafetyLayer).
```

---

## Execution Strategy

1. **Wave 1 (parallel):** R7_F01, R7_F02, R7_F03 — independent, no deps
2. **Wave 2 (after R3 core):** R3_F01 — depends on session infrastructure
3. **Wave 3 (after R3_F01):** R3_F02, R3_F03 — system prompts + safety
4. **Wave 4 (after R5 core):** R5_F01 — episode logging
5. **Wave 5 (after R5_F01):** R5_F02, R5_F03 — cascade router + cost
6. **Wave 6 (final):** R5_F04 — integration proof test

Since mega-parity runs 8 parallel agents, the R7_F* batches will execute in Wave 1
alongside other no-dep batches. R3_F* and R5_F* will execute as their deps resolve.
