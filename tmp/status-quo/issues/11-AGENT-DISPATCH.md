# Agent Dispatch Issues

Investigation of `crates/roko-agent/src/dispatcher/` and provider implementations.

## Critical

### Hook chain modified parameters silently dropped
- `dispatcher/mod.rs:415`: `Ok((_params, audit_records))` — modified params from `HookDecision::AllowModified` are discarded. Any hook that sanitizes arguments has no effect.

### No retry on 429/5xx despite RetryPolicy existing
- `openai_agent.rs:~179` converts 429 directly to failure, no backoff.
- `claude_cli_agent.rs:~635` same pattern.
- `retry.rs` defines `RetryPolicy::for_rate_limit()` with proper exponential backoff — but it's only wired at `tool_loop` layer, not `Agent::run()`.

### Safety hook silently no-ops without python3
- `claude_cli_agent.rs:43-66`: Hook starts with `command -v python3 >/dev/null 2>&1 || exit 0`. On systems without Python, the entire safety hook is disabled.
- Pattern coverage is narrow: blocks `git checkout/switch/push`, `rm -rf` but NOT `git reset --hard`, `git clean -fd`, `git branch -D`.

## High

### Stderr fallback as agent output
- `claude_cli_agent.rs:829-836`: If stdout is empty, stderr is used as the agent's response. Error messages and noise become the "answer".

### OpenAiAgent ignores system prompt and has no tool-use support
- `openai_agent.rs:156-161`: Only a single `user` message is sent. System prompt silently dropped.
- `supports_streaming()` returns `false`. No tool-call processing.

### ToolResultCache, DedupCache, ToolFailureMonitor never instantiated
- `dispatcher/mod.rs:114,137`: `tool_cache` defaults to `None`. No call site passes `with_tool_cache()` or `with_dedup_cache()`. These subsystems are completely inert.

## Medium

### Path prefix collision in cache invalidation
- `dispatcher/result_cache.rs:315-317`: `paths_overlap` uses `starts_with` without separator check. `/tmp/t` invalidates `/tmp/test.rs`.

### DefaultHasher used for security-sensitive dedup/audit hashes
- `dedup_cache.rs:227,241`, `result_cache.rs:280`, `hook_chain.rs:147`: Not cryptographic, not stable across runs. Audit records can collide.

### Hardcoded timeouts
- `claude_cli_agent.rs:121`, `openai_agent.rs:69`: `DEFAULT_REQUEST_TIMEOUT_MS` (120s). Long-running tasks time out.

### No backpressure on parallel tool calls
- `dispatcher/mod.rs:528-534`: `buffer_unordered(DEFAULT_MAX_CONCURRENT_TOOLS)` with no rate limiter.
