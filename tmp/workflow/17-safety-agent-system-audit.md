# Safety & Agent System Audit

8 provider backends, layered safety checks, behavioral contracts, tool dispatch pipeline — architecturally sound but critically fails open when contracts are missing.

## The Problem

The safety and agent system is substantially wired: 8 backends functional, ToolDispatcher enforces all pre-execution checks, orchestrator calls pre/post-dispatch checks. The critical issues: contract fail-open behavior (missing YAML → zero restrictions), recovery actions built but never invoked, optional safety budgets, and incomplete post-execution validation.

---

## 1. AgentContract (Behavioral Contracts)

### What Contracts Define

8 bundled YAML contracts in `roko-agent/src/safety/contracts/`:
- architect, auditor, auto-fixer, implementer, researcher, reviewer, scribe, strategist

**Example (architect.yaml):**
```yaml
role: architect
invariants:
  - MaxTokensPerTurn: 16000
governance:
  - MaxToolCallsPerTurn: 6
  - ForbiddenTools: ["edit_file", "write_file", "multi_edit", "apply_patch", "bash"]
  - RequireToolBeforeEdit: "read_file"
recovery:
  - trigger: contract_violation
    action: Alert
```

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
| MaxCostPerTurn(f64) | Built, TODO(UX26) — cumulative spend not enforced |
| MaxConsecutiveFailures(u32) | Wired |
| RequireToolBeforeEdit(String) | Wired |

### Recovery Actions (4 types)

Retry, Downgrade, Abort, Alert — all built, **none invoked at runtime**. `applicable_recovery()` exists but orchestrator never evaluates triggers.

### Critical: Fail-Open Behavior

```rust
fn contract_for_role(&self, role: &str) -> AgentContract {
    AgentContract::load_for_role(role).unwrap_or_else(|err| {
        tracing::warn!("no contract for role; using permissive default");
        AgentContract::permissive(role.to_string())  // ZERO restrictions
    });
}
```

**Impact:** A typo in role name or missing YAML silently removes ALL contract enforcement. `ContractLoadMode::Strict` and `RestrictedFallback` exist as alternatives but aren't used by default.

---

## 2. SafetyLayer: Pre/Post Execution Checks

### Pre-Execution (10 checks in `check_pre_execution()`)

| Check | Policy | Wired |
|---|---|---|
| Role tool whitelist | `role_tools` HashMap | Yes |
| Rate limit | `RateLimiter` (per tool/role) | Yes |
| OCaps warrant | `AgentWarrant` | Yes |
| Bash command rules | `BashPolicy` | Yes |
| Git command rules | `GitPolicy` | Yes |
| Network destination check | `NetworkPolicy` | Yes |
| Path escape prevention | `PathPolicy` | Yes |
| Safety budget consumption | `SafetyBudgetTracker` | Optional (None by default) |
| Temporal logic monitor | `TemporalMonitor` | Yes |
| Contract invariants/governance | `AgentContract` | Yes |

### Post-Execution (3 checks in `post_dispatch_check()`)

| Check | Wired | Notes |
|---|---|---|
| Secret scrubbing | Yes | 13 regex patterns (Anthropic, OpenAI, AWS, GitHub, GitLab, Slack, JWT, SSH keys, env vars) |
| Path escape in changed files | Yes | Warns if files contain `..` or start with `/` |
| Governance rule violations | Partial | Only checks forbidden file-write tools, not all rules |

### Post-Execution Gaps

`post_dispatch_check()` does **not** validate:
- Tool calls exceeding MaxToolCallsPerTurn
- Cumulative per-turn spend exceeding MaxCostPerTurn
- Consecutive failures triggering recovery rules
- Gate approval for commits (only at pre-execution)

### Output Scrubbing (`scrub.rs`)

13 secret patterns detected and redacted:
1. Anthropic keys (`sk-ant-api\d{2}-...`)
2. OpenAI keys (`sk-proj-...`, `sk-...`)
3. AWS keys (AKIA, ASIA)
4. GitHub tokens (ghp_, ghs_, gho_, ghu_, ghr_)
5. GitLab tokens (glpat-...)
6. Slack tokens (xox*)
7. JWTs (eyJ...)
8. Private key blocks (RSA, EC, DSA, OpenSSH, PGP)
9. Env assignments (PASSWORD=, SECRET=, TOKEN=, API_KEY=, DATABASE_URL=)

---

## 3. Provider Adapters (8 Backends)

All implement `ProviderAdapter` trait:

| Backend | Type | Status | Notes |
|---|---|---|---|
| Claude CLI | Process | Active | Primary — MCP config passed via `with_mcp_config()` |
| Claude API | HTTP | Active | Streaming, session management |
| OpenAI-compatible | HTTP | Active | GPT-4, Llama, etc. |
| Cursor ACP | HTTP | Active | Anthropic CLI Protocol |
| Gemini | HTTP | Active | Embedding + content caching |
| Perplexity | HTTP | Active | Search-grounded responses |
| Ollama | HTTP | Active | Local inference |
| ExecAgent | Process | Active | Fallback for unknown CLI commands |

### MCP Integration

- `AgentOptions.mcp_config: Option<PathBuf>` → passed to each adapter
- MCP discovery: `find_mcp_config()` walks up from cwd to home
- Tool merging: `DynamicToolRegistry` merges static + MCP tools
- **Silent failure:** Missing `.mcp.json` → no warning, agents continue without MCP tools

---

## 4. ToolDispatcher (10-Stage Pipeline)

```
1. Validate args vs JSON schema
2. Resolve ToolDef from registry
3. Check profile-based tool selector
4. Apply task-level tool filters (allowed_tools, denied_tools)
5. Check role capability permissions
6. Run SafetyLayer::check_pre_execution() [ALL 10 checks]
7. Run SafetyLayer::check_contract()
8. Run hook chain (optional)
9. Resolve handler + Execute with timeout/cancellation
10. Truncate output + Scrub secrets + Check recovery rules
```

**Features:**
- Batch dispatch with concurrency policy (parallel `join_all` or serial)
- Timeout + cancellation via `tokio::select!`
- Result truncation at `DEFAULT_MAX_RESULT_BYTES` (16KB)
- Tool result caching for dedup
- Audit emission (Engram signals)

**No anti-patterns detected** — single unified dispatch path, safety checks short-circuit, handler resolver is abstract.

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

**Key modules:**
- `agent_wrapper.rs` — wraps backend with manual loop
- `prune.rs` — context growth guard (120K token limit)
- `compaction.rs` — result truncation before re-injection
- `checkpoint.rs` — resumable state snapshots

**Known issues:**
- Not resume-compatible: crash mid-tool-loop re-executes side-effecting tools
- No adaptive iteration limits per task
- Result compaction may lose needed context

---

## 6. ProcessSupervisor & Lifecycle

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
| **Contract fail-open** | `safety/mod.rs:871` — `unwrap_or_else(permissive)` | Critical |
| **Optional safety layer in dispatcher** | `dispatcher/mod.rs:89` — `safety: Option<SafetyLayer>` | High |
| **`dangerously_skip_permissions` flag** | `provider/mod.rs:437` — boolean bypass | High |
| **Recovery actions never invoked** | `contract.rs` — `applicable_recovery()` built, never called | High |
| **Optional safety budget** | `SafetyLayer::with_defaults()` sets `safety_budget: None` | Medium |
| **Per-turn spend not enforced** | TODO(UX26) in contract.rs | Medium |
| **Rate limiter no per-task reset** | Global per (role, tool), no task scoping | Medium |
| **Post-execution checks incomplete** | Missing tool call count and spend validation | Medium |
| **MCP config silent failure** | Missing `.mcp.json` → no warning | Low |

---

## 9. What's Wired vs What's Not

| Component | Wired | Built | Gap |
|---|---|---|---|
| AgentContract loading | Partial | Yes | Fails open on missing YAML |
| Pre-execution checks (10) | Yes | Yes | — |
| Post-execution checks (3) | Partial | Yes | Missing tool counts, spend |
| ToolDispatcher pipeline (10-stage) | Yes | Yes | — |
| Rate limiter | Yes | Yes | No per-task scope |
| Safety budget tracker | Optional | Yes | Not instantiated by default |
| Contract recovery actions | No | Yes | Never invoked |
| MCP integration | Yes | Yes | Silent failure mode |
| ToolLoop | Yes | Yes | Not resume-safe |
| ProcessSupervisor | Yes | Yes | Race condition on exit |
| Output scrubbing | Yes | Yes | — |

---

## 10. File Inventory

### Core Safety (~1,000 LOC)
| File | LOC | Status |
|---|---|---|
| `roko-agent/src/safety/mod.rs` | ~800 | Main safety layer |
| `roko-agent/src/safety/contract.rs` | ~600 | Contract loading, invariants, governance |
| `roko-agent/src/safety/contracts/*.yaml` | ~111 | 8 role contracts |

### Policy Modules
| File | LOC | Status |
|---|---|---|
| `safety/bash.rs` | ~300 | Bash allowlist/denylist |
| `safety/git.rs` | ~400 | Branch protection, force-push prevention |
| `safety/network.rs` | ~250 | URL allowlist |
| `safety/path.rs` | ~300 | Path escape prevention |
| `safety/scrub.rs` | ~300 | Secret scrubbing (13 patterns) |
| `safety/rate_limit.rs` | ~300 | Per-tool rate limiting |

### Dispatcher & Provider (~2,000 LOC)
| File | LOC | Status |
|---|---|---|
| `dispatcher/mod.rs` | ~800 | 10-stage tool dispatch pipeline |
| `provider/mod.rs` | ~600 | Adapter pattern, agent factory |
| `provider/claude_cli.rs` | ~200 | Claude CLI adapter |
| `provider/anthropic_api.rs` | ~300 | Claude API adapter |
| `provider/*.rs` (6 more) | ~600 | Other backends |

### ToolLoop
| File | LOC | Status |
|---|---|---|
| `tool_loop/agent_wrapper.rs` | ~300 | Manual tool loop |
| `tool_loop/prune.rs` | ~200 | Context growth guard |
| `tool_loop/compaction.rs` | ~200 | Result truncation |

### Process Management
| File | LOC | Status |
|---|---|---|
| `process/registry.rs` | ~200 | PID tracking + cleanup |
| `process/group.rs` | ~150 | Process tree management |
