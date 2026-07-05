# 03 — Safety & Hooks Wiring

> **Priority**: 🔴 P0 — Agents can run destructive commands without guardrails
> **Parity sections**: §7.1.6, I.1 (Safety wiring)
> **Checklist ref**: `MORI-PARITY-CHECKLIST.md` §7.1.6, I.1

## Problem statement

Mori's `agent_hooks_settings()` (`/Users/will/dev/uniswap/bardo/apps/mori/src/agent/connection.rs:647-750`)
builds a JSON `--settings` blob that enforces safety constraints at the Claude CLI level:

- **PreToolUse hooks**: Block `git checkout`, `git switch`, `git branch -m`, `rm -rf /`
- **These are injected via `--settings <json>`** on every agent spawn

Roko has a full safety subsystem built but disconnected:

| Component | Path | Status |
|-----------|------|--------|
| Bash guard | `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/safety/bash_guard.rs` | ✅ Built, ❌ Not wired |
| Network guard | `.../safety/network_guard.rs` | ✅ Built, ❌ Not wired |
| Git guard | `.../safety/git_guard.rs` | ✅ Built, ❌ Not wired |
| Path guard | `.../safety/path_guard.rs` | ✅ Built, ❌ Not wired |
| Capability tokens | `/Users/will/dev/nunchi/roko/roko/crates/roko-orchestrator/src/safety/capability_tokens.rs` | ✅ Built, ❌ Not wired |
| Sandboxing | `.../safety/sandboxing.rs` | ✅ Built, ❌ Not wired |
| Taint propagation | `.../safety/taint_propagation.rs` | ✅ Built, ❌ Not wired |
| Audit chain | `.../safety/audit_chain.rs` | ✅ Built, ❌ Not wired |

## Checklist

### Phase A: Claude CLI settings hooks (match mori exactly)

- [ ] **3.1** Create `fn build_settings_json(role: AgentRole) -> String` in `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/claude_cli_agent.rs` (or a shared module)
- [ ] **3.2** PreToolUse hook: block `git checkout *` → error message
- [ ] **3.3** PreToolUse hook: block `git switch *` → error message
- [ ] **3.4** PreToolUse hook: block `git branch -m *` → error message
- [ ] **3.5** PreToolUse hook: block destructive filesystem ops (`rm -rf /`, etc.)
- [ ] **3.6** Pass `--settings <json>` in ClaudeCliAgent spawn command
- [ ] **3.7** Test: verify settings JSON matches mori's `agent_hooks_settings()` output structure

### Phase B: Wire roko-agent safety guards into dispatch pipeline

- [ ] **3.8** `ToolDispatcher::dispatch()` calls bash_guard before executing Bash tools
- [ ] **3.9** `ToolDispatcher::dispatch()` calls network_guard before HTTP tools
- [ ] **3.10** `ToolDispatcher::dispatch()` calls git_guard before git operations
- [ ] **3.11** `ToolDispatcher::dispatch()` calls path_guard for file operations
- [ ] **3.12** Integration test: bash_guard blocks `rm -rf /` in a real dispatch
- [ ] **3.13** Integration test: git_guard blocks `git checkout main` in a real dispatch

### Phase C: Orchestrator safety (capability tokens + sandboxing)

- [ ] **3.14** Executor issues capability tokens to agents at spawn time
- [ ] **3.15** Sandbox config applied per-role (Implementer gets write access to worktree only)
- [ ] **3.16** Taint propagation tracks which signals touched untrusted inputs
- [ ] **3.17** Audit chain logs all tool invocations for post-hoc review

> Maps to checklist: I.1.1 through I.1.7
