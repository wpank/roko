# MCP Stderr Capture and CostTable Gaps

**Priority**: P2 — subprocess hygiene / cost accuracy
**Size**: S (1 day)
**Crate**: `crates/roko-agent/src/mcp/client.rs`, `crates/roko-agent/src/task_runner.rs`

---

## Problem

Two independent subprocess/accounting issues that share a common theme of leaky
defaults:

### Issue 1: MCP stderr inherit

Every MCP server spawned by roko inherits the parent process's stderr via
`Stdio::inherit()`. MCP servers typically initialize their own `tracing_subscriber`
and emit debug/info/warn logs that leak into the user's terminal output. This pollutes
chat sessions, plan runs, and any other operation that triggers MCP tool calls.

Affected MCP servers include `roko-mcp-code`, `roko-mcp-scripts`, `roko-mcp-slack`,
`roko-mcp-github`, and any external MCP server a user configures. The noise is
especially bad during `roko run` and `roko chat` where the user expects clean output.

### Issue 2: CostTable pricing gaps

The `CostTable` hardcodes pricing for Claude (Opus, Sonnet, Haiku), GLM, Kimi, and GPT
models. Gemini, Perplexity, Cerebras, and Ollama all fall through to the Sonnet-rate
fallback (`$3/$15 per million tokens`) when tokens are reported, or report `$0.00` when
tokens are zero.

The Sonnet fallback is inaccurate for most of these providers:
- Gemini 2.5 Pro is priced differently from Sonnet
- Perplexity uses per-request pricing, not per-token
- Cerebras charges per-token but at different rates
- Ollama is genuinely free (local inference)

This distorts cascade router learning: providers with inaccurate cost data get wrong
cost-efficiency scores, which biases model selection away from or toward them
incorrectly.

---

## Section A: Current State

**A1.** MCP client spawn at `crates/roko-agent/src/mcp/client.rs` line 229:
```rust
.stderr(std::process::Stdio::inherit())
```
This is in the `tokio::process::Command` builder for MCP server subprocesses.

**A2.** `CostTable` at `crates/roko-agent/src/task_runner.rs` line 413 is a
`HashMap<String, ModelPricing>` keyed by model slug.

**A3.** `KNOWN_MODEL_PRICING` at line 427 contains 9 entries:
- `claude-opus-4-6` (15/75)
- `claude-sonnet-4-6` (3/15)
- `claude-haiku-4-5` (0.80/4)
- `glm-5.1` (1.40/4.40)
- `glm-5` (1/3.20)
- `kimi-k2.5` (0.60/3)
- `gpt-5.2` (2/8)
- `gpt-5.4` (2.50/10)
- `gpt-5.4-mini` (0.40/1.60)

Missing: all Gemini models, all Perplexity models, all Cerebras models, Ollama.

**A4.** `SONNET_FALLBACK` at line 419 is used when a model slug is unknown but
tokens > 0. This is a reasonable fallback for unknown Claude models but wrong for
other provider families.

**A5.** `CostTable::from_config_with_defaults()` at line 448 merges config-supplied
pricing (from `ModelProfile` entries in `[models]` config) with the hardcoded defaults.
Config-supplied pricing takes priority. This means users *can* override pricing today
via config, but no one does because the config fields are undocumented and
non-obvious.

**A6.** The `ModelProfile` struct (search in `crates/roko-core/src/config/model_registry.rs`)
has `cost_input_per_m: Option<f64>` and `cost_output_per_m: Option<f64>` fields that
feed into the config-based cost table construction.

---

## Section B: What To Do

### MCP stderr

**B1.** Change the MCP client spawn at `crates/roko-agent/src/mcp/client.rs` line 229
from `.stderr(Stdio::inherit())` to `.stderr(Stdio::piped())`.

**B2.** Spawn a background task that reads the piped stderr and routes it to a log file
at `.roko/logs/mcp-{server_name}.log`. Use the server name from the MCP config (the
key in the `[mcp.servers]` table). Create the `.roko/logs/` directory if it does not
exist.

**B3.** Apply day-based rotation to MCP stderr logs: keep at most 3 days of logs per
server, matching the pattern used by the chain-watcher log rotation. Search
`crates/roko-fs/src/log_rotation.rs` for the existing rotation implementation.

**B4.** If the piped stderr read errors or the log file cannot be opened, silently
discard the stderr output rather than crashing or falling back to inherit. Log a
single `tracing::debug!` noting the discard.

### CostTable

**B5.** Add `KNOWN_MODEL_PRICING` entries for the models actually used by the
codebase's provider adapters. Source pricing from provider documentation:

| Model slug | Input $/M | Output $/M | Notes |
|---|---|---|---|
| `gemini-2.5-pro` | 1.25 | 10.00 | Google AI Studio pricing |
| `gemini-2.5-flash` | 0.15 | 0.60 | Google AI Studio pricing |
| `gemini-2.0-flash` | 0.10 | 0.40 | Google AI Studio pricing |
| `sonar-pro` | 3.00 | 15.00 | Perplexity per-token pricing |
| `sonar` | 1.00 | 1.00 | Perplexity per-token pricing |
| `llama-4-scout` | 0.00 | 0.00 | Cerebras free tier |
| `qwen-3-32b` | 0.00 | 0.00 | Cerebras free tier |

Set `cache_read_per_m` and `cache_write_per_m` to zero for providers that do not
support prompt caching.

**B6.** For Ollama models, add a wildcard-style fallback: if the model slug starts
with `ollama/` or the provider ID is `"ollama"`, return zero cost without falling
through to the Sonnet fallback. Add a method like
`CostTable::is_local_provider(provider_id: &str) -> bool` that returns true for
`"ollama"` and `"local"`.

**B7.** Add a comment block above `KNOWN_MODEL_PRICING` with a "last verified" date
and a link to each provider's pricing page, so future maintainers know where to
check for updates.

**B8.** Document the `cost_input_per_m` / `cost_output_per_m` config fields in the
`ModelProfile` doc comment and in `roko config models list` output, so users know
they can override pricing.

---

## Acceptance criteria

- [ ] MCP server stderr is piped to `.roko/logs/mcp-{name}.log`, not to the parent terminal
- [ ] MCP stderr logs rotate with a 3-day retention
- [ ] Stderr pipe failures are silently discarded with a debug-level log
- [ ] `KNOWN_MODEL_PRICING` has entries for Gemini (3 models), Perplexity (2), Cerebras (2)
- [ ] Ollama models report zero cost without hitting the Sonnet fallback
- [ ] Pricing entries have a "last verified" comment with source URLs
- [ ] `cost_input_per_m` / `cost_output_per_m` config fields are documented
- [ ] `cargo test -p roko-agent` passes with no regressions
- [ ] Manual verification: run `roko chat` with an MCP server configured — no MCP debug output appears in the terminal

### Not in scope
- Real-time pricing API integration
- Billing or invoicing features
- Cost alerts or budget warnings (already handled by `BudgetGuardrail`)
- Changing the Sonnet fallback for genuinely unknown models
- Per-request pricing support for Perplexity (approximated as per-token)
