# Unified Chat Blockers — 2026-04-27

Root cause analysis + fix checklist for the `roko` (no-args) inline chat experience.

---

## Status Key

- [x] = Fixed
- [ ] = Open
- ~~text~~ = No longer relevant

---

## SECTION A: Log & Process Issues

### Issue 1: Chain-watcher floods terminal

- [x] **Fixed** — stderr redirected to `.roko/chain-watcher.log`, `ROKO_LOG=warn` env passed
- **Where:** `crates/roko-serve/src/lib.rs:267-300`
- **Root cause:** Separate binary with own tracing, inherited stderr, `.status()` not `.spawn()`

### Issue 2: In-process tracing leaks to terminal

- [ ] **A. Audit all `eprintln!` in serve startup path** (see `04-SUBPROCESS-LOG-LEAK-CATALOG.md`)
- [x] **B. `tui_mode` now includes unified chat mode** — `main.rs:1668`
- [ ] **C. Move ALL informational `eprintln!` behind `!quiet` or `!tui_mode` guards**

### Issue 3: Ctrl+C doesn't kill background tasks

- [ ] **A. Store child `Child` handle** for chain-watcher, kill on shutdown
- [ ] **B. Install process-level Ctrl+C handler** in `cmd_unified_chat`
- [ ] **C. Thread `CancellationToken`** into chat loop → propagate to all tasks
- [ ] **D. Ctrl+C during dispatch doesn't cancel the background task** — Claude CLI
  subprocess or HTTP request keeps running. Only UI moves to Input phase.
  `chat_inline.rs:305-323`

### Issue 4: No bounded viewport — logs interleave with chat

- [ ] **A. Use alternate screen mode** (tradeoff: lose scrollback)
- [ ] **B. Or: ensure ZERO stderr output** (fix issues 1-3 above)
- [ ] **C. Dedicated log panel** — bounded region at top for serve status

### Issue 5: Background serve starts everything unnecessarily

- [ ] **A. Add `ServeProfile` enum** (`Full`, `ChatOnly`) to `start_server_background`
- [ ] **B. Don't spawn chain-watcher from background serve**
- [ ] **C. Make each subsystem respect `CancellationToken`**

---

## SECTION B: Auth & Provider Issues

### Issue 6: Auth detection was picking broken Claude CLI

- [x] **A. Reordered:** `ZAI_API_KEY` → `ANTHROPIC_API_KEY` → `OPENAI_API_KEY` → claude CLI
- [x] **B. Updated roko.toml defaults** to `default_backend = "zhipu"`, `default_model = "glm-5.1"`
- [x] **C. Model name passed through `AuthMethod::OpenAiCompat`**
- [x] **D. `ZAI_MODEL` env var** for overriding the GLM model slug
- [ ] **E. auth_detect should read roko.toml** `[agent].default_backend` and prefer it.
  Currently pure env-based, ignores config entirely. Config loaded in `unified.rs`
  but never consulted for provider selection.
- [ ] **F. Auth detection doesn't validate keys** — it checks if env var is set, not
  if the key actually works. Should do a lightweight health check (e.g. GET /models)
  to avoid picking a dead provider.

### Issue 7: Cargo warnings flood terminal

- [ ] **A. Clean up ~80 warnings** — `cargo fix --lib -p roko-cli --tests`
- [ ] **B. Document:** use compiled binary `./target/debug/roko`, not `cargo run`
- [ ] **C. Add alias/wrapper** that builds quietly

---

## SECTION C: Hardcoded Values

### Issue 8: Hardcoded model names

| Location | Value | Should be |
|---|---|---|
| `dispatch_direct.rs:147` | `"claude-sonnet-4-6-20250514"` | Config or constant from `roko-agent` |
| `dispatch_direct.rs:225` | fallback `"gpt-4o"` | Config `[agent].default_model` |
| `auth_detect.rs:63` | `"glm-5.1"` | Config or `ZAI_MODEL` env (added but fragile) |
| `chat_inline.rs:735` | `"claude-sonnet-4-6 (Anthropic API)"` | Read from dispatch result |

- [ ] **Fix: All model names should come from config or be constants**

### Issue 9: Hardcoded URLs

| Location | Value | Should be |
|---|---|---|
| `dispatch_direct.rs:153` | `"https://api.anthropic.com/v1/messages"` | Constant from `roko-agent` |
| `auth_detect.rs:62` | `"https://open.bigmodel.cn/api/paas/v4"` | Config `[providers.zhipu].base_url` |
| `auth_detect.rs:80` | `"https://api.openai.com/v1"` | Already has env fallback, ok |
| `unified.rs:56` | `", serve :6677"` | Read actual bound port from state |

- [ ] **Fix: Read from config or roko-agent constants**

### Issue 10: Hardcoded API version

| Location | Value | Constant exists in |
|---|---|---|
| `dispatch_direct.rs:155` | `"2023-06-01"` | `roko-agent/src/claude_agent.rs:32` as `DEFAULT_ANTHROPIC_VERSION` |

- [ ] **Fix: Import and reuse the constant from roko-agent**

### Issue 11: Hardcoded token limits (inconsistent!)

| Location | Value | Library default |
|---|---|---|
| `dispatch_direct.rs:148` | `8192` (Anthropic) | `roko-agent` uses `4096` |
| `dispatch_direct.rs:230` | `8192` (OpenAI) | `roko-agent` uses `4096` |

- [ ] **Fix: Use roko-agent's `DEFAULT_MAX_TOKENS` constant or read from config**

### Issue 12: Hardcoded timeouts (inconsistent!)

| Location | Value | Context |
|---|---|---|
| `chat_inline.rs:296` | `33ms` | Event poll rate (~30fps) |
| `chat_inline.rs:922` | `120s` | HTTP request timeout |
| `chat_inline.rs:959` | `500ms` | Async poll interval |
| `chat.rs:289,313` | `2s` | Completely different timeout for same operation |

- [ ] **Fix: Define constants or read from `[tui]` config section**

### Issue 13: Hardcoded environment variable names

Undocumented env vars scattered across files:

| Env var | Where | Documented? |
|---|---|---|
| `ZAI_API_KEY` | `auth_detect.rs:57` | No |
| `ZAI_MODEL` | `auth_detect.rs:59` | No |
| `OPENAI_API_BASE` | `auth_detect.rs:78` | No |
| `OPENAI_BASE_URL` | `auth_detect.rs:79` | No |
| `ROKO_LOG` | `main.rs` | Partially |
| `ROKO_LOG_RAW` | `main.rs:1695` | No |
| `ROKO_TIMING` | `main.rs:1675` | No |
| `ROKO_EFFORT` | `main.rs:2335` | No |
| `ROKO_LOG_FORMAT` | `main.rs:2370` | No |

- [ ] **Fix: Centralize all env var names as constants in one module, document in CLAUDE.md**

---

## SECTION D: Architectural Issues

### Issue 14: dispatch_direct.rs duplicates roko-agent (~400 LOC)

`dispatch_direct.rs` reimplements Anthropic API dispatch, OpenAI-compat dispatch,
and response parsing that already exist in `roko-agent`. This means:

- Different `max_tokens` defaults (8192 vs 4096)
- Different API version headers (hardcoded vs constant)
- Different error handling paths
- Different response type definitions (duplicate structs)
- No streaming support (roko-agent has it, dispatch_direct doesn't use it)

| Component | dispatch_direct.rs | roko-agent equivalent |
|---|---|---|
| Anthropic dispatch | Lines 143-189 | `claude_agent.rs` (970 lines, full featured) |
| OpenAI dispatch | Lines 217-267 | `openai_agent.rs` (615 lines, full featured) |
| Response structs | Lines 191-295 | `translate.rs`, `chat_types.rs` |
| API version | Hardcoded `"2023-06-01"` | `DEFAULT_ANTHROPIC_VERSION` constant |

- [ ] **Fix: Refactor dispatch_direct to use roko-agent's `ProviderAdapter` trait.**
  This eliminates ~400 lines and syncs all dispatch behavior.

### Issue 15: No streaming — UI freezes during response

The chat claims to support streaming (has `Phase::Streaming`, `StreamingState`)
but actually **blocks on full response**:

1. `dispatch_prompt()` waits for complete text
2. Full response sent through channel as one block
3. `StreamingState.append()` never called incrementally
4. `render_viewport()` draws complete text, not growing stream

The Claude CLI path reads `--output-format stream-json` line by line but
accumulates ALL lines before returning. The infrastructure for streaming
exists (SSE parsing, delta appending) but isn't wired.

- [ ] **Fix: Emit deltas through channel as they arrive from the CLI/API.**
  Change `dispatch_prompt()` to accept a `mpsc::Sender<StreamDelta>` and
  emit incremental text as it's parsed from the JSON stream.

### Issue 16: No multi-turn conversation context

Each message is dispatched as a **standalone single-turn prompt**. There is no
conversation history. The user types "hello", gets a response, types "what did
I just say?" — and the model has no memory of "hello".

- `ChatSession` has no `messages: Vec<Message>` field
- HTTP path sends `{ "message": text }` — single message, no history
- Direct path calls `dispatch_prompt(auth, text)` — single prompt
- No system prompt injection

This is the #1 feature gap vs Claude Code.

- [ ] **Fix: Add conversation buffer to `ChatSession`, pass full history to dispatch.**
  For OpenAI-compat: send `messages: [{role: "user", content: "..."}, {role: "assistant", ...}, ...]`
  For Claude CLI: use `--resume` flag or build context in prompt.

### Issue 17: Cost tracking broken — always shows $0.0000

`CostMeter` exists and has proper fields, but:

1. `record_run()` is called with `cost: 0.0` (chat_inline.rs:334-337)
2. Token counts are **guessed** from `reply.len() / 4` instead of using
   actual `DispatchResult.input_tokens / output_tokens`
3. `DispatchResult` returns real token counts but they're ignored

- [ ] **Fix: Pass `DispatchResult` to `record_run()` with actual tokens and
  compute cost from model pricing table.** roko-learn already has `CostTable`.

### Issue 18: chat_inline.rs and chat.rs have ~40% code duplication

Two separate chat implementations:
- `chat_inline.rs` (1092 lines) — new ratatui inline TUI
- `chat.rs` (632 lines) — legacy line-oriented REPL

Shared logic: HTTP dispatch, response extraction, agent resolution, cost
tracking, slash commands. No shared base.

- [ ] **Fix: Extract shared logic into `chat_common.rs` or traits.**

### Issue 19: Config loaded but never used for dispatch

`unified.rs:42` loads config via `load_config_or_defaults()` but the config's
`[providers]` section is never consulted for dispatch. Auth detection is pure
env-based. The config's model routing, provider health, and cascade router
are all bypassed in the unified chat path.

- [ ] **Fix: Bridge auth_detect to config.** If config specifies
  `default_backend = "zhipu"`, resolve the provider from `[providers.zhipu]`
  instead of hardcoding the URL/model in auth_detect.

---

## SECTION E: Missing Chat Features (vs 01-UNIFIED-DESIGN.md)

| Feature | Design | Status | Difficulty |
|---|---|---|---|
| `/agent <name>` | Switch agent | Missing | Medium — needs agent registry |
| `/agents` | List agents | Missing | Easy — read `.roko/runtime/agents.json` |
| `/status` | Workspace status | Missing | Easy — call `collect_session_status()` |
| `/plan run` | Execute plan from chat | Missing | Hard — needs background orchestration |
| `/share` | Gist sharing | Missing | Medium — needs gh CLI |
| `/web` | Open browser | Missing | Easy — `open http://localhost:6677` |
| Multi-line input | Shift+Enter | Missing | Medium — crossterm key detection |
| Tab completion (commands) | `/` + Tab | **DONE** | Tab/Shift+Tab cycles through matches |
| Tab completion (paths) | File paths | Missing | Hard — filesystem scanning + prefix match |
| Streaming tokens | Live render | Missing | Hard — channel-based delta delivery |
| Multi-turn context | Conversation memory | Missing | Medium — message buffer + API change |
| Gate results inline | Show pass/fail | Missing | Medium — gate pipeline integration |
| Tool call display | Collapsible | Missing | Hard — structured tool call parsing |
| Cost waterfall | Per-response breakdown | Missing | Medium — needs real cost data |

---

## SECTION F: Demo Infrastructure Issues

### Issue 20: Demos used hardcoded `./target/release/roko` path

- [x] **Fixed** — All demo files auto-detect roko binary at runtime

The PTY shell may not have `roko` on PATH. Both `builder.html` and
`terminal.html` now **probe the shell** at runtime: `command -v roko` first,
then fall back to `./target/release/roko` → `./target/debug/roko`.

**Files fixed:**

| File | Change |
|---|---|
| `demo/demo-web/terminal.html` | `resolveRokoPath()` — probes shell, falls back to local builds |
| `demo/demo-web/builder.html` | `resolveRoko()` — same pattern |
| `demo/demo-resources/run-tui-demo.sh` | PATH-first resolution |
| `demo/demo-resources/provider-routing/common.sh` | PATH-first in `find_roko()` |
| `crates/roko-cli/src/demo_cmd.rs` | `current_exe()` instead of hardcoded path |

### Issue 21: Demos don't use a clean working directory

- [x] **Fixed in terminal.html + builder.html** — commands `cd` into `/tmp/roko-demo`
- [ ] **Shell scripts need similar treatment** — most operate in repo root, which
  pollutes the workspace with `.roko/`, PRDs, signals, etc.

**What needs to happen:**
- [ ] All shell demo scripts should set `DEMO_DIR="${DEMO_DIR:-/tmp/roko-demo}"` at top
- [ ] `roko init` should run in `$DEMO_DIR`, not repo root
- [ ] `run-all.sh` should create + cleanup the demo dir
- [ ] Demo `roko.toml` from `demo/demo-resources/roko.toml` should be copied in

**Scripts that still need demo dir:**
- [ ] `demo/demo-resources/full-self-hosting/demo-full-loop.sh`
- [ ] `demo/demo-resources/prd-workflow/demo-prd-cli.sh`
- [ ] `demo/demo-resources/benchmark-flow/demo-benchmark.sh`
- [ ] `demo/demo-resources/agent-matchmaking/demo-match.sh`
- [ ] `demo/demo-resources/agent-workflows/01-single-agent.sh`
- [ ] `demo/demo-resources/agent-workflows/02-multi-agent.sh`
- [ ] `demo/demo-resources/agent-workflows/03-chat-repl.sh`
- [ ] `demo/demo-resources/research-workflow/demo-research.sh`
- [ ] `demo/demo-resources/run-all.sh` (orchestrator)

### Issue 22: `demo_cmd.rs` uses `std::process::Command` not `tokio::process`

- [ ] **Low priority** — `demo_cmd.rs:57` spawns `roko init` synchronously via
  `std::process::Command`. This blocks the async runtime. Should use
  `tokio::process::Command` for consistency, though in practice this only
  runs once during demo setup.

### Issue 23: Demo commands fired without waiting for completion

- [x] **Fixed** — Both HTML demos now wait for shell prompt between commands

Previously `terminal.html` used fixed `setTimeout` delays (3-8s) between
commands. If a command took longer, the next one was typed mid-output,
producing garbled results. Now:

- **`builder.html`**: `runCmd()` types command, then `waitForPrompt()` polls
  terminal output for shell prompt regex (`[❯$>]\s*$`).
- **`terminal.html`**: Same pattern via `runCommand()` + `waitForPrompt()`.
- Both have configurable timeouts per command type.

### Issue 24: No way to pause/stop demo execution

- [x] **Fixed in terminal.html** — "Run All Demos" button toggles to "Pause"/"Resume"

During demo execution, clicking the button pauses all command submission.
Each command's loop checks `demoPaused` before proceeding. Click again
to resume.

---

## SECTION G: Tab Completion Architecture

### What was implemented

- [x] **Slash command tab completion** — Type `/` then Tab to cycle through
  matching commands. Shift+Tab cycles backwards. Any other key resets.
- [x] **`SLASH_COMMANDS` constant** — centralized list of all commands with
  descriptions, used by both tab completion and `/help` rendering.

### Current limitations + next steps

- [ ] **Tab completion only for commands, not arguments.** `/model ` + Tab
  should list available models (glm-5.1, glm-4-plus, etc.) but currently
  does nothing.
- [ ] **No inline completion hint.** Modern terminals show a dimmed suggestion
  after the cursor (e.g. fish shell). Would need changes to `render_viewport`
  to overlay suggestion text.
- [ ] **No file path completion.** `/share` or future tool-call arguments
  would benefit from filesystem tab completion.
- [ ] **`SLASH_COMMANDS` is disconnected from `handle_slash_command`.** The
  command list is defined twice — once in the constant, once in the match.
  Adding a command requires updating both. Should be a single source of truth.

### Architectural improvement: Command registry

Replace the current pattern (separate constant + match) with a registry:

```rust
struct SlashCommand {
    name: &'static str,
    aliases: &'static [&'static str],
    description: &'static str,
    // Whether the command takes arguments
    has_args: bool,
    // For argument tab-completion
    arg_completer: Option<fn(&str) -> Vec<String>>,
}

static COMMANDS: &[SlashCommand] = &[
    SlashCommand {
        name: "/model",
        aliases: &[],
        description: "show or change model",
        has_args: true,
        arg_completer: Some(complete_model_name),
    },
    // ...
];
```

This enables:
- Single source of truth for commands
- Argument-level completion per command
- Dynamic commands (plugins, agent-specific commands)
- Auto-generated help text

---

## SECTION H: Refactoring Proposals

### R1. Service dependency graph

- [ ] Define `Service` trait: `start()`, `shutdown()`, `name()`
- [ ] `ServiceManager` tracks handles + child PIDs
- [ ] `shutdown_all()` in reverse dependency order
- [ ] Each service gets `CancellationToken`

### R2. Log isolation architecture

- [ ] Rule: zero `eprintln!` from library crates
- [ ] All status → `StateHub` events
- [ ] Subprocess output → `.roko/logs/<name>.log`
- [ ] CI lint: `grep -rn 'eprintln!' crates/` should only match tests

### R3. Process tree management

- [ ] Store all `Child` handles in `ProcessRegistry` on `AppState`
- [ ] `shutdown()` sends SIGTERM → wait 2s → SIGKILL
- [ ] `atexit` handler kills children on panic
- [ ] Reuse `ProcessSupervisor` from `roko-runtime`

### R4. Unified provider abstraction

- [ ] Delete `dispatch_direct.rs` entirely
- [ ] Use `roko-agent`'s `ProviderAdapter` trait for all dispatch
- [ ] `AuthMethod` maps to `ProviderConfig` from roko.toml schema
- [ ] Single dispatch path for chat, oneshot, orchestration

### R5. Conversation state management

- [ ] `ConversationBuffer` struct: messages, token budget, system prompt
- [ ] Truncation strategy when approaching context limit
- [ ] Persist conversation to `.roko/sessions/<id>.jsonl`
- [ ] `/resume` command to reload a past session

### R6. Demo infrastructure overhaul

The demo system has three layers that are loosely connected:

1. **Web demos** (`demo/demo-web/`) — HTML+JS, talks to serve via WebSocket PTY
2. **Shell scripts** (`demo/demo-resources/`) — 20+ scripts, each with own setup
3. **Rust demo** (`demo_cmd.rs`) — `roko bench demo` command

Problems:
- [ ] No shared setup/teardown — each script reinvents workspace init
- [ ] No shared config — different `roko.toml` files in different dirs
- [ ] Web demos assume serve is already running on :6677
- [ ] Shell scripts assume repo root as CWD
- [ ] No CI integration — demos aren't tested in CI

Proposed fix:
- [ ] **`DemoWorkspace` struct** — creates `/tmp/roko-demo-{name}`, copies
  config, runs `roko init`, provides cleanup on Drop
- [ ] **`demo/shared/setup.sh`** — common shell preamble: find roko, create
  clean dir, copy config, trap cleanup on EXIT
- [ ] **All scripts source `setup.sh`** instead of rolling their own
- [ ] **`roko demo` subcommand** — replace `roko bench demo` with a proper
  demo runner that creates workspace, runs demo, shows results
- [ ] **CI job** — `roko demo --check` that runs all demos in headless mode

### R7. Input system generalization

The `InputState` in `chat_inline.rs` is a reimplementation of readline.
As features accumulate (tab completion, multi-line, Shift+Enter, kill ring),
it becomes a maintenance burden.

Options:
- [ ] **Extract to `crates/roko-cli/src/inline/input.rs`** — separate module
  with proper tests, reusable across chat_inline and future TUI panels
- [ ] **Use `rustyline` or `reedline`** — mature readline crates with tab
  completion, syntax highlighting, vi/emacs modes built in. Tradeoff:
  harder to integrate with ratatui's raw terminal mode.
- [ ] **At minimum: unify completion infrastructure** — `Completer` trait
  with implementations for commands, file paths, model names, agent names

### R1. Service dependency graph

- [ ] Define `Service` trait: `start()`, `shutdown()`, `name()`
- [ ] `ServiceManager` tracks handles + child PIDs
- [ ] `shutdown_all()` in reverse dependency order
- [ ] Each service gets `CancellationToken`

### R2. Log isolation architecture

- [ ] Rule: zero `eprintln!` from library crates
- [ ] All status → `StateHub` events
- [ ] Subprocess output → `.roko/logs/<name>.log`
- [ ] CI lint: `grep -rn 'eprintln!' crates/` should only match tests

### R3. Process tree management

- [ ] Store all `Child` handles in `ProcessRegistry` on `AppState`
- [ ] `shutdown()` sends SIGTERM → wait 2s → SIGKILL
- [ ] `atexit` handler kills children on panic
- [ ] Reuse `ProcessSupervisor` from `roko-runtime`

### R4. Unified provider abstraction

- [ ] Delete `dispatch_direct.rs` entirely
- [ ] Use `roko-agent`'s `ProviderAdapter` trait for all dispatch
- [ ] `AuthMethod` maps to `ProviderConfig` from roko.toml schema
- [ ] Single dispatch path for chat, oneshot, orchestration

### R5. Conversation state management

- [ ] `ConversationBuffer` struct: messages, token budget, system prompt
- [ ] Truncation strategy when approaching context limit
- [ ] Persist conversation to `.roko/sessions/<id>.jsonl`
- [ ] `/resume` command to reload a past session

---

## Priority Order (Updated 2026-04-27 evening)

| # | Issue | Status | Impact |
|---|---|---|---|
| 1 | ~~Issue 1~~ Chain-watcher logs | **DONE** | Was blocking all chat use |
| 2 | ~~Issue 6~~ Auth detection order | **DONE** | Was blocking GLM dispatch |
| 3 | ~~Issue 20~~ Demo binary paths | **DONE** | Demos couldn't run |
| 4 | ~~Tab completion~~ Slash commands | **DONE** | Tab/Shift+Tab cycle |
| 5 | Issue 16 Multi-turn context | Open | #1 UX gap vs Claude Code |
| 6 | Issue 15 Streaming | Open | UI feels broken/frozen |
| 7 | Issue 14 dispatch_direct duplication | Open | Maintenance burden, inconsistency |
| 8 | Issue 17 Cost tracking | Open | Always shows $0, misleading |
| 9 | Issue 3 Ctrl+C cleanup | Open | Orphaned processes |
| 10 | Issue 7 Cargo warnings | Open | Noisy dev experience |
| 11 | Issues 8-13 Hardcoded values | Open | Fragile, not configurable |
| 12 | Issue 19 Config integration | Open | Config exists but unused |
| 13 | Issue 21 Demo clean dirs | Partial | Only terminal.html fixed |
| 14 | Section E remaining features | Open | Design vs reality gap |
| 15 | R1-R7 Refactoring | Open | Structural debt |

### Completed today (2026-04-27)

1. Chain-watcher stderr → `.roko/chain-watcher.log` + `ROKO_LOG=warn`
2. Auth detection reordered: API keys first, Claude CLI last
3. GLM 5.1 as default model (`roko.toml` + `auth_detect.rs`)
4. `/model` and `/provider` slash commands
5. Tab/Shift+Tab completion for all slash commands
6. Demo binary resolution: runtime shell probe → PATH → local builds
7. Demo workspace: clean `/tmp/roko-demo` directory
8. Demo command sequencing: wait for prompt between commands
9. Demo pause/resume button
10. `tui_mode` extended to unified chat (tracing → file)
11. `--no-serve` flag for disabling background serve
12. Claude CLI dispatch fixed: prompt via stdin, stderr captured
13. Comprehensive issue catalog: 24 issues, 7 refactoring proposals
| 7 | Issue 3 Ctrl+C cleanup | Open | Orphaned processes |
| 8 | Issue 7 Cargo warnings | Open | Noisy dev experience |
| 9 | Issues 8-13 Hardcoded values | Open | Fragile, not configurable |
| 10 | Issue 19 Config integration | Open | Config exists but unused |
| 11 | Section E features | Open | Design vs reality gap |
| 12 | R1-R5 Refactoring | Open | Structural debt |
