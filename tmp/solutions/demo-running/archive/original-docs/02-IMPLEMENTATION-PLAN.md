# Implementation Plan: Demo-First Priority Order

Everything ordered by "what gets the demo working soonest." Each wave includes the files to modify, the exact changes, and verification steps.

---

## Wave 1: Pipeline Path Fixes (1-2 hours) — UNBLOCKS DEMO

These are the minimum code fixes to make `idea → draft → promote → plan → validate → run` work end-to-end.

### 1.1 Fix `plan run` path resolution with `--repo`

**File**: `crates/roko-cli/src/commands/plan.rs`
- Lines 209-228: Move `resolve_workdir()` call before `validate_before_run()`
- Lines 965-984: Change `validate_before_run()` to accept `workdir` param instead of using `std::env::current_dir()`
- Resolve `plans_dir` relative to workdir if it's not absolute

**Verify**: `roko --repo /tmp/test-ws plan run plans/` correctly finds plans in `/tmp/test-ws/plans/`

### 1.2 Fix `prd plan` extraction failure

**File**: PRD plan command handler (in `crates/roko-cli/src/commands/prd.rs`)
- Strip tool capabilities from the plan-generation agent dispatch (force text-only output)
- Add post-dispatch validation: if no tasks.toml was produced, print error
- Error message: "Plan generation failed: no tasks.toml produced. Agent may have used tool calls instead of text output."

**Verify**: `roko prd plan <slug>` either produces a valid tasks.toml or prints a clear error

### 1.3 Unify plan schema parsing (validate vs run)

**Files**:
- `crates/roko-cli/src/plan_validate.rs` — uses lenient parsing
- `crates/roko-orchestrator/` — uses strict parsing via `discover_plans()`

**What**: Extract shared `PlanFile` struct with required fields (`meta.plan`, `task[].role`, `task[].id`, `task[].prompt`) into `roko-orchestrator`. Both `plan validate` and `plan run` call the same parser.

**Verify**: A tasks.toml that passes `plan validate` also passes `plan run`, and vice versa

### 1.4 Demo scenario: consistent `--repo` usage

**File**: `demo/demo-app/src/lib/scenario-runners/prd-pipeline.ts`
- Ensure every roko command passes `--repo <workspace>` consistently
- OR: cd into workspace at start, use relative paths for all commands

**Verify**: Run PRD pipeline demo in the demo-app UI — all steps succeed sequentially

---

## Wave 2: Output Quality (2-3 hours) — MAKES DEMO PRESENTABLE

### 2.1 Route tracing to file, not stdout

**File**: `crates/roko-cli/src/main.rs` — tracing subscriber initialization
- Default: write tracing to `.roko/roko.log` file
- `--verbose` flag or `RUST_LOG` env: also write to stderr
- Add `--verbose` flag to the global CLI args (clap)

**Verify**: `roko prd idea "test"` outputs only the `💡 Captured:` line, no WARN/INFO

### 2.2 Suppress false config version warning

**File**: `roko-core/src/config/schema.rs` or `loader.rs`
- Change version detection: check `config_version` field value, not `[providers]` table presence
- Only warn if `config_version` is literally 1 (legacy format)

**Verify**: Freshly init'd workspace produces no warnings

### 2.3 Fix error deduplication

**Files**: Search for `eprintln!` in `crates/roko-cli/src/commands/*.rs`
- Remove all `eprintln!("error: ...")` and `eprintln!("Error: ...")` from command handlers
- Let the single top-level error handler in `main.rs` print errors once
- Use `anyhow::Context` to add descriptive messages to errors

**Verify**: Errors appear exactly once in output

### 2.4 Add spinners for long operations

**Dependency**: Add `indicatif = "0.17"` to `crates/roko-cli/Cargo.toml`

**Files**: `prd.rs`, `plan.rs`, `orchestrate.rs`
- Wrap `prd draft new`, `prd plan`, `plan run` in `indicatif` spinners
- Target: `⠋ Generating PRD draft...  (34s)` instead of silence or log spam
- Use `ProgressBar::new_spinner()` with `.enable_steady_tick(80ms)`

**Verify**: Long operations show animated spinner with elapsed time

### 2.5 Fix negative cost display

**File**: Status command cost display (likely in `crates/roko-cli/src/commands/status.rs`)
- Clamp: `f64::max(0.0, cost)` before formatting
- Replace `format!("${:.4}", cost)` with `format!("${:.4}", cost.max(0.0))`

**Verify**: `roko status` never shows negative cost

---

## Wave 3: Terminal Safety (1 hour) — PREVENTS DEMO CRASHES

### 3.1 Ctrl+C in all chat phases

**File**: `crates/roko-cli/src/chat_inline.rs` lines 1323-1338
- Add `KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL)` to Phase::Error
- Also add to any other phase that has a `_ => {}` wildcard
- Action: set `session.phase = Phase::Done` and break

**Verify**: Ctrl+C exits cleanly from error state in chat mode

### 3.2 RAII terminal cleanup guard

**File**: `crates/roko-cli/src/inline/terminal.rs` line 52
- Create `RawModeGuard` struct with `Drop` impl calling `disable_raw_mode()`
- Store guard in `InlineTerminal` struct — drops automatically on any exit
- Add to: `enable_raw_mode()` call

**Verify**: Kill chat mode with SIGTERM → terminal restored to normal

### 3.3 Panic hook for terminal restore

**File**: Same as 3.2, or in `chat_inline.rs` before entering event loop
- Set panic hook that calls `disable_raw_mode()` before default panic output
- Must be set BEFORE entering raw mode

**Verify**: Code that panics in chat mode still leaves terminal usable

---

## Wave 4: Demo UI Redesign (3-5 hours) — MAKES DEMO COMPELLING

See [04-DEMO-UI-REDESIGN.md](04-DEMO-UI-REDESIGN.md) for the full spec.

### 4.1 Create `CommandList` component
- Right sidebar: ordered list of commands with click-to-run
- Status icons: ○ pending, ⠋ running, ✓ success, ✗ failed
- Only the next sequential command is clickable

### 4.2 Create `ContextPanel` component
- Below command list: shows relevant content based on pipeline stage
- After draft: PRD title, requirements, acceptance criteria
- After plan: task list from tasks.toml
- During run: gate results (✓ compile, ✓ test, etc.)

### 4.3 Simplify PRD pipeline scenario layout
- Single terminal pane (left 70%) instead of multi-pane
- Command list + context panel (right 30%)
- Remove auto-play, speed selector, countdown for this scenario

### 4.4 Refactor `prd-pipeline.ts` scenario runner
- Change from sequential `run(ctx)` to data-driven command list
- Each command defined as `{ id, command, description }`
- `runCommand(ctx, commandId)` method called when user clicks

---

## Wave 5: Provider & Config Quality (3-5 hours) — PREVENTS SETUP FAILURES

### 5.1 Startup provider validation
**File**: Boot sequence in `main.rs`
- Before entering interactive mode: verify configured provider has valid API key
- Check: env var exists OR config has `api_key_env` with valid value
- If no valid provider: print actionable error and exit

### 5.2 Auth detection uses config
**Files**: `crates/roko-cli/src/auth_detect.rs`, config loader
- `detect_auth()` should load unified config, check which providers have credentials
- Return the provider that will ACTUALLY be used for dispatch
- Not an independent env var probe

### 5.3 ACP workspace auto-creation
**File**: `crates/roko-acp/src/handler.rs` line 26-35
- Call `ensure_workspace()` before `setup_file_logging()`
- Fixes: ACP "Failed to Launch" in Zed when .roko/ doesn't exist

### 5.4 ACP log file fallback
**File**: Same as 5.3
- If `.roko/acp.log` parent dir doesn't exist, fall back to `/tmp/roko-acp-{pid}.log`
- Also: send JSON-RPC error response before exit so editor shows meaningful message

---

## Wave 6: Remaining Audit Work (ongoing)

See [05-REMAINING-AUDIT.md](05-REMAINING-AUDIT.md) for the full inventory with technical detail.

---

## Timeline Estimate

| Wave | Scope | Effort | Demo Impact |
|------|-------|--------|-------------|
| 1 | Pipeline path fixes | 1-2h | Pipeline works E2E |
| 2 | Output quality | 2-3h | Clean, presentable output |
| 3 | Terminal safety | 1h | No more freezes |
| 4 | Demo UI redesign | 3-5h | Compelling UX |
| 5 | Provider/config | 3-5h | Setup doesn't fail |
| 6 | Full audit backlog | Ongoing | Quality + stability |

**Waves 1-3 are the critical path** (~4-6 hours). Get those done and the demo works, looks clean, and doesn't crash.
