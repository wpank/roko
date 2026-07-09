# 10 — Hardcoded Values Reference

Every hardcoded model string, API URL, token limit, timeout, and env var
in roko's runtime code paths with exact file:line locations.

---

## Hardcoded Model Strings (Runtime-Critical)

| File | Line | Value | Context |
|------|------|-------|---------|
| `crates/roko-cli/src/run.rs` | 530 | `"claude-sonnet-4-6"` | Claude CLI fallback when no routing config |
| `crates/roko-cli/src/run.rs` | 657 | `"llama3.1:8b"` | Ollama default model |
| `crates/roko-cli/src/run.rs` | ~1107 | `"claude-sonnet-4-6"` | `resolved_model()` final fallback |
| `crates/roko-cli/src/dispatch_direct.rs` | 208 | `"claude-sonnet-4-6-20250514"` | Anthropic API default |
| `crates/roko-cli/src/dispatch_direct.rs` | 291 | `"gpt-4o"` | OpenAI-compat default |
| `crates/roko-cli/src/auth_detect.rs` | 42 | `"claude-sonnet-4-6"` | Default model for AnthropicApi auth |
| `crates/roko-cli/src/auth_detect.rs` | 135 | `"claude-sonnet-4-6"` | Display label |
| `crates/roko-cli/src/orchestrate.rs` | 13987 | `"claude-sonnet-4-6"` | Task dispatch fallback |
| `crates/roko-cli/src/orchestrate.rs` | 13998 | `"claude-sonnet-4-6"` | Tier routing fallback |
| `crates/roko-cli/src/orchestrate.rs` | 14020 | `"claude-opus-4-6"` | Generic task fallback |
| `crates/roko-cli/src/orchestrate.rs` | 15110 | `"claude-haiku-4-5"` | Fallback model for --fallback-model |

## Hardcoded Model Strings (Test/Learning — Not Runtime-Critical)

| File | Value | Context |
|------|-------|---------|
| `roko-primitives/src/tier.rs` | `claude-haiku-4-5`, `claude-opus-4-6`, `claude-sonnet-4` | Tier routing matrix |
| `roko-conductor/src/watchers/` | `claude-sonnet-4-6`, `claude-opus-4-6`, `claude-haiku-4-5` | Synthetic test signals |
| `roko-dreams/src/` | `claude-haiku-4-5`, `claude-opus-4-6` | Dream consolidation |
| `roko-neuro/src/distiller.rs` | `claude-haiku-3-5`, `claude-sonnet-4-5` | Distillation |
| `roko-compose/src/enrichment/estimate.rs` | `gpt-4o`, `gpt-4o-mini`, `gpt-5`, etc. | Cost estimation |
| `roko-agent/tests/` | Various | Integration tests |
| `roko-demo/src/scenarios/` | `claude-sonnet-4-20250514` | Demo scenarios |
| `roko-acp/src/session.rs` | `glm-5.1`, `kimi-k2.5`, etc. | ACP session tests |

---

## Hardcoded Max Tokens

| File | Line | Value | Context |
|------|------|-------|---------|
| `crates/roko-cli/src/dispatch_direct.rs` | 212 | **8192** | Anthropic API direct |
| `crates/roko-cli/src/dispatch_direct.rs` | 296 | **8192** | OpenAI-compat direct |
| `crates/roko-agent/src/provider/anthropic_api/tool_loop.rs` | 242 | **4096** | Anthropic API via agent |
| `crates/roko-core/src/config/agent.rs` | 259 | **4096** | Default data_llm config |
| `crates/roko-serve/src/routes/gateway.rs` | 991 | **1024** | HTTP gateway |
| `crates/roko-neuro/src/distiller.rs` | 26 | **2048** | Knowledge distillation |
| `crates/roko-demo/src/scenarios/llm.rs` | 257 | **512** | Demo scenario |

---

## Hardcoded API URLs

| File | Line | URL | Context |
|------|------|-----|---------|
| `crates/roko-cli/src/dispatch_direct.rs` | 217 | `https://api.anthropic.com/v1/messages` | Anthropic API |
| `crates/roko-cli/src/auth_detect.rs` | 73 | `https://open.bigmodel.cn/api/paas/v4` | ZAI/GLM |
| `crates/roko-cli/src/auth_detect.rs` | 93 | `https://api.openai.com/v1` | OpenAI default |
| `crates/roko-serve/src/routes/providers.rs` | 710, 909, 1076, 1167 | `https://api.z.ai/api/paas/v4` | ZAI routes |
| `crates/roko-serve/src/routes/secrets.rs` | 184 | `https://api.anthropic.com` | Secret validation |
| `crates/roko-serve/src/routes/secrets.rs` | 208 | `https://api.openai.com/v1` | Secret validation |
| `crates/roko-agent/src/openai_compat_backend.rs` | 20 | `https://api.openai.com/v1` | Backend default |
| `crates/roko-agent/src/provider/openai_compat.rs` | 205, 216 | `https://api.openai.com/v1` | Provider default |
| `crates/roko-agent/src/ollama/agent.rs` | 33 | `http://localhost:11434` | Ollama default |
| `crates/roko-agent/src/perplexity/adapter.rs` | 30 | `https://api.perplexity.ai` | Perplexity |
| `crates/roko-agent/src/provider/openai_compat.rs` | 1198, 1251 | `https://openrouter.ai/api/v1` | OpenRouter |
| `crates/roko-agent/src/translate/openai.rs` | 314, 338 | `https://api.z.ai/api/mcp/zread/mcp` | ZAI MCP |
| `crates/roko-agent/src/gemini/native.rs` | 559, 604, 642, 775, 812 | `https://generativelanguage.googleapis.com` | Gemini |

---

## Hardcoded API Version

| File | Line | Value | Context |
|------|------|-------|---------|
| `crates/roko-cli/src/dispatch_direct.rs` | 219 | `"2023-06-01"` | Anthropic API version header |
| `crates/roko-agent/src/provider/anthropic_api.rs` | (constant) | `DEFAULT_ANTHROPIC_VERSION` | Agent crate constant |

dispatch_direct.rs hardcodes the version string instead of importing
`DEFAULT_ANTHROPIC_VERSION` from roko-agent.

---

## Hardcoded Timeouts

| File | Line | Value | Context |
|------|------|-------|---------|
| `crates/roko-cli/src/chat_inline.rs` | (tick rate) | **33ms** | Tick interval |
| `crates/roko-cli/src/chat_inline.rs` | 3211 | **120s** | HTTP request timeout |
| `crates/roko-cli/src/chat_inline.rs` | (poll) | **500ms** | Polling interval |
| `crates/roko-cli/src/chat_inline.rs` | (poll) | **2s** | Run status poll |
| `crates/roko-cli/src/chat.rs` | (poll) | **120s** | HTTP request timeout |
| `~/.roko/config.toml` | | **120000ms** | Default provider timeout |
| `~/.roko/config.toml` | | **180000ms** | Ollama/ZAI timeout |

---

## Environment Variables Checked

| Var | File | Line | Purpose |
|-----|------|------|---------|
| `ZAI_API_KEY` | `auth_detect.rs` | 67 | Zhipu/GLM auth |
| `ZAI_MODEL` | `auth_detect.rs` | 69 | Zhipu model override |
| `ANTHROPIC_API_KEY` | `auth_detect.rs` | 79 | Anthropic auth |
| `ANTHROPIC_API_KEY` | `neuro/episode_completion.rs` | 25 | Direct read (bypasses config) |
| `OPENAI_API_KEY` | `auth_detect.rs` | 86 | OpenAI auth |
| `OPENAI_API_BASE` | `auth_detect.rs` | 88 | OpenAI base URL override |
| `OPENAI_BASE_URL` | `auth_detect.rs` | 90 | OpenAI base URL override (alt) |
| `PERPLEXITY_API_KEY` | `roko-std/web_search.rs` | | Direct read (bypasses config) |
| `GEMINI_API_KEY` | `roko-agent/gemini/` | | Gemini auth |
| `MOONSHOT_API_KEY` | `roko-agent/tests/` | | Moonshot auth |
| `OPENROUTER_API_KEY` | `roko-agent/provider/` | | OpenRouter auth |
| `ROKO_LOG` | `main.rs` | | Log level override |
| `ROKO_LOG_RAW` | `main.rs` | | Raw log output |
| `ROKO_TIMING` | `main.rs` | | Timing output |
| `ROKO_EFFORT` | `main.rs` | | Effort override |
| `ROKO_LOG_FORMAT` | `main.rs` | | Log format override |

---

## CLI Flags Passed to Subprocesses

### Claude CLI flags used

| Flag | File(s) | Applied In |
|------|---------|-----------|
| `--print` | dispatch_direct.rs:51, run.rs | All Claude CLI paths |
| `--output-format stream-json` | dispatch_direct.rs:51, run.rs, bridge_events.rs | All Claude CLI paths |
| `--model <slug>` | run.rs, orchestrate.rs, claude_cli_agent.rs, dispatch_v2.rs | Multiple paths |
| `--fallback-model <slug>` | run.rs:541, orchestrate.rs:15110, dreams/runner.rs | Some paths only |
| `--tools <csv>` | claude_cli_agent.rs:334 | Agent layer only |
| `--resume <session_id>` | orchestrate.rs:1546, 1607, 15113 | Orchestrate only |
| `--bare` | run.rs (config), orchestrate.rs | Some paths |
| `--mcp-config <path>` | run.rs (config), orchestrate.rs | Some paths |
| `--dangerously-skip-permissions` | run.rs, orchestrate.rs | Some paths |
| `--append-system-prompt` | orchestrate.rs | Orchestrate only |
| `--effort <level>` | Not found in dispatch_direct.rs | Missing from chat path |
