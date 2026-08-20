# 52 — MCP Stderr Capture and CostTable Gaps

**Priority**: P2 — subprocess hygiene / cost accuracy: MCP logs pollute the terminal; missing model pricing biases the cascade router
**Size**: S (1 day)
**Crates**: `crates/roko-agent` (`src/mcp/client.rs`, `src/task_runner.rs`), `crates/roko-core` (`src/config/provider.rs`)
**Depends on**: None

---

## Background

This item covers two independent issues that both involve leaky defaults.

### Issue 1: MCP stderr inheritance

Every MCP server subprocess spawned by roko inherits the parent process's stderr via `Stdio::inherit()`. MCP servers (including roko's own `roko-mcp-code`, `roko-mcp-scripts`, `roko-mcp-slack`, `roko-mcp-github`, and any user-configured external server) typically initialize their own `tracing_subscriber` and emit `DEBUG`/`INFO`/`WARN` log lines. These leak directly into the user's terminal.

The pollution is especially disruptive during `roko chat` (interactive REPL) and `roko run` (plan execution with user-visible progress output). A single MCP server can emit dozens of log lines per request.

The fix is to redirect stderr to a file in `.roko/logs/mcp-{name}.log` instead of inheriting. If the log file cannot be opened, silently discard the output rather than crashing or falling back to inherit.

### Issue 2: CostTable model pricing gaps

The `CostTable` in `crates/roko-agent/src/task_runner.rs` hardcodes pricing for 9 Claude, GLM, Kimi, and GPT models. Gemini, Perplexity, Cerebras, and Ollama all fall through to the `SONNET_FALLBACK` (Sonnet 4.6 rates: $3/$15 per million tokens) when tokens are reported, or return `$0.00` when no tokens are used.

The fallback rate is wrong for most other providers: Gemini 2.5 Pro uses different rates, Cerebras charges per-token at different rates, and Ollama is free (local inference). Incorrect cost data flows into the cascade router's `CostSummary`, causing the router's cost-efficiency scoring to be wrong. Over time, this biases the router away from or toward certain providers based on phantom costs, degrading learned routing quality.

Users can override pricing via `cost_input_per_m` / `cost_output_per_m` fields on `ModelProfile` in their `roko.toml`, but these fields are undocumented in the CLI output.

## Current State

### MCP stderr

1. `StdioTransport::spawn_with_env()` at `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/mcp/client.rs:218` builds a `tokio::process::Command` and sets `.stderr(std::process::Stdio::inherit())` at line 229. The stdin and stdout are already piped for JSON-RPC communication.

2. `StdioTransport` stores `_child: Mutex<Child>` (line 202) with `kill_on_drop(true)`. The child handle is never explicitly waited on — it is dropped when the transport is dropped.

3. A log rotation implementation exists at `crates/roko-fs/src/log_rotation.rs`. It does size-based rotation for JSONL files. However, for MCP stderr we want day-based rotation, not size-based rotation (MCP servers can go quiet for long periods). The existing `rotate_if_needed()` function takes a `max_mb` threshold and is JSONL-specific. We need either a simpler date-stamped file approach or to extend the rotation module.

4. There is no `mcp` subdirectory under `.roko/logs/` currently. The `.roko/` directory is managed by `crates/roko-fs/src/layout.rs` (the `RokoLayout` struct). Check whether `.roko/logs/` is defined there before creating it manually.

### CostTable

5. `KNOWN_MODEL_PRICING` at `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/task_runner.rs:427` has 9 entries: `claude-opus-4-6`, `claude-sonnet-4-6`, `claude-haiku-4-5`, `glm-5.1`, `glm-5`, `kimi-k2.5`, `gpt-5.2`, `gpt-5.4`, `gpt-5.4-mini`.

6. `SONNET_FALLBACK` at line 419 has: `input_per_m: 3.00`, `output_per_m: 15.00`, `cache_read_per_m: 0.30`, `cache_write_per_m: 3.75`. This is used when `self.models.get(model_slug)` returns `None` but `total_tokens > 0`.

7. `CostTable::from_config_with_defaults()` at line 448 first reads `cost_input_per_m` / `cost_output_per_m` from each `ModelProfile` in the config, then merges hardcoded defaults for known models (config takes priority via `or_insert`). The config-based path works today — users just don't know about it.

8. `ModelProfile` fields for pricing are in `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/config/provider.rs:501-518`: `cost_input_per_m`, `cost_output_per_m`, `cost_input_per_m_high`, `cost_output_per_m_high`, `cost_cache_read_per_m`, `cost_cache_write_per_m`. The doc comments are brief (single-line, no source or example).

9. `roko config models list` output in `crates/roko-cli/src/commands/config_cmd.rs:828` shows columns: `Model`, `Provider`, `Slug`, `Key`. Cost fields are not shown.

## Implementation Plan

### MCP stderr (4 steps)

**Step 1: Change stderr from `inherit` to `piped`**

In `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/mcp/client.rs:229`, change:
```rust
.stderr(std::process::Stdio::inherit())
```
to:
```rust
.stderr(std::process::Stdio::piped())
```

Also update the doc comment on `spawn_with_env()` at line 214 to say "Stderr is captured to `.roko/logs/mcp-{server_name}.log`" instead of "Stderr is inherited".

**Step 2: Spawn a background drain task**

The `StdioTransport::spawn_with_env()` function receives `command` and `args` but does not have a server name parameter. Add a `name: &str` parameter (the MCP server config key from the `[mcp.servers]` table):

```rust
pub fn spawn_with_env(
    command: &str,
    args: &[String],
    env: &HashMap<String, String>,
    name: &str,
    logs_dir: Option<&Path>,
) -> Result<Self, McpError>
```

After spawning the child, take `child.stderr` and spawn a background task that reads from it line by line and writes to `.roko/logs/mcp-{name}.log`:

```rust
if let Some(stderr) = child.stderr.take() {
    let log_path = logs_dir
        .map(|d| d.join(format!("mcp-{name}.log")))
        .filter(|_| {
            if let Some(dir) = logs_dir {
                let _ = std::fs::create_dir_all(dir);
                true
            } else {
                false
            }
        });

    tokio::spawn(async move {
        let reader = tokio::io::BufReader::new(stderr);
        let mut lines = tokio::io::AsyncBufReadExt::lines(reader);
        let mut file = log_path.and_then(|p| {
            std::fs::OpenOptions::new().create(true).append(true).open(&p).ok()
        });
        while let Ok(Some(line)) = lines.next_line().await {
            if let Some(ref mut f) = file {
                let _ = writeln!(f, "{line}");
            }
            // If file is None, silently discard.
        }
    });
}
```

If the logs directory is `None` or the file cannot be opened, silently discard. Log a single `tracing::debug!("MCP server {name} stderr capture failed; discarding output")`.

**Step 3: Day-based log rotation**

For simplicity, use a date-stamped file name approach rather than extending the existing JSONL rotator. When opening the log file, use the current UTC date as a suffix: `mcp-{name}-{date}.log` (e.g., `mcp-code-20260819.log`). This gives automatic day-based rotation: each day a new file is created.

To clean up old files: after opening the current day's file, scan the logs directory for files matching `mcp-{name}-*.log` older than 3 days (by file name, since names are date-stamped) and delete them. This logic runs once per MCP server spawn, which is acceptable.

**Step 4: Update call sites**

Find all call sites of `StdioTransport::spawn()` and `StdioTransport::spawn_with_env()` in the codebase (likely in `crates/roko-agent/src/mcp/client.rs` and wherever MCP clients are constructed). Update them to pass the server name and the workspace `.roko/logs/` directory.

The workspace path is available through the config or through the runner context. Check how existing code accesses the workspace path in `crates/roko-agent/src/` to find the right pattern.

### CostTable (3 steps)

**Step 5: Add missing model pricing entries**

In `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/task_runner.rs`, add entries to `KNOWN_MODEL_PRICING` at line 427. Add a comment block above the constant explaining the source and last-verified date:

```rust
/// Hardcoded pricing for well-known models.
///
/// Sources (last verified 2026-08-19):
/// - Gemini: https://ai.google.dev/gemini-api/docs/models/gemini#gemini-2.5-pro
/// - Perplexity: https://docs.perplexity.ai/guides/pricing
/// - Cerebras: https://inference.cerebras.ai/ (free tier as of last check)
///
/// Format: (slug, input_$/M, output_$/M, cache_read_$/M, cache_write_$/M)
const KNOWN_MODEL_PRICING: &[(&str, f64, f64, f64, f64)] = &[
    // Existing entries (unchanged):
    ("claude-opus-4-6", 15.00, 75.00, 3.75, 18.75),
    ("claude-sonnet-4-6", 3.00, 15.00, 0.30, 3.75),
    ("claude-haiku-4-5", 0.80, 4.00, 0.08, 1.00),
    ("glm-5.1", 1.40, 4.40, 0.26, 1.75),
    ("glm-5", 1.00, 3.20, 0.50, 1.25),
    ("kimi-k2.5", 0.60, 3.00, 0.10, 0.75),
    ("gpt-5.2", 2.00, 8.00, 0.50, 2.50),
    ("gpt-5.4", 2.50, 10.00, 0.63, 3.13),
    ("gpt-5.4-mini", 0.40, 1.60, 0.10, 0.50),
    // New Gemini entries:
    ("gemini-2.5-pro", 1.25, 10.00, 0.00, 0.00),
    ("gemini-2.5-flash", 0.15, 0.60, 0.00, 0.00),
    ("gemini-2.0-flash", 0.10, 0.40, 0.00, 0.00),
    // New Perplexity entries (per-token approximation):
    ("sonar-pro", 3.00, 15.00, 0.00, 0.00),
    ("sonar", 1.00, 1.00, 0.00, 0.00),
    // New Cerebras entries (free tier):
    ("llama-4-scout", 0.00, 0.00, 0.00, 0.00),
    ("qwen-3-32b", 0.00, 0.00, 0.00, 0.00),
];
```

**Step 6: Add Ollama/local provider zero-cost shortcut**

In `CostTable::calculate()` at line 488, add a check before the `self.models.get(model_slug)` lookup:

```rust
pub fn calculate(&self, model_slug: &str, usage: &Usage, provider_id: Option<&str>) -> f64 {
    // Local inference is always free — skip the cost table entirely.
    if Self::is_local_provider(provider_id.unwrap_or(""))
        || model_slug.starts_with("ollama/") {
        return 0.0;
    }
    // ... existing logic ...
}

pub fn is_local_provider(provider_id: &str) -> bool {
    matches!(provider_id, "ollama" | "local" | "lm_studio")
}
```

This prevents Ollama models from hitting `SONNET_FALLBACK`. The `provider_id` parameter must be threaded through from the call site — find where `cost_table.calculate()` is called (likely in the task runner's cost accounting path) and add the provider ID argument.

**Step 7: Document `cost_input_per_m` in `roko config models list`**

In `crates/roko-cli/src/commands/config_cmd.rs`, in the `format_models_list_rows()` function at line 828, add a cost column to the output. Also update the `ModelsListRow` struct at line 821 to include a `cost` field:

```rust
struct ModelsListRow {
    model: String,
    provider: String,
    slug: String,
    key_status: String,
    cost: String, // e.g. "$3.00/$15.00 /M" or "from config" or "-"
}
```

Populate it from `profile.cost_input_per_m` and `profile.cost_output_per_m` when available. Update `ModelProfile`'s doc comments in `crates/roko-core/src/config/provider.rs:501-518` to include an example:

```rust
/// Input token cost per million tokens in USD.
/// Override the built-in pricing: `cost_input_per_m = 1.25`
/// Set to 0.0 for local/free models.
pub cost_input_per_m: Option<f64>,
```

## Acceptance Criteria

1. MCP server stderr output does not appear in the user's terminal during `roko chat` or `roko run`.
2. MCP stderr is written to `.roko/logs/mcp-{name}-{date}.log`.
3. MCP log files older than 3 days are deleted on the next MCP server spawn.
4. If log file creation fails, roko continues normally with no error or crash.
5. `KNOWN_MODEL_PRICING` contains entries for at least 3 Gemini models, 2 Perplexity models, and 2 Cerebras models.
6. Ollama models (slug starting with `ollama/` or provider ID `"ollama"`) report zero cost without hitting the Sonnet fallback.
7. The "last verified" comment block above `KNOWN_MODEL_PRICING` includes source URLs and the date.
8. `roko config models list` shows a cost column when `cost_input_per_m` / `cost_output_per_m` are set in config.
9. `ModelProfile` cost fields have updated doc comments with an example.
10. `cargo test -p roko-agent` passes with no regressions.

## Verification Checklist

- [ ] Configure an MCP server in `roko.toml` (e.g. `roko-mcp-code`); run `roko chat`; verify no MCP debug logs appear in the terminal
- [ ] Verify `.roko/logs/mcp-{name}-{date}.log` exists and contains the captured stderr
- [ ] Verify old log files (manually rename to an older date) are deleted on the next spawn
- [ ] Add `[models.gemini-test] provider = "gemini_api" slug = "gemini-2.5-pro"` to `roko.toml`; verify `roko config models list` shows cost columns
- [ ] Run a task with Gemini provider; verify the cost in the session output uses $1.25/$10.00 rates, not the Sonnet fallback
- [ ] Run a task with an Ollama provider; verify cost is $0.00
- [ ] Run `cargo test -p roko-agent` — all tests pass

## Files to Modify

| File | Change |
|---|---|
| `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/mcp/client.rs` | Change `.stderr(Stdio::inherit())` to `.stderr(Stdio::piped())` at line 229; add `name: &str` and `logs_dir: Option<&Path>` parameters to `spawn_with_env()`; add background drain task; update `spawn()` to pass defaults; update doc comment at line 209 |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/task_runner.rs` | Add Gemini, Perplexity, Cerebras entries to `KNOWN_MODEL_PRICING` (line 427); add "last verified" comment block; add `is_local_provider()` method; add `provider_id` parameter to `calculate()` and update call site |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/config/provider.rs` | Update doc comments on `cost_input_per_m` and `cost_output_per_m` (lines 501-506) with examples |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/config_cmd.rs` | Add cost column to `ModelsListRow` (line 821) and `format_models_list_rows()` (line 828) |
