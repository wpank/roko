# Complete Remaining Work Inventory

Every open item from three audit documents (`infrastructure-audit.md`, `model-provider-audit.md`, `redesign-plan.md`), with full technical context for implementation by agents without prior knowledge.

**Codebase**: Roko — 18 Rust crates, ~177K LOC. Workspace at `/Users/will/dev/nunchi/roko/roko/`.

---

## What's DONE (47 batches, committed)

These are resolved and should NOT be re-implemented:

- Provider/model synthesis removed from ModelCallService, ACP, core (batches 6, 10-14, 19-20)
- Direct agent-exec learning persistence wired across all dispatch paths (batches 8-9, 12, 15-18, 21)
- CLI model selection requires explicit config profiles (batch 22)
- Serve provider health requires explicit config (batch 23)
- Provider factory rejects missing providers (batch 24)
- ACP explicit config path, session revalidation, config options (batches 25-30)
- Demo workspace-explicit commands, PRD CWD guards (batches 31-34)
- Safety contract trust boundaries + asset embedding (batches 35, 38)
- Model max_output ceilings (batch 36)
- Health endpoint + graceful shutdown (batch 37)
- Request-timeout, retry-policy, circuit-breaker, tool-loop, vision-loop defaults centralized (batches 39-47)
- Cascade router persistence with stable model candidates (batch 5)
- Gateway provider health records provider IDs (batch 7)
- SystemPromptBuilder, EpisodeLogger, ProcessSupervisor, MCP passthrough, gate pipeline (all wired)

---

## P0 — Blocks Basic Usage

### Terminal Raw Mode Cleanup (chat freeze)

**Problem**: Running `roko` chat mode → getting an error → terminal completely unresponsive. Three compounding issues:

1. `crates/roko-cli/src/chat_inline.rs` lines 1323-1338: Phase::Error handler has `_ => {}` wildcard that EATS Ctrl+C
2. `crates/roko-cli/src/inline/terminal.rs` line 52: `enable_raw_mode()` with no RAII Drop guard
3. `crates/roko-cli/src/chat_inline.rs` line 1280: Synchronous `crossterm::event::poll(33ms)` blocks async signal handling

**Fix**: See [01-BLOCKERS.md](01-BLOCKERS.md) B6 for complete code.

### ACP "Failed to Launch" in Zed

**Problem**: Zed editor shows "Failed to Launch — server shut down unexpectedly" when using roko as ACP provider.

**Root cause**: `crates/roko-acp/src/handler.rs` lines 26-35 — `setup_file_logging(config.log_file())` fails if `.roko/` doesn't exist. Error goes to stderr (invisible to Zed). Process exits 1.

**Fix**:
1. Call `ensure_workspace()` before logging setup (same as CLI does in `unified.rs:44`)
2. Canonicalize workdir: `std::fs::canonicalize(workdir)` (default is relative `"."`)
3. Log fallback: if `.roko/acp.log` fails, use `/tmp/roko-acp-{pid}.log`
4. Send JSON-RPC error response on stdout before exit so editor shows meaningful message

**Files**: `crates/roko-acp/src/handler.rs`, `crates/roko-cli/src/main.rs:1765-1774`

### Auth Detection ↔ Dispatch Config Mismatch

**Problem**: Banner says "auth: glm-5.1 (OpenAI-compat)" but dispatch fails with "no API key for provider 'anthropic_api'".

**Root cause**: Two independent systems:
- `detect_auth()` in `crates/roko-cli/src/auth_detect.rs` probes env vars: Claude CLI → ANTHROPIC_API_KEY → ZAI_API_KEY → OPENAI_API_KEY. Finds ZAI_API_KEY → reports "glm-5.1"
- `ChatAgentSession` loads `roko.toml` config, uses cascade router, picks default model which may point to different provider (anthropic_api)
- These systems never talk to each other

**Fix**: `detect_auth()` should load the unified config, check which configured providers have valid credentials, and return the provider that will ACTUALLY be used for dispatch. Not an independent env var probe.

### Unified Config Loading (20+ loaders → 1)

**Problem**: 20+ distinct `load_roko_config()` functions across the codebase, each with different behavior for global merge, env overrides, validation:

| # | Function | File |
|---|----------|------|
| 1 | `AcpConfig::load_roko_config` | `roko-acp/src/config.rs:48` |
| 2 | `load_roko_config` | `roko-cli/src/config_helpers.rs:121` |
| 3 | `load_roko_config` | `roko-cli/src/orchestrate.rs:863` |
| 4 | `load_roko_config` | `roko-cli/src/agent_serve.rs:559` |
| 5 | `load_roko_config` | `roko-cli/src/main.rs:2480` |
| 6-20 | Various | event_sources, subscriptions, vision_loop, serve, run, etc. |

**Fix**: Delete the one-off loaders. Replace each callsite with `roko_core::config::loader::load_config_unified(workdir)`. Core loader already exists with 6 entry points — reduce to 2: `load(workdir)` and `load_file(path)`.

### Multi-Process File Locking

**Problem**: `RokoLayout` defines `.roko/runtime/roko.lock` but **never creates or checks it**. Two simultaneous `roko plan run` commands corrupt `.roko/state/executor.json`, `.roko/episodes.jsonl`, `.roko/learn/*.json`.

**Fix**: Advisory file lock via `fs2::FileExt::lock_exclusive()` at startup:
```rust
fn acquire_workspace_lock(roko_dir: &Path) -> Result<std::fs::File> {
    let lock_path = roko_dir.join("runtime/roko.lock");
    std::fs::create_dir_all(lock_path.parent().unwrap())?;
    let file = std::fs::OpenOptions::new().create(true).write(true).open(&lock_path)?;
    match file.try_lock_exclusive() {
        Ok(()) => { writeln!(&file, "{}", std::process::id())?; Ok(file) }
        Err(_) => bail!("Another roko process running (PID {})", fs::read_to_string(&lock_path)?.trim()),
    }
}
```

---

## P1 — Frustrating But Workable

### Boot Sequence Abstraction

**Problem**: Each entry point (CLI chat, ACP, serve, plan run) implements ad-hoc startup with different config loading, workspace init, signal handling, error reporting.

**Fix**: Single `RokoBootstrap` struct:
```rust
struct RokoBootstrap {
    config: RokoConfig,
    workdir: PathBuf,
    available_providers: Vec<ProviderStatus>,
    workspace_ready: bool,
}
impl RokoBootstrap {
    fn new(workdir: &Path, opts: BootOpts) -> Result<Self, BootError> {
        // 1. Canonicalize workdir
        // 2. Load unified config (global + project + env)
        // 3. Ensure workspace (.roko/) exists
        // 4. Validate providers (check API keys)
        // 5. Return boot state or actionable error
    }
}
```

### Workspace Persistence

**Problem**: `ephemeral_workspaces: RwLock<HashMap<...>>` in `AppState` is in-memory only — lost on server restart. Demo workspaces vanish when `roko serve` restarts.

**Fix**: Persistent workspace registry at `.roko/workspaces.json`. Validate paths on access. Workspace reattach: demo UI sends workspace ID, server returns same if valid.

### Config Loaded from Worktree, Not Project Root

**File**: `crates/roko-cli/src/orchestrate.rs:1568`

**Problem**: `load_roko_config(&cfg.exec_dir)` where `exec_dir` is a git worktree with no `roko.toml`. Falls through to `unwrap_or_default()` → empty config. All user-configured provider routing silently ignored for parallel tasks.

**Fix**: Load config from project root, not worktree.

### PRD Promote Is Non-Atomic

**File**: `crates/roko-cli/src/commands/prd.rs` lines 746-747

**Problem**: Writes published file, then deletes draft. Crash between write and delete leaves both. Next promote silently overwrites.

**Fix**: Atomic: write to `.tmp` file, rename to final path, then delete draft.

### Silent Fallback to `cat` Agent

**Problem**: Running `roko` without `roko.toml` prints warning then enters chat with `cat` agent that just echoes input. User thinks roko is broken.

**Fix**: Refuse to enter chat mode without valid provider. Print: "No roko.toml found. Run `roko init` or set ANTHROPIC_API_KEY."

### No Provider Validation During `roko init`

**Problem**: Generates `roko.toml` with `claude_cli` default but never checks if `claude` is installed or API keys exist.

**Fix**: After init, check configured provider binary on PATH and API key availability. Print summary with warnings.

### Doctor Doesn't Offer Fixes

**Problem**: `roko doctor` says "[fail] config: missing project roko.toml" but doesn't suggest: "fix: run `roko init`"

**Fix**: Add suggestion lines to each diagnostic.

### Provider Binary Pre-Flight

**Problem**: Using `claude_cli` when `claude` not installed → "spawn failed: No such file or directory" AFTER task context built (minutes wasted).

**Fix**: Check provider binary at boot, not at dispatch time.

### Gate Dependencies Pre-Flight

**Problem**: Gates assume `cargo`, `git`, `clippy` available. Missing → gate fails mid-execution.

**Fix**: Pre-validate before starting `plan run`.

### Tracing Noise to Stdout

See [01-BLOCKERS.md](01-BLOCKERS.md) B4.

### Plan Run Progress Output

**Problem**: `roko plan run` executes for minutes with zero CLI output.

**Fix**: Add `indicatif` multi-progress with per-task spinners:
```
⟐ Running plan: temp-converter (2 tasks)
  [1/2] scaffold — ⠋ Implementing...  (34s)
```

### Duplicated Error Messages

See [01-BLOCKERS.md](01-BLOCKERS.md) B7.

### False Config Version Warning

See [01-BLOCKERS.md](01-BLOCKERS.md) B5.

### Negative Cost Display

See [01-BLOCKERS.md](01-BLOCKERS.md) B8.

### PRD Commands Don't Record Learning Observations

**Problem**: `prd draft`, `prd plan`, `research` commands dispatch LLMs but don't instantiate learning context. Cascade router never sees outcomes from PRD operations — blind spot.

**Fix**: Create minimal learning context (load router, record observation, save) in PRD command handlers.

### Safety Layer Is Optional

**File**: `crates/roko-agent/src/dispatcher/mod.rs:108-118`

**Problem**: `safety` field defaults to `None`. Dispatchers without `.with_safety()` have zero safety checks. No warning.

**Fix**: Make safety layer required (change from `Option<SafetyLayer>` to `SafetyLayer`).

---

## P2 — Polish & Ergonomics

### Dev Orchestrator (`roko dev`)

**Problem**: Triple process spawn (cargo watch + roko serve + demo-app dev) with "Address already in use" on port 6677, no signal propagation.

**Fix**: `roko dev` command with PID file, port retry, signal propagation to children.

### Terminal Session Reattach

**Problem**: PTY sessions destroyed on WebSocket disconnect. Page refresh loses terminal.

**Fix**: Persist sessions backed by tmux or state file. Reconnect reattaches.

### Docker/Railway Deployment

**Remaining**: Multi-stage Dockerfile (not a 1.5GB image), sidecar separation, production CORS.

### Process Confinement for Bash Tool

**File**: `crates/roko-std/src/tool/builtin/bash.rs:86-100`

**Problem**: Bash tool has denylist (`rm -rf /` blocked) but trivially bypassed. Full filesystem access.

### Sync Mutex in Async Functions

**File**: `roko-serve/src/state.rs:361` — `parking_lot::Mutex<DaimonState>` accessed from async handlers blocks OS thread.

**Fix**: Replace with `tokio::sync::Mutex`.

### Nested Async Mutex Deadlock Risk

**File**: `roko-learn/src/playbook.rs:727-741` — `save_or_merge` acquires two locks across `.await` points.

### MCP Client Lock Across I/O

**File**: `roko-agent/src/mcp/client.rs:157-160` — stdin/stdout mutexes held during child process I/O.

### Polling-Based Cancellation

**File**: `crates/roko-agent/src/dispatcher/cancel.rs:26-35` — polls at 50ms instead of using `tokio::sync::Notify`.

### Race Conditions (TOCTOU)

Pattern across codebase: `if path.exists() { fs::read(&path) }` — file may be deleted between check and read.

**Fix**: Use `fs::read` directly; handle `NotFound` in error branch.

### Three Gate Rungs Always Skipped

**File**: `crates/roko-cli/src/orchestrate.rs:17423-17443` — Symbol, PropertyTest, Integration rungs permanently skipped.

### Rung Config Uses Raw Integers

**File**: `crates/roko-cli/src/orchestrate.rs:17619-17678` — `if rung == 5`, `if rung > 6` mixed with `Rung` enum.

### Frontmatter Parser Not YAML

**File**: `crates/roko-cli/src/commands/prd.rs:484-521` — manual line scanner breaks on colons, quotes, lists.

### Provider Error Messages Not Actionable

**Problem**: Raw HTTP 404/401/429 bodies surface to users without context.

**Fix**: Wrap at dispatch boundary:
- 404 → "Model '{slug}' not found on {provider}. Check roko.toml."
- 401 → "{provider} API key invalid. Check ${env_var}."
- 429 → "{provider} rate limited. Wait or switch provider."
- ENOENT → "{command} not found on PATH. Install it."

### No Provider Pre-Flight Check

**Problem**: Checked at dispatch time after minutes of context assembly.

**Fix**: Include in boot sequence.

### Provider Config Discovery Opaque

**Problem**: No `roko config providers available` listing supported kinds.

**Fix**: List all `ProviderKind` variants with required env var, example URL, compatible models.

### Resolve Enrichment Backend Removal

**Problem**: `resolve_enrichment_backend()` in `orchestrate.rs` uses substring matching to guess provider (e.g., "gemini" → Codex, which is wrong).

**Fix**: Delete this function. Use config-backed model resolution exclusively.

### AgentBackend::from_model() Removal

**Problem**: Core heuristic guesses provider from model slug substring. `starts_with("sonnet-")` routed to Cursor (wrong, collides with Claude).

**Partial**: CLI/serve paths fixed (batches 22-23). Core heuristic still exists for compatibility.

**Fix**: Error on unknown model slug instead of guessing.

### ACP Live Config Watch/Reload

**Problem**: Config changes require ACP restart.

### Effective_models() Tier Field

**Problem**: `slug_to_tier()` uses substring heuristic as fallback.

**Fix**: Add `tier: Option<ModelTier>` to `ModelProfile` schema. Use config field.

### Guided Provider Setup Wizard, `config providers add`, Streaming Chat, `status --quick`

All not started — see implementation plan Wave 5.

### CLI Output Redesign

**Not started**: Full redesign with `CliReporter` trait, `indicatif` spinners, `owo-colors`, `MultiProgress` for plan run, `--output json` flag, error dedup.

**Key crates**: `indicatif = "0.17"`, `owo-colors = "4"` (add to roko-cli/Cargo.toml)

**See**: redesign-plan.md Phase 14 for full architecture.

### Build Friction

- `crates/roko-cli/src/main.rs:10-20`: Blanket `#![allow(clippy::all, ...)]` hides real issues
- ~70 `#[allow(dead_code)]` for Phase 2+ scaffolding
- No `rust-toolchain.toml` (requires 1.91+ but not enforced)

---

## P3 — Error Quality & Code Health

### Error Type Hierarchy

**Problem**: Mix of `anyhow::Result`, custom enums, `Box<dyn Error>` with no hierarchy.

**Fix**: Per-crate error enum using `thiserror` with typed variants.

### Silent Error Swallowing (120+ instances)

Pattern: `let _ = fallible_op()` without logging. Worst: 60+ in `roko-acp/src/bridge_events.rs`.

### Unwrap/Expect in Production (150+ instances)

Worst: 86 in `roko-chain/src/marketplace.rs`, `panic!("wrong variant")` in Display impl at `roko-core/src/error/mod.rs:652-685`.

### Context Window Pressure Watcher — Dead Subsystem

**Files**: `context_window_pressure.rs` — hardcodes only Claude models (Opus 1M, Sonnet 200K, Haiku 200K). Returns None for all other models. Intervention signals have no consumer. Entire subsystem has no runtime effect.

### Tool Calling Inconsistencies (cross-provider)

- Three different DEFAULT_MAX_TOKENS across providers
- Tool loop iteration limits differ by provider
- Gemini URL double-prefix
- GeminiAdapter never instantiated
- Claude CLI usage reports zero tokens
- Anthropic API tool loop is dead code

### Observability Gaps

- No TTFT measurement at HTTP layer
- No Prometheus /metrics endpoint
- No distributed request tracing
- No per-plan cost aggregation in API
- No bench regression detection

### Demo App Anti-Patterns (P3)

- Polling loops (setInterval) instead of SSE
- Stale closures in useEffect
- Missing cleanup functions
- Magic numbers (hardcoded URLs, timeouts)
- No error boundaries

---

## Redesign Plan Phase Status

| Phase | Description | Status | Key Remaining |
|-------|-------------|--------|---------------|
| 0 | Boot/terminal fixes | **Not started** | RAII guard, Ctrl+C, provider validation, ACP workspace |
| 1 | Core foundations | **Not started** | Config unification, error hierarchy |
| 2 | Provider redesign | **Done** (batches 5-30) | — |
| 3 | Tool system | **Partial** | Schemas done; process confinement not started |
| 4 | Orchestration | **Partial** | Fixes applied; full state-machine redesign not done |
| 5 | Workspace/serve | **Partial** | Health done; workspace persistence not started |
| 6 | Learning/compose | **Done** | — |
| 7 | Frontend | **Not started** | Full demo-app redesign |
| 8 | Deployment/CI | **Partial** | Health probes done; Docker not started |
| 9 | Concurrency | **Partial** | Some fixes; sync mutex, lock ordering remain |
| 10 | ACP/Editor | **Partial** | Batches 25-30 done; live config watch not started |
| 11 | First-run UX | **Not started** | Cat fallback, provider setup wizard, doctor fixes |
| 12 | Error quality | **Not started** | Typed errors, silent swallowing, unwrap audit |
| 13 | Dev experience | **Not started** | Clippy blanket, dead code, rust-toolchain.toml |
| 14 | CLI output | **Not started** | CliReporter, indicatif, owo-colors, MultiProgress |
