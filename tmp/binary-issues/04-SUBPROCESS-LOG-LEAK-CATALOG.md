# Subprocess & Log Leak Catalog — 2026-04-27

Every place in the codebase where a child process, background binary, or direct
stderr write could leak output into the user's terminal. Organized by severity.

See also: `03-UNIFIED-CHAT-BLOCKERS.md` for the unified chat-specific issues.

---

## CRITICAL — Separate binaries with own tracing (bypass parent's file redirect)

These are standalone binaries spawned as child processes. They have their own
`tracing_subscriber::fmt().init()` and write directly to inherited stderr.
Redirecting tracing in the parent process has ZERO effect on these.

### C1. roko-chain-watcher

- [x] **Fixed** — stderr now redirected to `.roko/chain-watcher.log`, `ROKO_LOG=warn` passed
- **Spawned at:** `crates/roko-serve/src/lib.rs:267-286`
- **Binary:** `apps/roko-chain-watcher/src/main.rs`
- **Own tracing:** `apps/roko-chain-watcher/src/main.rs:42-49` — defaults to `info,roko_chain_watcher=debug`
- **Output:** INFO/DEBUG pheromone deposits, block observations, chain stats, tick completions
- **Volume:** ~20 lines/second when chain is active

### C2. MCP server subprocesses (explicit `stderr(Stdio::inherit())`)

- [ ] **Needs fix**
- **Spawned at:** `crates/roko-agent/src/mcp/client.rs:182-189`
- **What:** Any MCP server binary (roko-mcp-scripts, roko-mcp-slack, roko-mcp-code, etc.)
- **Code:**
  ```rust
  .stderr(std::process::Stdio::inherit())  // EXPLICITLY INHERITED
  ```
- **Risk:** MCP servers can be noisy — startup messages, tool execution logs, errors all go
  straight to parent terminal. Each MCP server has its own tracing or println! calls.
- **Fix:** Redirect stderr to `.roko/logs/mcp-<name>.log` or pipe and forward to tracing.

### C3. MCP servers that have their own tracing/eprintln

These binaries would inherit stderr if spawned (via C2 above):

- [ ] `crates/roko-mcp-scripts/src/main.rs` — has its own `tracing_subscriber` init
- [ ] `crates/roko-mcp-slack/src/main.rs` — has its own `tracing_subscriber` init
- [ ] `crates/roko-mcp-code/` — MCP code intelligence server

---

## HIGH — Subprocess spawns without explicit stderr handling

These use `Command::new()` without setting `.stderr()`. The `.output()` method
captures by default, but if any of these switch to `.spawn()` + manual read,
stderr would leak.

### H1. Compile gate

- [ ] **Audit needed**
- **Location:** `crates/roko-gate/src/compile.rs:85-110`
- **What:** Runs `cargo build` / `npm run build` / etc.
- **Code:** No `.stderr()` or `.stdout()` set, relies on `.output()` to capture
- **Risk:** If the gate runs during a background task in unified chat mode,
  compilation warnings could theoretically leak if the capture fails or if
  the code is refactored to use `.spawn()` instead of `.output()`.
- **Note:** Currently safe because `.output()` captures both streams, but fragile.

### H2. Test gate

- [ ] **Audit needed**
- **Location:** `crates/roko-gate/src/test_gate.rs:116-135`
- **What:** Runs `cargo test` / `npm test` / etc.
- **Same pattern as H1** — no explicit stdio setup, relies on `.output()`.

### H3. Bash tool

- [ ] **Audit needed**
- **Location:** `crates/roko-std/src/tool/builtin/bash.rs:86-90`
- **What:** Runs arbitrary `bash -c "<command>"` for agent tool calls
- **Code:** No `.stderr()` setup, relies on `.output()`.
- **Risk:** Same as H1 — fragile capture.

### H4. Run-tests tool

- [ ] **Audit needed**
- **Location:** `crates/roko-std/src/tool/builtin/run_tests.rs:73-100`
- **What:** Runs `cargo test`, `npm test`, `go test`, `pytest`, `forge test`, `make test`
- **Code:** No `.stderr()` setup, relies on `.output()`.

### H5. Process group discovery (pgrep)

- [ ] **Low risk**
- **Location:** `crates/roko-agent/src/process/group.rs:48-50`
- **What:** `pgrep -P <pid>` for process tree discovery
- **Risk:** Minimal — pgrep errors are rare and one-shot.

### H6. Parent process lookup (ps)

- [ ] **Low risk**
- **Location:** `crates/roko-agent/src/process/registry.rs:190-192`
- **What:** `ps -o ppid= -p <pid>` for parent PID
- **Risk:** Minimal — same as H5.

---

## MEDIUM — Direct eprintln!/println! in library crates

These bypass tracing entirely and always go to stderr/stdout regardless of
any tracing configuration. They're the hardest to silence.

### M1. Claude CLI agent — MCP config warning

- [ ] **Needs fix**
- **Location:** `crates/roko-agent/src/claude_cli_agent.rs:271`
- **What:** `eprintln!` warning about MCP config validation
- **When:** Every agent spawn with MCP config

### M2. Claude CLI agent — benign stderr filtering

- [ ] **Needs fix**
- **Location:** `crates/roko-agent/src/claude_cli_agent.rs:394`
- **What:** `eprintln!` when filtering stderr lines
- **When:** During agent execution

### M3. Main.rs — "no config found" warning

- [ ] **Needs fix**
- **Location:** `crates/roko-cli/src/main.rs:2416-2418`
- **What:** `eprintln!("warning: no config found — agent command is \"cat\"...")`
- **When:** Every `roko` invocation without roko.toml
- **Impact:** Shows up in unified chat mode before the ratatui viewport takes over

### M4. Main.rs — observability bootstrap warning

- [ ] **Needs fix**
- **Location:** `crates/roko-cli/src/main.rs:2483`
- **What:** `eprintln!("warning: observability bootstrap failed: {err}")`
- **When:** Observability init failure

### M5. Main.rs — orphaned process reap message

- [ ] **Needs fix**
- **Location:** `crates/roko-cli/src/main.rs:2520`
- **What:** `eprintln!("reaped {reaped} orphaned agent process(es)")`
- **When:** Startup cleanup finds orphaned agents

### M6. Main.rs — timing output

- [ ] **Guard behind tui_mode check**
- **Location:** `crates/roko-cli/src/main.rs:1822-1826`
- **What:** `eprintln!("Completed in {secs:.1}s")` / `eprintln!("Completed in {mins}m...")`
- **When:** `--timing` flag or `ROKO_TIMING=1`

### M7. Main.rs — workdir auto-correction warning

- [ ] **Guard behind quiet check**
- **Location:** `crates/roko-cli/src/main.rs:2293-2295`
- **What:** `eprintln!("Auto-correcting: running from inside .roko/...")`
- **When:** User runs roko from inside `.roko/` directory

### M8. Main.rs — invalid env var warnings

- [ ] **Guard behind quiet check**
- **Location:** `crates/roko-cli/src/main.rs:2335-2336` (ROKO_EFFORT)
- **Location:** `crates/roko-cli/src/main.rs:2370-2371` (ROKO_LOG_FORMAT)
- **What:** `eprintln!("warning: ROKO_EFFORT=... is not valid")`

---

## SAFE — Properly handled subprocess spawns

These are already correct. Listed for completeness and as reference patterns.

| Location | What | Handling |
|---|---|---|
| `crates/roko-agent/src/exec.rs:157-168` | Generic exec agent | `stdout(piped), stderr(piped)` |
| `crates/roko-agent/src/claude_cli_agent.rs:303-348` | Claude CLI spawn | `stdout(piped), stderr(piped)` |
| `crates/roko-cli/src/agent_serve.rs:1002-1012` | Agent sidecar check | `stdout(null), stderr(null)` |
| `crates/roko-cli/src/commands/plan.rs:175-197` | Git commands | `stdout(null), stderr(null)` |
| `crates/roko-runtime/src/process.rs:884-888` | ProcessSupervisor | `stdout(piped), stderr(piped)` |
| `crates/roko-serve/src/routes/vision_loop.rs:133-138` | Vision loop agent | `stdout(piped), stderr(piped)` |
| `crates/roko-acp/src/bridge_events.rs:369-381` | ACP bridge | `stdout(piped), stderr(piped)` |
| `crates/roko-orchestrator/src/worktree.rs:258-268` | Git worktree | `.output()` (captures both) |
| `crates/roko-cli/src/dispatch_direct.rs:48-54` | Claude CLI dispatch | `stdin(piped), stdout(piped), stderr(piped)` |

---

## All app binaries (potential subprocess targets)

| Binary | Path | Has own tracing | Spawned from serve? |
|---|---|---|---|
| `roko` | `crates/roko-cli/` | Yes (main.rs) | No (entry point) |
| `roko-demo` | `crates/roko-demo/` | Yes | No |
| `roko-mcp-scripts` | `crates/roko-mcp-scripts/` | Yes | Via MCP client (C2) |
| `roko-mcp-slack` | `crates/roko-mcp-slack/` | Yes | Via MCP client (C2) |
| `roko-chain-watcher` | `apps/roko-chain-watcher/` | Yes | Yes (C1, now fixed) |
| `agent-relay` | `apps/agent-relay/` | Yes | Not currently |
| `mirage-rs` | `apps/mirage-rs/` | Yes | Not currently |

---

## Recommended fix pattern

For any subprocess spawn, apply this pattern:

```rust
// 1. Always set both stdout and stderr explicitly
let log_path = workdir.join(".roko").join("logs").join(format!("{name}.log"));
let log_file = std::fs::OpenOptions::new()
    .create(true).append(true).open(&log_path)?;
let log_file2 = log_file.try_clone()?;

Command::new(binary)
    .stdout(Stdio::from(log_file))
    .stderr(Stdio::from(log_file2))
    // ... args ...
    .spawn()?;

// 2. For MCP servers specifically, change inherit() to piped() or file
// 3. For .output() calls, add explicit .stderr(Stdio::piped()) for clarity
// 4. Convert all library-crate eprintln! to tracing::warn! or tracing::info!
```

---

## Priority order for fixes

1. **C2** — MCP server stderr inherit (affects every agent with MCP tools)
2. **M1-M2** — Claude CLI agent eprintln (fires during every dispatch)
3. **M3** — "no config found" warning (fires on every bare `roko`)
4. **M4-M8** — Remaining main.rs eprintln calls (guard behind `!tui_mode`)
5. **H1-H4** — Add explicit `.stderr(Stdio::piped())` to gate/tool spawns (defensive)
6. **C3** — Audit MCP server binaries for their own noisy logging
