# Safety Layer Issues

## Critical

### Claude CLI bypasses entire SafetyLayer per-tool
- `tool_loop/mod.rs:1-10`: Module comment: "Claude CLI drives its own internal loop and bypasses this entirely."
- ToolDispatcher → SafetyLayer (9 policies) only invoked by OpenAI-compat ToolLoop.
- Claude CLI (default) spawns subprocess with own tool loop. Roko's SafetyLayer is NEVER called.
- Compensating control: Python hook in `claude_cli_agent.rs:36-72` — only blocks ~5 git commands and `rm -rf`.

### `--dangerously-skip-permissions` on by default
- `claude_cli_agent.rs:120`: `dangerously_skip_permissions: true` for every `ClaudeCliAgent::new()`.
- Only toggled to `false` for Auditor/Strategist roles via `orchestrate.rs:19889-19892`.
- Claude can write files, execute shell, make network calls without confirmation.

### Post-dispatch violations are warn-only, never block
- `orchestrate.rs:17100-17120`, `safety/mod.rs:749-821`: `SecretLeak` and `PathEscape` violations assigned `Warn` severity. Pipeline continues. No task failure path.
- Path-escape detection is also weak: only checks `.contains("..")` or `.starts_with('/')`.

## High

### SpendingLimiter/SafetyBudget never wired at runtime
- `spending.rs:58`: `reject_on_critical: false` by default.
- `safety/mod.rs:259`: `safety_budget: None` in default construction.
- Budget is only checked AFTER task completes (`orchestrate.rs:17138`). Cannot pre-block mid-turn.

### Contract enforcement cleared for TOML-configured roles
- `safety/mod.rs:949-956`: Any role with TOML entry but no YAML contract gets `allowed_tools = None` (fully open), bypassing deny-all fallback.

## Medium

### Bash denylist gaps
- `safety/bash.rs:95-128`: Blocks `rm -rf /` but NOT `rm -rf /etc/`, `chmod 777`, `wget | python`, `eval $VAR`.
- Claude CLI hook: regex misses `rm -r /etc` (the `r` flag without `f`).

### Symlink escape not denied by default
- `safety/path.rs:80-87`: `deny_symlinks: false` default. And PathPolicy only runs in ToolLoop path, never for Claude CLI.

### `cost_usd` is f32 — accumulation precision loss
- `chat_types.rs:121`: Many small f32 costs accumulated into f64 budget ceiling = rounding errors.
