# ACP Task Batches — Ready to Bundle

## Batch Definitions

These can be added to mega-parity as an additional runner (Runner 8: ACP Integration)
or run as a standalone follow-up.

---

### ACP-W01: Replace Static Prompts with SystemPromptBuilder

**Scope:** `crates/roko-acp/src/bridge_events.rs`, `crates/roko-acp/src/session.rs`, `crates/roko-acp/Cargo.toml`

**What:**
1. Add `roko-compose` to Cargo.toml dependencies
2. Remove `CODE_MODE_SYSTEM_PROMPT`, `PLAN_MODE_SYSTEM_PROMPT`, `RESEARCH_MODE_SYSTEM_PROMPT` constants
3. Call `SystemPromptBuilder::new().with_mode(session.mode).with_role(role).build()`
4. Pass enrichment context (workspace info, active file, etc.)

**Gate:** cargo check + clippy + existing ACP tests pass

**Depends on:** Runner 5 (dispatch standardization may change builder interface)

---

### ACP-W02: Wire Episode Logging

**Scope:** `crates/roko-acp/src/bridge_events.rs`, `crates/roko-acp/Cargo.toml`

**What:**
1. Add `roko-learn` to Cargo.toml dependencies
2. Initialize `EpisodeLogger` in `run_acp_server()`
3. On each dispatch: `logger.begin_turn(session_id, model, prompt)`
4. On each response complete: `logger.end_turn(session_id, outcome, usage)`
5. On session end: `logger.close_episode(session_id)`

**Gate:** cargo check + clippy + verify `.roko/episodes.jsonl` gets entries after ACP dispatch

**Depends on:** Runner 2 (execution contract — defines the episode emission interface)

---

### ACP-W03: Wire CascadeRouter

**Scope:** `crates/roko-acp/src/bridge_events.rs`, `crates/roko-acp/src/session.rs`

**What:**
1. On dispatch: `cascade_router.suggest(task_hint, effort_level)` → use suggested model if routing=auto
2. On outcome: `cascade_router.record(model, outcome, latency_ms, tokens)`
3. Respect `routing_mode` config option (auto vs manual)

**Gate:** cargo check + clippy + test that router file updates after ACP dispatch

**Depends on:** Runner 3 (model selection — ensures CascadeRouter is a stable service)

---

### ACP-W04: Wire Safety Contracts

**Scope:** `crates/roko-acp/src/handler.rs`, `crates/roko-acp/src/bridge_events.rs`

**What:**
1. Before dispatch: load `AgentContract` for session mode (code/plan/research)
2. Check `contract.allowed_tools` against requested operation
3. Check `contract.allowed_paths` against file targets
4. If contract missing: use permissive default (existing behavior) with warning log

**Gate:** cargo check + clippy + test that research mode blocks file writes

**Depends on:** Runner 4 (safety — ensures contract YAML format is stable)

---

### ACP-W05: Route Pipeline Through roko-agent Dispatcher

**Scope:** `crates/roko-acp/src/runner.rs`

**What:**
1. Replace `Command::new("claude").arg("--print")` calls with roko-agent dispatcher
2. Use `Dispatcher::dispatch(model, prompt, tools, config)` for Strategist/Implementer/AutoFixer/Reviewer
3. This enables pipeline phases to use ANY provider (not just Claude CLI)
4. Respect `max_tokens`, `temperature` from pipeline config

**Gate:** cargo check + clippy + pipeline integration test with mock dispatcher

**Depends on:** Runner 5 (dispatch — standardizes the dispatcher interface)

---

### ACP-W06: Wire Cost Tracking

**Scope:** `crates/roko-acp/src/runner.rs`, `crates/roko-acp/src/workflow.rs`

**What:**
1. After each pipeline phase: accumulate `input_tokens + output_tokens` in WorkflowRun
2. Calculate cost based on model pricing (from roko.toml `[models.*.pricing]`)
3. Emit `UsageUpdate` notification to editor after each phase
4. Enforce budget limit if configured (halt pipeline if exceeded)

**Gate:** cargo check + clippy + test that WorkflowRun.total_tokens > 0 after dispatch

**Depends on:** Runner 6 (projection — may define cost calculation service)

---

### ACP-W07: Session Concurrency Safety

**Scope:** `crates/roko-acp/src/session.rs`

**What:**
1. Wrap `SessionManager.sessions: HashMap` in `Arc<RwLock<>>`
2. Use read lock for `session/list`, `session/load`
3. Use write lock for `session/new`, `session/prompt`, `session/config/update`
4. This enables future multi-threaded handler (daemon mode with multiple editors)

**Gate:** cargo check + clippy + all existing session tests pass

**Depends on:** Nothing (independent)

---

### ACP-W08: File Change Notifications

**Scope:** `crates/roko-acp/src/runner.rs`, `crates/roko-acp/src/types.rs`

**What:**
1. After pipeline commit: run `git diff --stat HEAD~1` to get changed files
2. Parse output into `FileChange { path, change_type: Added|Modified|Deleted }`
3. Emit `session/update` with `file_change` notification type
4. Editor can then refresh its file tree

**Gate:** cargo check + clippy + test that parses git diff output correctly

**Depends on:** Nothing (independent)

---

### ACP-W09: Conversation History + Context Injection

**Scope:** `crates/roko-acp/src/session.rs`, `crates/roko-acp/src/bridge_events.rs`

**What:**
1. After each prompt+response: append to `session.conversation_history`
2. Before next dispatch: inject history into system prompt (FIFO trim at 64K tokens)
3. Support `includeContext` param: prepend editor-provided file contents
4. History survives session persistence (already in struct, just not used)

**Gate:** cargo check + clippy + test multi-turn conversation carries context

**Depends on:** Nothing (independent)

---

### ACP-W10: Missing Slash Commands + Polish

**Scope:** `crates/roko-acp/src/session.rs`

**What:**
1. Add `/plan-show <name>` → `roko plan show <name>`
2. Add `/plan-resume <id>` → `roko plan run --resume <id>`
3. Add `/agent-start <name>` → `roko agent start --name <name>`
4. Add `/agent-stop <name>` → `roko agent stop --name <name>`
5. Fix `/review` and `/audit` stubs to call actual review workflow

**Gate:** cargo check + clippy + test command dispatch for new commands

**Depends on:** Nothing (independent)

---

## Execution Order

```
Independent (can run anytime):
  ACP-W07 (concurrency)
  ACP-W08 (file change)
  ACP-W09 (conversation history)
  ACP-W10 (slash commands)

After mega-parity runners:
  Runner 2 done → ACP-W02 (episodes)
  Runner 3 done → ACP-W03 (cascade router)
  Runner 4 done → ACP-W04 (safety)
  Runner 5 done → ACP-W01 (system prompts), ACP-W05 (dispatcher)
  Runner 6 done → ACP-W06 (cost tracking)
```

## Estimated LOC

| Batch | New LOC | Modified LOC | Net |
|-------|---------|--------------|-----|
| ACP-W01 | ~50 | ~80 (replace statics) | ~130 |
| ACP-W02 | ~100 | ~30 | ~130 |
| ACP-W03 | ~60 | ~40 | ~100 |
| ACP-W04 | ~80 | ~20 | ~100 |
| ACP-W05 | ~120 | ~200 (replace subprocess) | ~320 |
| ACP-W06 | ~80 | ~40 | ~120 |
| ACP-W07 | ~20 | ~60 (wrap in locks) | ~80 |
| ACP-W08 | ~60 | ~20 | ~80 |
| ACP-W09 | ~80 | ~40 | ~120 |
| ACP-W10 | ~40 | ~20 | ~60 |
| **Total** | **~690** | **~550** | **~1,240** |

Small, focused changes. No big-bang rewrite.
