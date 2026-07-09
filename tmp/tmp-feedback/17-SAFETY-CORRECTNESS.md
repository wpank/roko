# Safety & Correctness Architecture

## Current Safety Posture

### Protections That Exist

Roko has a layered safety architecture centered on `SafetyLayer` in
`crates/roko-agent/src/safety/mod.rs`. The dispatcher chains 8 pre-execution
checks in order; the first failure short-circuits.

| Protection | Module | Status |
|---|---|---|
| Bash command denylist (rm -rf, sudo, curl\|sh, fork bomb, mkfs, dd, chmod 777) | `safety/bash.rs` | **Wired** -- `check_command_with_policy()` called at `mod.rs:406` |
| Git branch protection (force push, hard reset, branch delete on main/master) | `safety/git.rs` | **Wired** -- `check_git_command_with_policy()` called at `mod.rs:407` |
| Network destination allowlist (HTTPS-only, SSRF/RFC1918 blocking, deny/allow hosts) | `safety/network.rs` | **Wired** -- `check_url_with_policy()` called at `mod.rs:414` |
| Path confinement (worktree escape prevention, symlink detection) | `safety/path.rs` | **Wired** -- `canonicalize_with_policy()` called at `mod.rs:429` |
| Secret scrubbing on outputs (API keys, JWTs, PEM blocks, env-file values) | `safety/scrub.rs` | **Wired** -- `scrub_secrets()` called post-dispatch at `orchestrate.rs:16686` |
| Per-tool per-role rate limiting (sliding window, 60/60s default) | `safety/rate_limit.rs` | **Wired** -- `check_and_record()` called at `mod.rs:388` |
| OCaps-style capability warrants (tool/path/exec/network capabilities) | `safety/capabilities.rs` | **Wired** -- `check_capability()` called at `mod.rs:394` |
| Plugin trust tiers (5-tier: Untrusted/Sandboxed/Standard/Trusted/Kernel) | `safety/capabilities.rs` | **Built** -- `check_plugin_tier()` exported, not yet called from MCP bridge |
| Declarative agent contracts (per-role YAML, invariants + governance rules) | `safety/contract.rs` | **Wired** -- `check_pre_execution()` called at `mod.rs:453-455` |
| Temporal logic monitor (Never/Always/Eventually property checking) | `safety/temporal.rs` | **Wired** -- `check_as_tool_error()` called at `mod.rs:449` |
| Adaptive-risk safety budget (irreversibility, blast radius, footprint, cost) | `safety/risk.rs` | **Wired** -- `check_and_consume()` called at `mod.rs:435` |
| Taint-aware authorization (ExternalFetch/ThirdPartyPlugin escalation) | `safety/hooks.rs` | **Wired** -- `authorize_call_with_taint()` at `mod.rs:476-543` |
| Spending limiter (daily/lifetime budget enforcement) | `safety/spending.rs` | **Wired** -- `SafetyHook` trait for hook chains |
| Data provenance tracking | `safety/provenance.rs` | **Built** -- append-only JSONL audit log |
| Witness records | `safety/witness.rs` | **Built** -- append-only JSONL witness log |
| Result hallucination filter | `safety/hallucination.rs` | **Built** |
| Data-to-LLM flow control | `safety/data_llm.rs` | **Built** |
| Allowlist guard | `safety/allowlist.rs` | **Built** |
| Atomic writes (write-to-tmp + rename) | `roko-core/src/io.rs` | **Wired** -- used for state-critical writes |
| `read_optional()` TOCTOU-safe read | `roko-core/src/io.rs:61` | **Built** -- available but not used at all TOCTOU sites |
| MCP transport timeouts (5s write, 30s response) | `roko-agent/src/mcp/client.rs` | **Wired** |
| Bounded LLM streaming channels | `roko-agent/src/dispatch_v2.rs` | **Wired** |

### Protections That Are Missing

1. **Plugin trust tier is not enforced at the MCP bridge** -- `check_plugin_tier()` exists but is never called from the MCP tool call path.
2. **`read_optional()` is available but TOCTOU patterns remain** -- 10+ check-then-act sites still use `.exists()` before I/O.
3. **SIGTERM handler missing from `roko dev`** -- only SIGINT handled; containers send SIGTERM.
4. **Non-atomic JSONL appends** -- 80+ `OpenOptions::append(true)` sites across the codebase risk partial lines on crash.
5. **No filesystem-level sandboxing** -- path confinement is advisory (canonicalize-and-reject), not kernel-enforced. No use of namespaces, seccomp, or pledge.
6. **Contract fallback is permissive** -- `AgentContract::permissive("default")` at `mod.rs:254` means unknown roles get full access.
7. **No audit log wiring** -- provenance and witness modules write to JSONL but the main dispatch path does not call them.

---

## 1. TOCTOU Fixes (Task 047)

### What Is a TOCTOU Bug Here

Every `path.exists()` / `path.is_dir()` / `path.is_file()` followed by a
separate I/O operation on the same path is a time-of-check-to-time-of-use race.
Between the check and the use, the file can be created, deleted, or replaced.

The `roko-core/src/io.rs` module already provides the correct pattern:

```rust
// roko_core::io::read_optional (io.rs:61-67)
pub fn read_optional(path: &Path) -> io::Result<Option<String>> {
    match std::fs::read_to_string(path) {
        Ok(s) => Ok(Some(s)),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}
```

And its async variant `read_optional_async` (io.rs:70-76). These exist but are
not used at the 10 identified TOCTOU sites.

### Every TOCTOU Pattern Found

All paths below are under `crates/roko-cli/src/`.

| # | File:Line | Pattern | Actual Fix Status | Risk |
|---|---|---|---|---|
| 1 | `runner/plan_loader.rs:33` | `if !tasks_path.exists()` then `read_to_string` | **UNFIXED** -- check is still present | Plan load fails with wrong error type; should be `read_optional` returning `Ok(None)` |
| 2 | `runner/plan_loader.rs:77` | `dir.join("tasks.toml").exists()` before `load_plan()` | **UNFIXED** | Race: plan directory deleted between check and `load_plan()` call |
| 3 | `runner/plan_loader.rs:89` | `path.is_dir() && path.join("tasks.toml").exists()` | **UNFIXED** -- double TOCTOU | Two separate stat calls on same path; directory can be removed between them |
| 4 | `runner/plan_loader.rs:143` | `if path.exists()` before `read_to_string` | **UNFIXED** | PRD excerpt file disappears between check and read; loss of context |
| 5 | `runner/plan_loader.rs:234` | `if crate_dir.exists()` | **UNFIXED** | Crate directory check for scaffold skip decision |
| 6 | `runner/plan_loader.rs:334` | `if !ws_cargo_path.exists()` | **UNFIXED** | Workspace Cargo.toml detection race |
| 7 | `runner/event_loop.rs:2622-2627` | `orchestrator_json.exists()` and `!path.exists() && !path.exists()` | **UNFIXED** -- both paths checked with `.exists()` | Resume state detection: if both paths disappear between check and read, the error is misclassified as "no snapshot" instead of "I/O error" |
| 8 | `runner/event_loop.rs:2762` | `if !paths.orchestrator_json.exists()` | **UNFIXED** | Checkpoint misdiagnosed as corrupt when file is merely missing |
| 9 | `runner/event_loop.rs:4080` | `if !episodes_path.exists()` | **UNFIXED** | Episode compaction skipped or panics if file disappears mid-check |
| 10 | `runner/event_loop.rs:4270` | `if pb_dir.exists()` | **UNFIXED** | Playbook seeding race: directory appears between check and create |
| 11 | `runner/extension_loader.rs:128` | `if !dir.exists()` | **UNFIXED** | Plugin discovery: directory created after check returns false |

Additional TOCTOU patterns found outside the runner (lower risk but worth tracking):

| # | File:Line | Pattern | Risk |
|---|---|---|---|
| 12 | `subscriptions.rs:84` | `if !path.exists()` | Subscription file race |
| 13 | `subscriptions.rs:102` | `if !path.exists()` | Same |
| 14 | `credentials.rs:76` | `if path.exists()` before read | Credential store race |
| 15 | `credentials.rs:128` | `if !path.exists()` | Same |
| 16 | `credentials.rs:143` | `if path.exists()` | Same |
| 17 | `dispatch_v2.rs:486` | `if !self.workdir.exists()` | Workdir check race |
| 18 | `dispatch_v2.rs:1050` | `if !self.workdir.exists()` | Same |
| 19 | `daemon.rs:574` | `if plist_path.exists()` | Launchd plist race |
| 20 | `daemon.rs:775` | `if !path.exists()` | PID file race |
| 21 | `daemon.rs:1035` | `if socket_path.exists()` | Unix socket race |
| 22 | `unified.rs:241` | `if !roko_dir.exists()` | Init directory race |
| 23 | `gate_runner.rs:108` | `if !full_path.exists()` | Gate target file race |

### Fix Design for Each Category

**Category A: Read-after-check (patterns 1, 4, 7, 8, 9, 14, 15, 16)**

Replace with `roko_core::io::read_optional()`:

```rust
// BEFORE (plan_loader.rs:33):
if !tasks_path.exists() {
    return Err(anyhow!("no tasks.toml found"));
}
let content = fs::read_to_string(&tasks_path)?;

// AFTER:
let content = match roko_core::io::read_optional(&tasks_path)? {
    Some(c) => c,
    None => return Err(anyhow!("no tasks.toml found")),
};
```

**Category B: Directory-existence-before-traverse (patterns 2, 3, 10, 11, 12, 13, 22)**

Replace with direct operation and `NotFound` matching:

```rust
// BEFORE (plan_loader.rs:89):
if path.is_dir() && path.join("tasks.toml").exists() {
    plans.push(load_plan(&path)?);
}

// AFTER:
match load_plan(&path) {
    Ok(plan) => plans.push(plan),
    Err(e) if is_not_found(&e) => { /* skip: plan dir removed */ },
    Err(e) => return Err(e),
}
```

**Category C: Existence-guard-before-create (patterns 5, 6, 19, 20, 21, 23)**

Replace with `create_dir_all` (idempotent) or `OpenOptions::create_new(true)`:

```rust
// BEFORE (daemon.rs:1035):
if socket_path.exists() {
    fs::remove_file(&socket_path)?;
}

// AFTER:
match fs::remove_file(&socket_path) {
    Ok(()) => {},
    Err(e) if e.kind() == io::ErrorKind::NotFound => {},
    Err(e) => return Err(e.into()),
}
```

---

## 2. Atomic Writes (Tasks 046, 052)

### Atomic Write Infrastructure

Two implementations exist, both using the write-to-tmp-then-rename pattern:

**`roko-core/src/io.rs`** (lines 15-49):
- `atomic_write(path, data)` -- sync, uses `fs::write` + `fs::rename`
- `atomic_write_async(path, data)` -- async, uses `tokio::fs::write` + `tokio::fs::rename`
- `atomic_write_str(path, data)` -- sync string wrapper
- `atomic_write_str_async(path, data)` -- async string wrapper
- Temp file naming: `<path>.tmp.<pid>.<counter>` (process-global `AtomicU32` counter prevents collisions)
- Parent directory creation: automatic via `create_dir_all`
- Cleanup on failure: `remove_file` in `inspect_err`

**`roko-fs/src/atomic.rs`** (lines 29-80):
- `atomic_write_json<T: Serialize>(path, value)` -- serialize + atomic write
- `atomic_write_bytes(path, data)` -- same pattern but in `roko-fs` crate
- Has its own temp naming: `<path>.tmp.<pid>.<counter>`

### Sites Using Atomic Writes (Correctly Protected)

| Site | File:Line | Function |
|---|---|---|
| PRD task generation | `roko-cli/src/prd.rs:355` | `atomic_write_str(&tasks_path, &rendered)` |
| PRD promote | `roko-cli/src/prd.rs:823` | `atomic_write_str(&dst, &content)` |
| PRD plan tasks.toml | `roko-cli/src/prd.rs:1296` | `atomic_write_str(&plan_dir.join("tasks.toml"), ...)` |
| PRD plan.md | `roko-cli/src/prd.rs:1307,1319` | `atomic_write_str(&plan_dir.join("plan.md"), ...)` |
| PRD update | `roko-cli/src/prd.rs:1889` | `atomic_write_str(prd_path, &updated)` |
| Cascade router | `roko-learn/src/cascade_router.rs:1675` | `atomic_write_str(path, &json)` |
| Executor snapshot | `roko-cli/src/runner/persist.rs:284` | `atomic_write(&paths.executor_json, ...)` |
| Orchestrator snapshot | `roko-cli/src/runner/persist.rs:295` | `atomic_write(&paths.orchestrator_json, ...)` |
| Agent PIDs | `roko-cli/src/runner/persist.rs:301` | `atomic_write(&paths.agent_pids_json, ...)` |
| Run state | `roko-cli/src/runner/persist.rs:307` | `atomic_write(&paths.run_state_json, ...)` |
| Checkpoint | `roko-cli/src/runner/persist.rs:538` | `atomic_write(&checkpoint_path, ...)` |
| Snapshot writer | `roko-cli/src/runner/snapshot_writer.rs:183-185` | 3x `atomic_write(...)` |
| PID file | `roko-cli/src/commands/dev.rs:140` | `atomic_write(&pid_path, ...)` |
| Secret store | `roko-core/src/secrets/file.rs:128` | `write_atomic_restricted(&self.path, ...)` |
| Config secrets | `roko-cli/src/config_cmd.rs:505` | `write_atomic_restricted(path, ...)` |
| Conductor snapshot | `roko-learn/src/conductor.rs:194` | `atomic_write_json(path, &snapshot)` |
| Error pattern store | `roko-learn/src/error_pattern_store.rs:267` | `atomic_write_json(path, self)` |
| Provider health | `roko-learn/src/provider_health.rs:406` | `atomic_write_json(path, snapshot)` |
| Latency store | `roko-learn/src/latency.rs:410` | `atomic_write_json(path, snapshot)` |
| Section effect | `roko-learn/src/section_effect.rs:146` | `atomic_write_json(path, &snapshot)` |
| Serve state | `roko-serve/src/state.rs:905` | `atomic_write_async(&path, ...)` |
| Demo seed | `roko-cli/src/demo_seed.rs:1232,1259,1279` | `atomic_write_bytes/atomic_write_json` |
| Knowledge store rewrite | `roko-neuro/src/knowledge_store.rs:1660` | Custom write-tmp-then-rename (implements atomic pattern manually with `sync_all()`) |

### Sites Using Non-Atomic Writes (Risk Assessment)

**HIGH RISK -- state-critical files:**

| Site | File:Line | Write Pattern | Risk |
|---|---|---|---|
| Process registry | `roko-agent/src/process/registry.rs:44` | `std::fs::write(&path, ...)` | Agent PID registry corrupt on crash; agents become untrackable |
| Index graph | `roko-index/src/graph.rs:250` | `std::fs::write(path, &bytes)` | Code intelligence index corrupted; requires full rebuild |
| Neuro context store | `roko-neuro/src/context.rs:249` | `std::fs::write(path, json)` | Context snapshot partial; agent loses accumulated context |
| File cache | `roko-agent/src/file_cache.rs:65` | `std::fs::write(path, json)` | Cache corruption; benign (rebuilt on next use) |

**MEDIUM RISK -- JSONL append-per-entry (80+ sites):**

All JSONL append sites share this pattern:
```rust
OpenOptions::new().create(true).append(true).open(&path)?;
file.write_all(line.as_bytes())?;
file.write_all(b"\n")?;
```

Key JSONL files at risk:

| File | JSONL Path | Impact of Partial Line |
|---|---|---|
| Episodes | `.roko/episodes.jsonl` | Episode log truncated; learning data lost for last entry |
| Efficiency | `.roko/learn/efficiency.jsonl` | Efficiency metrics truncated |
| Signals | `.roko/signals.jsonl` | Signal chain broken at last entry |
| Run ledger | `.roko/state/runs.jsonl` | Run audit trail incomplete |
| Provenance | provenance audit log | Safety audit record lost |
| Witness | witness log | Witness record lost |
| C-factor | `.roko/learn/c-factor.jsonl` | Metrics partial |
| Costs log | `.roko/learn/costs.jsonl` | Cost tracking partial |
| Routing log | `.roko/learn/routing.jsonl` | Routing decisions lost |
| Gateway events | gateway event log | Event lost |

**LOW RISK -- test-only or ephemeral:**

| Site | File:Line | Notes |
|---|---|---|
| MCP scripts test | `roko-mcp-scripts/src/main.rs:629` | Test scaffold only |
| Claude CLI agent tests | `roko-agent/src/claude_cli_agent.rs:1044+` | Test script creation |
| Bench routes | `roko-serve/src/routes/bench.rs:826` | Benchmark data |
| Serve bench | `roko-serve/src/bench.rs:498,640` | Bench harness |
| Tier progression reports | `roko-neuro/src/tier_progression.rs:1036` | Markdown reports |

### Migration Plan

**Phase 1: State-critical files (immediate)**

Replace `std::fs::write` with `roko_core::io::atomic_write` at:
- `roko-agent/src/process/registry.rs:44`
- `roko-index/src/graph.rs:250`
- `roko-neuro/src/context.rs:249`
- `roko-agent/src/file_cache.rs:65`

**Phase 2: JSONL corruption resilience (short-term)**

Add a truncation recovery header to JSONL parsers. On load, detect and skip
partial trailing lines:

```rust
fn load_jsonl_tolerant<T: DeserializeOwned>(path: &Path) -> io::Result<Vec<T>> {
    let content = match read_optional(path)? {
        Some(c) => c,
        None => return Ok(Vec::new()),
    };
    let mut entries = Vec::new();
    for line in content.lines() {
        if line.trim().is_empty() { continue; }
        match serde_json::from_str::<T>(line) {
            Ok(entry) => entries.push(entry),
            Err(e) => {
                tracing::warn!(line_len = line.len(), %e, "skipping corrupt JSONL line");
            }
        }
    }
    Ok(entries)
}
```

**Phase 3: Buffered JSONL writes (medium-term)**

Replace per-entry `OpenOptions::append` with a buffered writer that:
1. Opens the file once per session
2. Writes entries to an in-memory buffer
3. Flushes at configurable intervals or on explicit sync
4. Uses `BufWriter` with `write_all` + `flush` to minimize partial-line risk

The `roko-runtime/src/jsonl_logger.rs` already has a logger pattern that could
be generalized.

---

## 3. Timeout Architecture (Task 044)

### Timeouts That Exist

| Operation | Timeout | Where |
|---|---|---|
| MCP stdin write | 5s | `roko-agent/src/mcp/client.rs:222` -- `DEFAULT_MCP_STDIN_WRITE_TIMEOUT_SECS` |
| MCP response wait | 30s | `roko-agent/src/mcp/client.rs:243` -- `DEFAULT_MCP_RESPONSE_TIMEOUT_SECS` |
| MCP tool discovery | configurable | `roko-agent/src/mcp/bridge.rs:72` -- `MCP_DISCOVERY_TIMEOUT` |
| MCP scripts execution | 60s default (configurable via `--timeout-secs` or `ROKO_MCP_SCRIPTS_TIMEOUT_SECS`) | `roko-mcp-scripts/src/main.rs:232,527` |
| Plugin manifest timeout | 30000ms default (configurable per-plugin `timeout_ms`) | `roko-plugin/src/manifest.rs:141-142` |
| Agent process termination | configurable (SIGTERM then wait then SIGKILL) | `roko-cli/src/commands/dev.rs:172-180` |
| Agent turn timeout | configurable via `agent.turn_timeout_secs` | roko.toml config |
| Gate rung timeouts | per-rung configurable | gate pipeline config |
| HTTP serve shutdown | ctrl_c await | `roko-cli/src/commands/server.rs:115` |

### Timeouts That Are Missing

| Operation | Current Behavior | Risk |
|---|---|---|
| HTTP client requests (`reqwest`) in agent backends | **No explicit timeout** -- defaults to reqwest's own defaults (which may be infinite for connect) | Agent hangs indefinitely on unresponsive LLM provider |
| `tokio::process::Command` in bash tool handler | **No timeout** -- waits for child process exit | Runaway subprocess blocks the agent turn forever |
| WebSocket connections in `roko-agent-server` | **No ping/pong timeout** -- connection stays open | Dead WebSocket connections leak resources |
| Plan runner `select!` branches | **No per-task timeout** -- waits for agent completion | Single stuck agent blocks entire plan execution |
| Knowledge store `rewrite_all` | **No timeout on fsync** -- `sync_all()` can block | NFS or slow disk blocks the knowledge store indefinitely |
| `roko serve` route handlers | **No per-request timeout** -- axum default | Slow handler blocks a connection slot |

### Timeout Value Analysis

| Timeout | Value | Assessment |
|---|---|---|
| MCP stdin write 5s | **Appropriate** -- stdin should never block this long unless the child is stuck |
| MCP response 30s | **Appropriate** -- MCP tools can involve LLM calls; 30s is generous but not excessive |
| MCP scripts 60s | **Appropriate** -- scripts vary widely; configurable override is good |
| Plugin 30s | **Appropriate** -- standard for plugin execution |

### Missing Timeout Test

The MCP timeout tests are noted as missing:
```
Missing: #[tokio::test(start_paused = true)] test advancing past 30s.
```

A `start_paused` test should verify that the timeout fires correctly when the
MCP server does not respond within 30s, returning a timeout error rather than
hanging forever.

---

## 4. Path Safety (Task 076)

### Current Path Confinement Design

The path safety layer lives at `crates/roko-agent/src/safety/path.rs` and
implements a 5-step algorithm:

1. **Join** -- if `arg_path` is absolute, use as-is; otherwise join with worktree root
2. **Canonicalize** -- resolve both worktree and joined path; for non-existent leaves, canonicalize the deepest existing ancestor and re-attach tail
3. **Escape check** -- when `PathPolicy::prevent_escapes` is true (default), canonical joined must `starts_with` canonical worktree
4. **Symlink check** -- when `PathPolicy::deny_symlinks` is true, walk on-disk components and reject any symlink
5. **Relative computation** -- strip worktree prefix to produce relative form

This is called from `SafetyLayer::check_pre_execution()` (mod.rs:419-431) for
all tools in `FILE_TOOLS`: `read_file`, `write_file`, `edit_file`, `multi_edit`,
`apply_patch`, `notebook_edit`, `ls`, `glob`, `grep`.

### Bash Command Path Confinement

The bash policy has a secondary path confinement layer at
`safety/bash.rs:145-162` -- `check_path_confinement()`:

- When `allowed_path_prefixes` is non-empty, tokens in the command that look
  like absolute paths (start with `/`, no shell metacharacters) must match a
  prefix
- Shell metacharacters (`$`, backtick, `|`, `;`, `&`, `(`, `)`) cause the token
  to be skipped (shell syntax, not a path)
- This is intentionally best-effort; full shell parsing is not attempted

### Where User-Controlled Paths Are Used Without Validation

| Site | Risk | Notes |
|---|---|---|
| `dispatch_v2.rs:486` | **MEDIUM** -- workdir check uses `.exists()` but workdir itself comes from config | Config is trusted; the path confinement check happens in SafetyLayer, not here |
| MCP server tool calls via `roko-agent-server` | **HIGH** -- external HTTP callers can supply arbitrary tool arguments | The sidecar does call `ToolDispatcher` which includes `SafetyLayer`, but the sidecar's own route authentication is minimal |
| Plugin-originated tool calls | **MEDIUM** -- plugins at Sandboxed tier or above can invoke tools | `check_plugin_tier()` exists but is not wired into the MCP bridge dispatch path |
| `roko-serve` route handlers accepting path parameters | **LOW** -- paths are resolved relative to the workspace by convention | No path policy is applied to HTTP route path parameters (e.g., `/plans/{name}`) |

### Sandbox Design for Tool Execution

The current design is **advisory confinement**: the path policy validates paths
before I/O but the process itself has full filesystem access. There is no
kernel-level sandboxing.

**Recommended hardening layers (in order of implementation ease):**

1. **Wire `check_plugin_tier()` into MCP bridge** -- this is the lowest-hanging
   fruit. When a tool call originates from an MCP server, check the server's
   tier against the required capability before dispatching.

2. **`chroot`/`pivot_root` for subprocess execution** -- for the `bash` tool
   handler, optionally execute commands inside a restricted filesystem view.
   Requires Linux namespaces; macOS would need `sandbox-exec` (deprecated) or
   a different approach.

3. **seccomp-bpf profile for agent subprocesses** -- restrict system calls
   available to agent-spawned processes. This prevents the agent from bypassing
   path confinement by opening files directly with raw syscalls.

4. **AppArmor/SELinux profiles** -- for deployment environments, provide
   distribution-ready profiles that confine the `roko` process to its workspace.

---

## 5. Agent Safety Layer

### Current Permission Model

The `SafetyLayer` (mod.rs:184-217) aggregates all policy dimensions into a
single struct:

```
SafetyLayer {
    bash_policy: BashPolicy,          // command denylist + allowlist
    git_policy: GitPolicy,            // branch protection
    network_policy: NetworkPolicy,    // destination allowlist
    path_policy: PathPolicy,          // worktree confinement
    scrub_policy: ScrubPolicy,        // output secret scrubbing
    rate_limiter: Option<RateLimiter>, // per-tool per-role rate limit
    safety_budget: Option<SafetyBudgetTracker>, // adaptive risk budget
    role: String,                     // role label
    contract: AgentContract,          // declarative YAML contract
    warrant: Option<AgentWarrant>,    // OCaps capability token
    role_tools: HashMap<...>,         // per-role tool whitelist
    temporal_monitor: Option<TemporalMonitor>, // LTL property checking
}
```

**Pre-execution check chain** (mod.rs:370-458):
1. Role tool whitelist check
2. Rate limit check-and-record
3. OCaps warrant capability check
4. Bash/git command policy
5. Network URL policy
6. File path confinement policy
7. Safety budget check-and-consume
8. Temporal logic monitor
9. Declarative contract invariants

**Post-execution checks** (mod.rs, orchestrate.rs:16717):
- Secret scrubbing on agent output
- Safety violation detection

**Authorization API** (mod.rs:469-546):
- `authorize_call()` / `authorize_call_with_taint()` return `AuthzDecision`
- Tainted context escalates network/write/bash to `AllowWithConfirm`
- Confirmation channels: `DenyAllChannel` (fail-closed default), `ApproveAllChannel` (test), `LogAndDenyChannel` (daemon mode)

### Gaps in Permission Checking

| Gap | Severity | Details |
|---|---|---|
| **Default contract is permissive** | HIGH | `SafetyLayer::with_defaults()` at mod.rs:254 uses `AgentContract::permissive("default")`. Any role without a YAML contract file gets full tool access. The contract system is fail-open. |
| **Plugin tier not enforced** | HIGH | `PluginTier` and `check_plugin_tier()` are built (capabilities.rs:111-133) but never called from the MCP tool dispatch path. A Sandboxed MCP server can invoke write tools. |
| **MCP tool results not scrubbed** | MEDIUM | Secret scrubbing runs on agent output (orchestrate.rs:16686) but MCP tool results flow directly into the agent context without scrubbing. |
| **Subprocess execution bypasses path policy** | MEDIUM | `check_exec_command()` (mod.rs:562-602) validates bash and git policies but does NOT apply path confinement. A subprocess can read/write any path the process can access. |
| **No warrant expiry enforcement** | LOW | `AgentWarrant` has `expires_at: Option<u64>` (capabilities.rs:161) but `check_capability()` (capabilities.rs:204-209) never checks expiry. Warrants are valid forever once issued. |
| **Provenance/witness not wired into dispatch** | LOW | The provenance and witness modules write audit records but are not called from the main `check_pre_execution` or `check_post_execution` paths. |
| **Bash denylist is substring-based** | LOW | Simple evasion possible: `r\m -rf /` (backslash), `$(echo rm) -rf /` (subshell), base64-encoded commands. The denylist is defense-in-depth, not a security boundary. |
| **`env::set_var` data race in tests** | LOW | `roko-cli/src/main.rs:2458,2462` and `roko-core/src/config/loader.rs:957` use `unsafe { std::env::set_var(...) }`. In multi-threaded test contexts this is a data race. |

### Design for Proper Sandboxing

**Tier 1: Wire what exists (immediate, no new code)**
1. Call `check_plugin_tier()` from the MCP bridge before tool dispatch
2. Check `AgentWarrant::expires_at` in `check_capability()`
3. Change default contract from `permissive` to a minimal contract that only allows read tools

**Tier 2: Harden the dispatch path (short-term)**
1. Apply path confinement to subprocess execution (`check_exec_command`)
2. Scrub MCP tool results before they enter agent context
3. Add provenance records to the dispatch audit trail

**Tier 3: Process isolation (medium-term)**
1. Spawn agent subprocesses in a restricted namespace (Linux: unshare + chroot)
2. Apply a seccomp-bpf filter to agent child processes
3. Use cgroups to limit CPU/memory consumption per agent task

---

## 6. Thread Safety Issues

### `unsafe { std::env::set_var(...) }` Data Races

Rust 1.66+ marks `set_var` as unsafe because it is not thread-safe. The
following sites call it in contexts where other threads may be reading env vars:

| File:Line | Context | Risk |
|---|---|---|
| `roko-cli/src/main.rs:2458` | `ROKO_HIGH_CONTRAST` set in accessibility setup | Low -- happens early in main, before threads spawn |
| `roko-cli/src/main.rs:2462` | `ROKO_REDUCED_MOTION` set in accessibility setup | Low -- same |
| `roko-core/src/config/loader.rs:957` | `ROKO__AGENT__DEFAULT_MODEL` in test | **Medium** -- multi-threaded test runtime; other config loader tests may read env concurrently |

**Fix:** For tests, use `#[serial_test::serial]` or `temp_env` crate. For
main.rs, move the `set_var` calls before any async runtime is started.

### Nested Runtime in `spawn_blocking`

`roko-cli/src/tui/verdicts.rs` creates a `current_thread` runtime inside
`spawn_blocking`. This works but is architecturally wrong: `spawn_blocking` is
for CPU-bound work, not for running async code. The fix is to use
`tokio::spawn` with a proper future instead.

---

## 7. Signal Handling (Task 049)

### Current State

`roko dev` (commands/dev.rs:96-97) uses only `tokio::signal::ctrl_c()`:

```rust
// Line 96-97:
// 7. Signal handling: wait for SIGINT/SIGTERM.
tokio::signal::ctrl_c().await.context("listen for ctrl+c")?;
```

The comment says "SIGINT/SIGTERM" but only SIGINT is actually handled.

`roko serve` (commands/server.rs:115) has the same pattern.

### Impact

Container runtimes (Docker, Railway, Kubernetes) send SIGTERM (signal 15) to
gracefully stop processes. Without a SIGTERM handler:

1. `roko dev` won't clean up its PID file (`.roko/dev.pid`), blocking the next `roko dev` start
2. `roko serve` won't flush pending state to disk
3. Agent subprocesses won't receive graceful shutdown

### Fix

```rust
use tokio::signal::unix::{signal, SignalKind};

let mut sigterm = signal(SignalKind::terminate())
    .context("register SIGTERM handler")?;

tokio::select! {
    _ = tokio::signal::ctrl_c() => { tracing::info!("received SIGINT"); },
    _ = sigterm.recv() => { tracing::info!("received SIGTERM"); },
}
```

---

## 8. Port Binding Race (Task 048)

### Current Pattern

`crates/roko-cli/tests/common/mod.rs:464-469`:

```rust
pub fn pick_unused_port() -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind random port");
    let port = listener.local_addr().expect("listener addr").port();
    drop(listener);  // <-- port released here
    port             // <-- another process can grab it before server.bind()
}
```

Used at:
- `roko-cli/tests/smoke.rs:347`
- `roko-cli/tests/common/mod.rs:379,403`

### Fix

Bind to port 0 and pass the pre-bound listener to the server:

```rust
pub fn bind_unused() -> (std::net::TcpListener, u16) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    (listener, port)
}

// In test:
let (listener, port) = bind_unused();
let server = axum::Server::from_tcp(listener)
    .expect("from_tcp")
    .serve(app.into_make_service());
```

---

## Priority-Ordered Fix List

| Priority | Issue | Severity | Complexity | Files |
|---|---|---|---|---|
| **P0** | Default contract is permissive (fail-open) | HIGH | Low | `safety/mod.rs:254` -- change `permissive` to minimal |
| **P0** | Plugin tier not enforced at MCP bridge | HIGH | Low | Wire `check_plugin_tier()` into MCP handler |
| **P1** | TOCTOU in plan_loader (10 patterns) | HIGH | Medium | `runner/plan_loader.rs`, `runner/event_loop.rs`, `runner/extension_loader.rs` |
| **P1** | Warrant expiry not checked | MEDIUM | Low | `safety/capabilities.rs:204` -- add timestamp check |
| **P1** | SIGTERM handling missing | MEDIUM | Low | `commands/dev.rs`, `commands/server.rs` -- 5 lines each |
| **P2** | Non-atomic state-critical writes | MEDIUM | Low | 4 files: `process/registry.rs`, `graph.rs`, `context.rs`, `file_cache.rs` |
| **P2** | JSONL truncation resilience | MEDIUM | Medium | Add tolerant JSONL parser; all 80+ JSONL reader sites |
| **P2** | Subprocess path confinement missing | MEDIUM | Medium | `safety/mod.rs:562-602` -- add path policy to `check_exec_command` |
| **P2** | MCP tool results not scrubbed | MEDIUM | Low | Add `scrub_secrets()` call after MCP tool execution |
| **P2** | TOCTOU in credentials/daemon/subscriptions (13 patterns) | MEDIUM | Medium | Various `roko-cli/src/` files |
| **P3** | Port race in test harness | LOW | Low | `tests/common/mod.rs` -- 1 function |
| **P3** | `env::set_var` data race in tests | LOW | Low | 3 sites -- add `#[serial_test::serial]` |
| **P3** | Nested runtime in verdicts.rs | LOW | Low | Refactor to `tokio::spawn` |
| **P3** | Provenance/witness not wired into dispatch | LOW | Medium | Add audit record emission to `check_pre/post_execution` |
| **P3** | Missing MCP timeout test | LOW | Low | Add `#[tokio::test(start_paused = true)]` test |

---

## Testing Strategy for Safety Properties

### Unit Tests (per-module, already exist for most)

| Property | Test Approach | Current Coverage |
|---|---|---|
| Bash denylist | Blocked/allowed assertions per pattern | 20+ tests in `safety/bash.rs` |
| Git branch protection | Blocked/allowed per subcommand variant | 21 tests in `safety/git.rs` |
| Network SSRF blocking | Private IP, loopback, deny/allow host tests | 18 tests in `safety/network.rs` |
| Path confinement | Escape, symlink, relative, absolute path tests | 14 tests in `safety/path.rs` |
| Secret scrubbing | Per-secret-type redaction tests | 17 tests in `safety/scrub.rs` |
| Rate limiting | Cap boundary, expiry, concurrency stress test (20 threads x 10 calls) | 13 tests in `safety/rate_limit.rs` |
| Capabilities | Warrant check, delegation, tier checks | 11 tests in `safety/capabilities.rs` |
| Contracts | Per-role invariant checks | Tests in `safety/contract.rs` |
| Risk budget | Dimension exhaustion, check-and-consume atomicity | 5 tests in `safety/risk.rs` |
| Authorization | Decision resolution with confirmation channels | 10 tests in `safety/authz.rs` |
| Tainted strings | Information flow sink checks, zero-on-drop | 4 tests in `safety/hooks.rs` |

### Integration Tests (need additions)

| Test | Status | What It Should Verify |
|---|---|---|
| SafetyLayer end-to-end | **EXISTS** -- `tests/safety_integration.rs` | Full pre-execution chain with all policies enabled |
| Contract enforcement | **EXISTS** -- `tests/contracts.rs` | Role-specific tool restrictions from YAML contracts |
| TOCTOU regression | **MISSING** | Concurrent file deletion during plan loading; should produce `Ok(None)` not panic |
| Atomic write crash resilience | **MISSING** | Kill process mid-write; verify file is either old content or new content, never partial |
| MCP timeout under load | **MISSING** | `start_paused` test advancing past 30s; verify timeout error returned |
| Plugin tier enforcement | **MISSING** | Sandboxed MCP server attempts write tool; verify rejection |
| Port binding stability | **MISSING** | Pass pre-bound listener to server; verify no EADDRINUSE |

### Property-Based Tests (recommended additions)

1. **Path confinement fuzzing** -- generate random path strings (with `..`, symlinks, unicode) and verify that `canonicalize_with_policy` never returns a path outside the worktree when `prevent_escapes` is true.

2. **Bash denylist fuzzing** -- generate random shell command strings and verify that the denylist never allows a command that contains a known-dangerous substring after shell expansion.

3. **JSONL truncation recovery** -- write N entries, truncate at a random byte offset, verify that the tolerant parser recovers N-1 or N entries (never panics).

4. **Rate limiter fairness** -- run M threads with K different keys and verify that each key admits exactly `cap` calls, no more, no less.
