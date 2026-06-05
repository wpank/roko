# ACP Gap Analysis

## Critical Gaps (Learning Loop Broken)

### 1. No Episode Logging
- **Impact:** ACP sessions invisible to learning system
- **What exists:** `roko-learn` EpisodeLogger wired in orchestrate.rs
- **Fix:** Call `EpisodeLogger::record_turn()` from bridge_events dispatch
- **Shared with mega-parity?** Yes — Runner 2 (execution contract) likely covers episode emission

### 2. No CascadeRouter Feedback
- **Impact:** Model routing can't improve from ACP usage
- **What exists:** CascadeRouter in orchestrate.rs persists to `.roko/learn/cascade-router.json`
- **Fix:** After each ACP dispatch, log outcome (success/failure, latency, cost) to router
- **Shared with mega-parity?** Yes — Runner 3 (model selection) covers router training

### 3. No Safety Contracts
- **Impact:** ACP agents run with full permissions, no role auth
- **What exists:** `AgentContract` in roko-agent safety layer, wired in orchestrate.rs
- **Fix:** Load contract YAML per session mode, enforce in bridge_events before dispatch
- **Shared with mega-parity?** Yes — Runner 4 (safety) covers contract enforcement

## High-Priority Gaps (UX/Quality)

### 4. Static System Prompts (Not Using 9-Layer Builder)
- **Impact:** ACP agents get simple mode strings instead of rich context
- **What exists:** `roko-compose` SystemPromptBuilder with 9 template layers
- **Fix:** Add roko-compose dep, call `SystemPromptBuilder::build()` with session state
- **Complexity:** Medium — need to map ACP session config → builder params

### 5. No MCP Tool Dispatch
- **Impact:** MCP servers declared in session config but tool calls don't route to them
- **What exists:** MCP passthrough in roko-agent dispatcher
- **Fix:** When pipeline agent reports tool_call, check if it matches an MCP server and route
- **Complexity:** High — requires agent ↔ MCP tool loop in subprocess model

### 6. No Budget Enforcement
- **Impact:** `total_cost_usd` always 0, no spend limits
- **What exists:** Budget tracking in orchestrate.rs CFactorSummary
- **Fix:** Track token usage per provider call, accumulate in WorkflowRun, enforce limits
- **Complexity:** Low-medium (token counts available in stream, just not accumulated)

### 7. Pipeline Agents Locked to Claude CLI
- **Impact:** Strategist/Implementer/AutoFixer/Reviewer can only use Claude, not other providers
- **What exists:** `roko-agent` dispatcher supports 8 backends
- **Fix:** Route pipeline phases through roko-agent dispatcher instead of raw subprocess
- **Complexity:** Medium — need to adapt dispatcher to pipeline's spawn model

## Medium-Priority Gaps (Feature Completeness)

### 8. No Parallel Agent Execution
- **Impact:** Can't run multiple agents in parallel (tournament mode, etc.)
- **What exists:** `roko-orchestrator` ParallelExecutor, DAG scheduling
- **Fix:** ACP pipeline could spawn parallel worktrees for N-agent tournament
- **Complexity:** High

### 9. Session Concurrency Not Thread-Safe
- **Impact:** Potential data race if multiple requests arrive (shouldn't in stdio, but could in daemon mode)
- **What exists:** Raw HashMap in SessionManager
- **Fix:** Wrap in `Arc<RwLock<>>` or use DashMap
- **Complexity:** Low

### 10. No Conversation History Accumulation
- **Impact:** Multi-turn context doesn't carry across prompts properly
- **What exists:** `conversation_history` field in Session struct
- **Fix:** Append each turn (user prompt + assistant response) to history, inject into next dispatch
- **Complexity:** Low-medium

### 11. Remaining Slash Commands (4 not started)
- `/plan-show`, `/plan-resume`, `/agent-start`, `/agent-stop`
- **Complexity:** Low (just CLI subprocess calls)

### 12. No File Change Notifications
- **Impact:** Editor doesn't know which files were modified
- **What exists:** ACP spec defines `file_change` update type
- **Fix:** After pipeline commit, parse git diff and emit file_change notifications
- **Complexity:** Low

## Low-Priority Gaps (Vision / Phase 2+)

### 13. Custom Workflow Templates (.roko/workflows/*.toml)
### 14. Trigger-Based Automation (file watch, webhook, cron)
### 15. Marketplace Integration
### 16. Visual Authoring (recipe/DAG views)
### 17. Agent Following (cursor tracking + call graph)
### 18. Episode Replay in Editor
### 19. Permission Scoping (per-action approve/deny)

## Anti-Patterns to Fix

| Issue | Fix |
|-------|-----|
| Gate thresholds shared file (race) | Use file lock or coordinator |
| Brittle file count estimation | Use `git diff --stat` after commit |
| Dead WorkflowRun cost fields | Wire to actual provider usage |
| `dangerously-skip-permissions` hardcoded | Respect session permission config |
