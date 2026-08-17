# Safety & Agent System Audit

6 provider backends (via `ProviderKind`), layered safety checks, behavioral contracts, tool dispatch pipeline — architecturally sound but critically fails open when contracts are missing via the `contract_for_role()` path.

## The Problem

The safety and agent system is substantially wired: 6 `ProviderKind` backends (plus Ollama/Codex/ExecAgent as secondary agents outside the adapter registry), ToolDispatcher enforces pre-execution checks, orchestrator calls pre/post-dispatch checks. The critical issues: contract fail-open behavior in `contract_for_role()` (missing JSON asset → permissive default), optional safety budgets, and incomplete post-execution validation. Recovery actions ARE invoked at the dispatcher level via `check_recovery()` but only for per-tool results, not for orchestrator-level task failures.

---

## 1. AgentContract (Behavioral Contracts)

### What Contracts Define

8 bundled JSON contracts (stored with `.yaml` extension but parsed via `serde_json`) in `roko-agent/src/safety/contracts/`:
- architect, auditor, auto-fixer, implementer, researcher, reviewer, scribe, strategist

**Example (architect.yaml — JSON format):**
```json
{
  "role": "architect",
  "invariants": [
    { "MaxTokensPerTurn": 16000 }
  ],
  "governance": [
    { "MaxToolCallsPerTurn": 6 },
    { "ForbiddenTools": ["edit_file", "write_file", "multi_edit", "apply_patch", "bash"] },
    { "RequireToolBeforeEdit": "read_file" }
  ],
  "recovery": [
    { "trigger": "contract_violation", "action": "Alert" }
  ]
}
```

Note: Files have `.yaml` extension but are parsed by `serde_json::from_str` (JSON format, not YAML). The loader at `contract.rs:117-132` uses `serde_json`. The `allowed_tools` field is not present in architect's contract (no explicit allowlist — tool use gated only by `ForbiddenTools`).

### Invariants (3 types)

| Invariant | Status |
|---|---|
| MaxTokensPerTurn(u32) | Wired |
| RequireGateBeforeCommit | Wired |
| NoNetworkAccess | Wired |

### Governance Rules (5 types)

| Rule | Status |
|---|---|
| MaxToolCallsPerTurn(u32) | Wired |
| ForbiddenTools(Vec<String>) | Wired |
| MaxCostPerTurn(f64) | Partially wired — checks single-call estimated cost; TODO(UX26) for cumulative per-turn spend |
| MaxConsecutiveFailures(u32) | Wired — counts trailing failures from `ctx.external_actions` |
| RequireToolBeforeEdit(String) | Wired |

### Recovery Actions (4 types)

Retry, Downgrade, Abort, Alert — all built. `applicable_recovery()` IS invoked at the dispatcher level: `SafetyLayer::check_recovery()` is called in `dispatcher/mod.rs` after each tool result (line ~457 of dispatch method). Recovery actions are **not** invoked at the orchestrator level for task-level failures — only per-tool-call in the dispatcher.

Supported trigger strings: `"contract_violation"`, `"attempted_edit"`, `"tool_budget_exhausted"`.

### Critical: Fail-Open Behavior

The `contract_for_role()` private method in `safety/mod.rs:864` still uses permissive fallback:

```rust
fn contract_for_role(&self, role: &str) -> AgentContract {
    let mut contract = AgentContract::load_for_role(role).unwrap_or_else(|err| {
        tracing::warn!(%role, %err, "no contract for role; using permissive default");
        AgentContract::permissive(role.to_string())  // ZERO restrictions
    });
    // ... optional role_override budget injection ...
    contract
}
```

This is called by `SafetyLayer::with_role()`. The alternative `load_for_role_with_mode(role, ContractLoadMode::RestrictedFallback)` produces a deny-all contract and logs a warning, but it is not used in this code path.

**Impact:** A typo in role name or missing JSON asset silently removes ALL contract enforcement. `ContractLoadMode::Strict` and `RestrictedFallback` exist as alternatives but the `with_role()` path uses permissive fallback.

---

## 2. SafetyLayer: Pre/Post Execution Checks

### Pre-Execution (8 direct checks in `check_pre_execution()` + contract run separately)

`check_pre_execution()` in `safety/mod.rs:324` runs these in order:

| Check | Policy | Wired |
|---|---|---|
| Role tool whitelist | `role_tools` HashMap (from roko.toml) | Yes |
| Rate limit | `RateLimiter` (per tool/role) | Yes |
| OCaps warrant | `AgentWarrant` | Yes (if `warrant` is set) |
| Bash + Git command rules | `BashPolicy` + `GitPolicy` | Yes |
| Network destination check | `NetworkPolicy` | Yes |
| Path escape prevention | `PathPolicy` | Yes |
| Safety budget consumption | `SafetyBudgetTracker` | Optional (None by default) |
| Temporal logic monitor | `TemporalMonitor` | Yes (if attached) |
| Contract invariants/governance | `AgentContract::check_pre_execution` | Yes — called at end of `check_pre_execution()` AND again via `check_contract()` in dispatcher |

Note: The dispatcher calls `safety.check_pre_execution()` then `safety.check_contract()` separately (dispatcher lines ~332 and ~346), but `check_pre_execution()` itself already calls `self.contract.check_pre_execution()` — this means contract checks run twice. `check_contract()` is a thin wrapper around the same call.

### Post-Execution (3 checks in `post_dispatch_check()`)

| Check | Wired | Notes |
|---|---|---|
| Secret scrubbing | Yes | 14 Pattern entries / 9 categories (Anthropic, OpenAI, AWS×2, GitHub×5, GitLab, Slack, JWT, private key blocks, env vars) |
| Path escape in changed files | Yes | Warns if files contain `..` or start with `/` |
| Governance rule violations | Partial | Only checks forbidden file-write tools, not all rules |

### Post-Execution Gaps

`post_dispatch_check()` does **not** validate:
- Tool calls exceeding MaxToolCallsPerTurn
- Cumulative per-turn spend exceeding MaxCostPerTurn
- Consecutive failures triggering recovery rules
- Gate approval for commits (only at pre-execution)

### Output Scrubbing (`scrub.rs`)

14 `Pattern` entries across 9 numbered categories:
1. Anthropic keys (`sk-ant-api\d{2}-...`)
2. OpenAI keys (`sk-proj-...`, `sk-...`) — single pattern covering both prefixes
3. AWS keys: 3a. AKIA (16 chars), 3b. ASIA (STS temporary, 16 chars) — 2 separate patterns
4. GitHub PATs: 4a. `ghp_`, 4b. `ghs_`, 4c. `gho_`, 4d. `ghu_`, 4e. `ghr_` — 5 separate patterns
5. GitLab PATs (`glpat-...`)
6. Slack tokens (`xox[abpsr]-...`)
7. JWTs (`eyJ...eyJ...`) — three base64url segments
8. Private key blocks (RSA, EC, DSA, OpenSSH, PGP — multiline)
9. Env-file assignments: PASSWORD, SECRET, TOKEN, API_KEY, APIKEY, PRIVATE_KEY, DATABASE_URL (case-insensitive, value-only replacement)

The env pattern also covers `APIKEY` and `PRIVATE_KEY` in addition to the list above. The scrubber replaces only the value part of env assignments (preserves `KEY=[REDACTED]` format).

---

## 3. Provider Adapters (6 ProviderKind backends + secondary agents)

`ProviderKind` enum in `roko-core/src/agent.rs` has 6 variants. All 6 implement `ProviderAdapter` trait and are registered in `adapter_for_kind()`:

| Backend | ProviderKind | Type | Status | Notes |
|---|---|---|---|---|
| Claude CLI | `ClaudeCli` | Process | Active | Primary — MCP config passed via `with_mcp_config()` |
| Anthropic API | `AnthropicApi` | HTTP | Active | Streaming, session management |
| OpenAI-compatible | `OpenAiCompat` | HTTP | Active | GPT-4, Llama, Codex (`codex` CLI routes here), OpenRouter |
| Cursor ACP | `CursorAcp` | HTTP | Active | Cursor Agent Client Protocol (ACP over HTTP `/v1/prompt`) |
| Gemini | `GeminiApi` | HTTP | Active | Google Gemini API |
| Perplexity | `PerplexityApi` | HTTP | Active | Sonar API, search-grounded responses |

**Secondary agents (outside ProviderKind adapter registry):**
- `OllamaAgent` / `OllamaLlmBackend` — direct Ollama `/api/chat` adapter; implemented in `ollama/agent.rs`, not a `ProviderKind` variant
- `ExecAgent` — process fallback for unknown CLI commands (used when `provider_kind_for_known_protocol_command` returns `None`); not a `ProviderAdapter`
- `CodexAgent` — the `codex` CLI routes to `ProviderKind::OpenAiCompat` via `provider_kind_for_known_protocol_command()`

### MCP Integration

- `AgentOptions.mcp_config: Option<PathBuf>` → passed to each adapter
- MCP discovery: `find_mcp_config()` walks up from cwd to home
- Tool merging: `DynamicToolRegistry` merges static + MCP tools
- **Silent failure:** Missing `.mcp.json` → no warning, agents continue without MCP tools

---

## 4. ToolDispatcher Pipeline

The dispatcher doc comment lists 6 steps but the actual `dispatch()` method has more stages. Actual pipeline as implemented:

```
1. Validate args vs JSON schema (§36.42)
2. Resolve ToolDef from registry
2b. Profile-based tool selector check (TOOL-03) — optional ToolSelector
3. Apply task-level tool filters (allowed_tools, denied_tools from ToolContext)
4. Authorize role capabilities (def.permission.satisfied_by(&role_perms))
3b. SafetyLayer::check_pre_execution() [role whitelist, rate limit, warrant, bash/git/network/path, budget, temporal, contract]
    + SafetyLayer::check_contract() [duplicate contract check — both call contract.check_pre_execution()]
3c. Safety hook chain (optional SafetyHookChain, TOOL-02)
4. Resolve handler via HandlerResolver
5. Race handler.execute against timeout + cancellation (tokio::select!)
6. Truncate oversized output to DEFAULT_MAX_RESULT_BYTES (16_384 bytes)
7. Scrub secrets from output (SafetyLayer::scrub_output)
8. Check recovery rules (SafetyLayer::check_recovery — calls applicable_recovery())
```

Note: Step numbering in the source comments is non-sequential (3, 3b, 3c, 4) because the steps were added incrementally. The doc header only lists steps 1-6.

**Features:**
- Batch dispatch with concurrency policy (parallel `join_all` or serial, via `partition_by_concurrency`)
- Timeout + cancellation via `tokio::select!`
- Result truncation at `DEFAULT_MAX_RESULT_BYTES` (16,384 bytes = 16KB)
- Tool result caching via optional `ToolResultCache` (AGT-10)
- Audit emission (Engram signals) at each stage
- Hook chain for extensible pre-execution policies (TOOL-02)

**Single unified dispatch path** — no bypass routes. `safety: Option<SafetyLayer>` means safety is skippable at construction time (anti-pattern — see Section 8).

---

## 5. ToolLoop (Non-Claude Backends)

For backends without built-in tool support (OpenAI, Ollama, Gemini):

```
1. Render tools via Translator
2. Send (messages + tools) to backend
3. Parse response for tool calls
4. Dispatch via ToolDispatcher
5. Collect results
6. Inject back into conversation
7. Repeat until Stop or MaxIterations (25)
```

**Key modules** (`roko-agent/src/tool_loop/`):
- `agent_wrapper.rs` — wraps backend with manual loop
- `prune.rs` — context growth guard (120K token limit)
- `compaction.rs` — result truncation before re-injection
- `checkpoint.rs` — resumable state snapshots
- `max_iter.rs` — iteration limit tracking
- `result_msg.rs` — result message formatting
- `backends/` — backend-specific loop implementations

**Known issues:**
- Not resume-compatible: crash mid-tool-loop re-executes side-effecting tools
- No adaptive iteration limits per task
- Result compaction may lose needed context

---

## 6. ProcessSupervisor & Lifecycle

The process management code lives in `roko-agent/src/process/` with modules: `registry.rs`, `group.rs`, `kill.rs`, `mcp.rs`, `env.rs`, `stderr.rs`.

### Process Registry

- Global `OnceLock<Mutex<HashSet<u32>>>` tracking spawned PIDs
- Persists to `.roko/runtime/agent-pids.json` on every mutation
- Startup: `cleanup_orphaned_agents()` sends SIGTERM → SIGKILL

### Process Group Management

- Collects all descendant PIDs via system calls
- Kills entire process tree (agent + shell + subprocesses)

### Known Issues

- **Race condition:** If Roko exits before `persist_pids()`, orphans not recorded
- **No graceful shutdown:** SIGKILL immediately, no cleanup opportunity

---

## 7. Orchestrator Integration

### Pre-Dispatch (orchestrate.rs:14992)

```rust
self.safety_layer.pre_dispatch_check(plan_id, task, role, exec_dir)
```

Validates: execution directory not path-traversal, contract token budget non-zero, safety budget not exhausted.

### Post-Dispatch (orchestrate.rs:15444)

```rust
self.safety_layer.post_dispatch_check(plan_id, task, role, output, changed_files)
```

Validates: no secrets in output, no path escapes in files, no forbidden writes. **Violations are warnings only — don't block.**

---

## 8. Anti-Patterns

| Anti-Pattern | Where | Severity |
|---|---|---|
| **Contract fail-open** | `safety/mod.rs:864-871` — `contract_for_role()` uses `unwrap_or_else(permissive)` | Critical |
| **Optional safety layer in dispatcher** | `dispatcher/mod.rs:89` — `safety: Option<SafetyLayer>` | High |
| **`dangerously_skip_permissions` flag** | `claude_cli_agent.rs:128` — defaults to `true`; `provider/mod.rs:437` exposes it on `AgentOptions` | High |
| **Recovery actions not invoked at orchestrator level** | `check_recovery()` is called per-tool-call in dispatcher (line ~457); not called for orchestrator-level task failures | Medium |
| **Optional safety budget** | `SafetyLayer::with_defaults()` sets `safety_budget: None` | Medium |
| **Cumulative per-turn spend not enforced** | `GovernanceRule::MaxCostPerTurn` checks single-call estimate only; TODO(UX26) in `contract.rs:447` for cumulative tracking | Medium |
| **Rate limiter no per-task reset** | Global per (role, tool), no task scoping | Medium |
| **Post-execution checks incomplete** | `post_dispatch_check()` missing tool call count and cumulative spend validation | Medium |
| **Duplicate contract check** | Dispatcher calls `check_pre_execution()` (which calls contract) then `check_contract()` (same call again) | Low |
| **MCP config silent failure** | Missing `.mcp.json` → no warning | Low |

---

## 9. What's Wired vs What's Not

| Component | Wired | Built | Gap |
|---|---|---|---|
| AgentContract loading | Partial | Yes | `contract_for_role()` fails open on missing JSON asset |
| Pre-execution checks (8 direct + contract) | Yes | Yes | Contract checked twice (check_pre_execution + check_contract) |
| Post-execution checks (3) | Partial | Yes | Missing tool call count and cumulative spend validation |
| ToolDispatcher pipeline | Yes | Yes | Safety is optional (`Option<SafetyLayer>`) |
| Rate limiter | Yes | Yes | No per-task scope |
| Safety budget tracker | Optional | Yes | Not instantiated by default in `with_defaults()` |
| Contract recovery actions (per-tool) | Yes | Yes | Invoked in dispatcher; NOT invoked for orchestrator-level task failures |
| MCP integration | Yes | Yes | Silent failure mode |
| ToolLoop | Yes | Yes | Not resume-safe |
| ProcessSupervisor | Yes | Yes | Race condition on exit |
| Output scrubbing (14 patterns) | Yes | Yes | — |
| Hook chain (SafetyHookChain) | Optional | Yes | Not attached by default |
| Tool selector (ToolSelector) | Optional | Yes | Not attached by default |

---

## 10. File Inventory

### Core Safety
| File | Status | Notes |
|---|---|---|
| `roko-agent/src/safety/mod.rs` | Main safety layer | SafetyLayer struct, check_pre_execution, pre/post_dispatch_check |
| `roko-agent/src/safety/contract.rs` | Contract loading, invariants, governance | AgentContract, Invariant, GovernanceRule, RecoveryKind |
| `roko-agent/src/safety/contracts/*.yaml` | 8 role contracts (JSON format) | architect, auditor, auto-fixer, implementer, researcher, reviewer, scribe, strategist |

### Policy Modules (`safety/`)
| File | Status | Notes |
|---|---|---|
| `bash.rs` | Bash allowlist/denylist | |
| `git.rs` | Branch protection, force-push prevention | |
| `network.rs` | URL allowlist | |
| `path.rs` | Path escape prevention | |
| `scrub.rs` | Secret scrubbing | 14 Pattern entries across 9 categories |
| `rate_limit.rs` | Per-tool rate limiting | |
| `allowlist.rs` | AllowlistGuard | |
| `authz.rs` | Authorization decisions (AuthzDecision, AuthzChannel) | |
| `capabilities.rs` | OCaps warrant, capability types | |
| `data_llm.rs` | Data LLM routing + sanitization | |
| `hallucination.rs` | HallucinationDetector | |
| `hooks.rs` | SafetyHook trait, HookDecision | |
| `provenance.rs` | Taint, Custody, CustodyLogger | |
| `result_filter.rs` | ResultFilter | |
| `risk.rs` | SafetyBudget, SafetyBudgetTracker, BetaDistribution | |
| `spending.rs` | SpendingLimiter, ToolCostEstimate | |
| `temporal.rs` | TemporalMonitor, LtlProperty | |
| `witness.rs` | WitnessDag, WitnessLogger | |

### Dispatcher (`dispatcher/`)
| File | Status | Notes |
|---|---|---|
| `mod.rs` | Tool dispatch pipeline | ToolDispatcher, HandlerResolver, DEFAULT_MAX_RESULT_BYTES=16384 |
| `alert.rs` | Alert dispatch | |
| `cancel.rs` | Cancellation wait | |
| `dedup_cache.rs` | Dispatch-level dedup (DEPLOY-09) | |
| `emit_metric.rs` | Metric emission | |
| `hook_chain.rs` | SafetyHookChain (TOOL-02) | |
| `parallel.rs` | Concurrency partitioning | |
| `result_cache.rs` | Tool result caching (AGT-10) | |
| `timeout.rs` | Timeout wrapper | |
| `tool_selector.rs` | Profile-based tool selector (TOOL-03) | |
| `truncate.rs` | Result truncation | |
| `validate.rs` | Arg schema validation | |

### Provider (`provider/`)
| File | Status | Notes |
|---|---|---|
| `mod.rs` | Adapter pattern, agent factory, ProviderKind routing | |
| `claude_cli.rs` / `claude_cli/` | Claude CLI adapter | |
| `anthropic_api.rs` / `anthropic_api/` | Anthropic API adapter | |
| `cursor_acp.rs` | Cursor ACP adapter | |
| `openai_compat.rs` | OpenAI-compat adapter | |
| `openrouter_meta.rs` | OpenRouter model catalog helper | |

### ToolLoop (`tool_loop/`)
| File | Status | Notes |
|---|---|---|
| `agent_wrapper.rs` | Manual tool loop | |
| `prune.rs` | Context growth guard (120K token limit) | |
| `compaction.rs` | Result truncation before re-injection | |
| `checkpoint.rs` | Resumable state snapshots | |
| `max_iter.rs` | Iteration limit tracking | |
| `result_msg.rs` | Result message formatting | |
| `backends/` | Backend-specific loop implementations | |

### Process Management (`process/`)
| File | Status | Notes |
|---|---|---|
| `registry.rs` | PID tracking + cleanup | |
| `group.rs` | Process tree management | |
| `kill.rs` | Kill helpers | |
| `mcp.rs` | MCP process management | |
| `env.rs` | Environment setup | |
| `stderr.rs` | Stderr capture | |

---

## Sources

Key source files verified for this audit:

- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/safety/mod.rs` — SafetyLayer, check_pre_execution, pre/post_dispatch_check, contract_for_role
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/safety/contract.rs` — AgentContract, Invariant, GovernanceRule, RecoveryKind, ContractLoadMode, load_for_role_with_mode
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/safety/scrub.rs` — default_patterns (14 Pattern entries)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/safety/contracts/architect.yaml` — architect contract (JSON format)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/safety/contracts/implementer.yaml` — implementer contract
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/safety/contracts/reviewer.yaml` — reviewer contract
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/safety/contracts/auto-fixer.yaml` — auto-fixer contract
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/dispatcher/mod.rs` — ToolDispatcher, dispatch(), dispatch_batch()
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/provider/mod.rs` — ProviderAdapter, adapter_for_kind, AgentOptions
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/cursor_agent.rs` — Cursor ACP (Agent Client Protocol) definition
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/claude_cli_agent.rs` — dangerously_skip_permissions default
- `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/agent.rs` — ProviderKind enum (6 variants)
