# Default-Build Architecture Map (2026-05-01)

This is the "ground truth" wiring map for the **default** `roko-cli` build. Every batch in this runner targets one of the live paths below. If a fix tempts you into a path marked **legacy / inactive**, stop and re-read the prompt.

## Build features

`crates/roko-cli/Cargo.toml`:

```
[features]
default = []
legacy-orchestrate = []
```

`crates/roko-cli/src/lib.rs:64-66, 85-86, 136-137`:

```
#[cfg(feature = "legacy-orchestrate")]
pub mod orchestrate;
#[cfg(feature = "legacy-orchestrate")]
pub mod dispatch_direct;
```

Same flag gates portions of `run.rs` (legacy `dispatch_agent`, legacy `run_once`, legacy `build_system_prompt`, etc.).

## Live entry points

### `roko chat` (interactive)

```
main.rs cmd_chat
  → unified.rs cmd_unified_chat
    → spawn_background_serve()  (roko-serve)
    → chat_inline::run_unified_inline
      → build_unified_inline_agent_session
        → ChatAgentSession  (chat_session.rs)
          → send_turn_api / send_turn_streaming  (provider/openai_compat_backend.rs)
          → or Claude CLI via dispatch_claude_cli  (claude_cli_agent.rs)
```

Live state-of-the-world for chat:

- `chat_session.rs:328` — `api_history: Vec<ApiMessage>` (conversation memory)
- `chat_session.rs:575-577` — `system_prompt: Option<String>` (set on `/system`)
- `chat_session.rs:585` — `knowledge_ids: Vec<String>` (currently always empty — BI_16)
- `chat_session.rs:631-633` — `mcp_config` chained into `caller`
- `chat_session.rs:851-854` — `effort` consumed by builder
- `chat_session.rs:1411-1430` — `build_chat_system_prompt` pulls workspace context

### `roko run <prompt>` (single-shot)

```
main.rs cmd_run
  → run.rs run_workflow_engine_with_services    (NOT the legacy run_once)
    → workflow_engine.rs WorkflowEngine::run
      → effect_driver.rs apply_effect
        → ModelCallService::call (model_call_service.rs)
          → ProviderCallCell::execute (rebuilds agent — BI_55)
    → run.rs print_workflow_run_report          (post-hoc — BI_25 streams instead)
```

### `roko plan run <dir>`

```
commands/plan.rs cmd_plan_run
  → RunConfig { dangerously_skip_permissions: true, ... }   (line 402 — BI_04)
  → WorkflowEngine::run
  → events emitted via WorkflowEngine::emit (workflow_engine.rs:179-183, 346-348, 538-544)
  → currently consumed only by post-hoc report — BI_24 forwards inline
```

### `roko serve` / `roko unified` (HTTP API)

```
main.rs cmd_serve / cmd_unified
  → roko-serve lib.rs start_background
    → routes/mod.rs build_router
      → middleware (auth, CORS) — see BI_SEC notes below
      → terminal routes (require_api_key conditional — BI_03 caps sessions)
      → shared_runs routes (`GET /api/shared/{token}` — BI_15 fixes Share.tsx mismatch)
      → bench, dream, knowledge, gateway routes
    → state.rs StateHub                         (separate from TUI hub — BI_13)
```

## Live secondary subsystems

| Concern | Crate / file | Notes |
|---------|--------------|-------|
| Cost tracking | `roko-learn/src/cost_table.rs` | `lookup`, `with_defaults`, `blended_cost_per_m` — BI_42, BI_43 |
| Episode logging | `roko-learn/src/episode_logger.rs` | `compact()` exists, no production caller — BI_08 |
| Cascade router | `roko-learn/src/cascade_router.rs` + `cascade/persistence.rs` | Snapshot misses LinUCB arms — BI_10 |
| Dream triggers | `roko-runtime/src/runtime_feedback/dreams.rs` + `commands/plan.rs:394-396` | Sink without runner — BI_09 |
| Affect | `roko-runtime/src/workflow_engine.rs:590-593` | `let _ = policy.persist().await` — BI_33 |
| MCP client | `roko-agent/src/mcp/client.rs:167-187` | `Stdio::inherit()` for stderr — BI_27 |
| FS watcher | `roko-serve/src/fswatcher.rs:27` | JoinHandle dropped — BI_36 |
| EventBus FIXME | `roko-serve/src/lib.rs:1448-1449` | Double REST delivery — BI_37 |
| Web search | `roko-std/src/tool/builtin/web_search.rs:32-35` | `PERPLEXITY_API_URL`, `DEFAULT_MODEL` const — BI_44 |
| PID registry | `roko-agent/src/process/registry.rs:24-27` | `current_dir().join(".roko/...")` — BI_45 |
| Anthropic defaults | `roko-agent/src/claude_agent.rs:29, 32` | `DEFAULT_BASE_URL`, `DEFAULT_ANTHROPIC_VERSION` — BI_39, BI_40 |
| Naive cost | `chat_inline.rs:4240-4241` | `15.0`, `75.0` — BI_42 |
| Auth detection | `auth_detect.rs:70-75` | `Command::output()` no timeout — BI_26 |
| Audit signals lock | `dispatcher/mod.rs:779,786-787` | `expect("audit signals lock")` — BI_50 |
| LRU cache mutex | `model_call_service.rs:921-960` | `expect("... mutex poisoned")` — BI_51 |
| Feed register | `routes/feeds.rs:125-127` | `expect("just registered")` — BI_52 |
| Lint suppress | `roko-agent/src/lib.rs:22-42` | crate-level allows — BI_53 |

## Live slash commands (where the fixes go)

`crates/roko-cli/src/chat_inline.rs` — `handle_slash_command(cmd, session, term, theme)`:

| Command | Current line | Fix in batch |
|---------|--------------|--------------|
| `/system <text>` | ~2672-2689 | (already wired — verified FIXED) |
| `/effort <level>` | ~2898-2908 | (already wired — verified FIXED) |
| `/gate ...` | ~2950-2987 | BI_17 |
| `/config set <k> <v>` | ~3124-3143 | BI_18 |
| `/run <prompt>` | ~3390-3398 | BI_19 |
| `/plan run <dir>` | ~3427-3435 | BI_20 |
| `/prd idea <text>` | ~3457-3465 | BI_21 |
| `/research <query>` | ~3488 | BI_22 |

The handler is one giant `match` — keep additions inside that match arm. Don't refactor the match into a registry as part of these batches (that's an `RP_` post-parity concern).

## Cross-cutting helpers worth knowing

- `crates/roko-runtime/src/workflow_engine.rs` — `WorkflowEngine::emit` is the existing event-fan-out point. Use it for streaming.
- `crates/roko-cli/src/chat_inline.rs` — `term.push_lines(&[...])` is how the TUI gets text. Use `styled::section_start / continuation / section_end` for consistent rendering.
- `crates/roko-agent/src/provider/mod.rs:94-99` — `shared_http_client()` exists, returns `Arc<ReqwestPoster>` with `connect_timeout(10)`. Anything making a fresh `reqwest::Client::new()` should clone this instead.
- `crates/roko-runtime/src/cancel.rs` — workspace cancellation primitives (used by post-parity XC_04 work; this runner uses what's already there).
