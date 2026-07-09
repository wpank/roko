# Environment Variables Reference

## Required for Production

### LLM Provider Keys

Set these for providers you actually use. **Only providers with keys will be available for routing.**

| Variable | Provider | Models | Required? |
|---|---|---|---|
| `ANTHROPIC_API_KEY` | Anthropic | claude-sonnet-4-6, claude-opus-4-6, claude-haiku-4-5 | **Yes** (default provider) |
| `OPENAI_API_KEY` | OpenAI | gpt-5.4, gpt-5-mini, o3, o4-mini | If using OpenAI models |
| `PERPLEXITY_API_KEY` | Perplexity | sonar-deep-research, sonar-reasoning-pro | If using web search / research |
| `GEMINI_API_KEY` | Google | gemini-2.5-pro, gemini-2.5-flash | If using Gemini models |

### Server Config

| Variable | Default | Purpose |
|---|---|---|
| `PORT` | `6677` | HTTP server port (Railway sets this automatically) |
| `RUST_LOG` | `info` | Log level. Use `info,roko=debug` for debugging. |

## Optional Overrides

### ROKO__ Prefix (Config Overrides)

Any roko.toml key can be overridden via env var: `ROKO__<SECTION>__<KEY>`.
Double underscore = dot separator in TOML path.

```bash
# Examples:
ROKO__AGENT__DEFAULT_MODEL=claude-sonnet-4-6
ROKO__SERVER__PORT=8080
ROKO__SERVE__AUTH__ENABLED=true
ROKO__RUNNER__PLAN_TIMEOUT_SECS=120
```

### CLI-Style Overrides

| Variable | Equivalent CLI flag | Purpose |
|---|---|---|
| `ROKO_MODEL` | `--model` | Override default model for all dispatches |
| `ROKO_ROLE` | `--role` | Override system role/persona |
| `ROKO_EFFORT` | `--effort` | Set effort level |
| `ROKO_QUIET` | `--quiet` | Suppress non-essential output |

### Server / SPA

| Variable | Default | Purpose |
|---|---|---|
| `ROKO_SPA_DIR` | (embedded in binary) | Path to pre-built `demo-app/dist/` on disk. Falls back to rust-embed. |
| `ROKO_SERVE_URL` | `ws://localhost:9092` | WebSocket URL for TUI agent stream |
| `ROKO_SERVER_URL` | `http://localhost:9092` | HTTP base URL for TUI server calls |
| `ROKO_SERVER_AUTH_TOKEN` | (none) | Bearer token for authenticated server requests |

### Integrations

| Variable | Purpose |
|---|---|
| `GITHUB_TOKEN` / `GH_TOKEN` | GitHub API access (PR creation, issue tracking) |
| `SLACK_BOT_TOKEN` | Slack MCP integration |
| `SLACK_SIGNING_SECRET` | Slack webhook verification |

### Debug / Dev

| Variable | Purpose |
|---|---|
| `ROKO_DEBUG` | Enable debug logging |
| `ROKO_LOG_FORMAT` | `json` or `text` log format |
| `ROKO_LOG_RAW` | `1` to disable secret redaction (NEVER in prod) |
| `ROKO_DEMO_CACHE` | `1` to enable demo caching |
| `ROKO_ACP_LEGACY` | Set to use legacy ACP pipeline |

## Dotenv Files

Roko loads env files in **`crates/roko-cli/src/main.rs`** (`load_startup_env_files`) as:

1. **`~/.roko/.env`** — loaded first; does **not** override variables already set in the process environment.
2. **`<cwd>/.roko/.env`** — loaded second with **override**; wins over global file and over step (1) for keys it sets.

Process environment (e.g. Railway/Fly injected vars) should be treated as highest precedence when already set before CLI startup.

### Managing secrets via CLI

```bash
# Store a secret (writes to ~/.roko/.env with 0600 perms)
roko config set-secret ANTHROPIC_API_KEY sk-ant-...

# Validate all configured secrets have values
roko config check-secrets
```

## Variable Interpolation in roko.toml

Config values support `${VAR}` and `${VAR:-fallback}`:

```toml
[providers.anthropic]
api_key_env = "ANTHROPIC_API_KEY"  # resolved at runtime
```

## Railway-Specific

Railway auto-sets `PORT`. Use `ROKO__SERVER__PORT=$PORT` or update the start command:

```bash
roko serve --bind 0.0.0.0 --port $PORT
```

---

## Mechanisms (not single variables)

| Mechanism | Purpose |
|-----------|---------|
| `ROKO__SECTION__FIELD` | Maps to dotted config path `section.field` (any depth). Can carry secrets if pointed at secret fields. |
| `${VAR}` in TOML | Resolved when config strings are interpolated. |
| `api_key_env` on `[providers.*]` | Names the env var for that provider’s key (any string — not limited to a fixed enum). |
| `ROKO_SECRET_<CATEGORY>_<PROVIDER>` | Namespaced secrets (`crates/roko-core/src/secrets/namespace.rs`). |

**Auth:** Privy integration uses **config** (`privy_app_id`, JWKS) — there are **no** `PRIVY_*` env vars in the Rust codebase.

---

## Expanded inventory (representative)

Use `rg 'std::env::var' crates/` when adding to prod — this list is representative, not exhaustive.

### Core CLI / config

| Variable | Role |
|----------|------|
| `ROKO_CONFIG` | Path to main config file. |
| `HOME`, `XDG_CONFIG_HOME`, `PATH` | Paths and tool discovery. |
| `NO_COLOR`, `CLICOLOR`, `CLICOLOR_FORCE` | Color policy. |
| `RUST_LOG`, `ROKO_LOG` | Tracing filters. |
| `ROKO_LOG_RAW` | **Dangerous in prod** — disables secret redaction in logs. |
| `ROKO_TIMING` | Timing instrumentation. |
| `ROKO_MODEL`, `ROKO_BACKEND`, `ROKO_EFFORT`, `ROKO_CONTEXT_LIMIT_K`, `ROKO_MAX_AGENTS`, `ROKO_BUDGET_USD`, `ROKO_PARALLEL`, `ROKO_EXPRESS`, `ROKO_SKIP_TESTS`, `ROKO_CLIPPY` | Merged config overrides (`schema.rs` / `apply_env`). |
| `ROKO_PROVIDER`, `ROKO_MODEL_SLUG` | Synthetic profile overrides. |
| `ROKO_ROLE`, `ROKO_QUIET`, `ROKO_LOG_FORMAT` | CLI-style overrides (`main.rs`). |
| `NUNCHI_DASHBOARD_URL` | Browser login dashboard URL (default `http://localhost:5173`). |
| `ROKO_API_KEY` | Client API key when talking to `roko serve`. |
| `ROKO_SERVE_URL`, `ROKO_SERVER_URL`, `ROKO_SERVER_AUTH_TOKEN` | TUI / remote server connection. |
| `ROKO_ATTEST_SIGNING_KEY_HEX` | Attestation signing material. |
| `ROKO_GATE_PLAN_ID`, `ROKO_GATE_TASK_ID`, `ROKO_GATE_RUNG`, `ROKO_GATE_ATTEMPT_SENTINEL` | Injected into gate subprocess env. |
| `ROKO_TEMPLATE_JSON`, `ROKO_CONTROL_PLANE_URL`, `ROKO_DEPLOYMENT_ID`, `PORT` | Cloud worker bootstrap (`worker/mod.rs`). |
| `GITHUB_TOKEN`, `GH_TOKEN` | GitHub API (PRs, server templates, Railway env collection). |
| `EDITOR` | Config editor (default `vi`). |

### Provider / LLM keys (and common alternates)

| Variable | Notes |
|----------|--------|
| `ANTHROPIC_API_KEY` | Default-path detection in several flows. |
| `OPENAI_API_KEY`, `OPENAI_API_BASE`, `OPENAI_BASE_URL` | OpenAI-compatible bases. |
| `PERPLEXITY_API_KEY` | Search / research tools. |
| `GEMINI_API_KEY` | Google. |
| `ZAI_API_KEY`, `ZAI_MODEL` | Z.AI. |
| `MOONSHOT_API_KEY`, `CEREBRAS_API_KEY`, `OPENROUTER_API_KEY` | As configured in `roko.toml`. |

### Server (`roko-serve`)

| Variable | Role |
|----------|------|
| `PORT` | Cloud bind helper in `lib.rs`. |
| `ROKO_SPA_DIR` | Override embedded SPA directory. |
| `SHELL` | PTY shell (default `/bin/zsh` in `terminal.rs`). |
| `RAILWAY_PUBLIC_DOMAIN`, `FLY_APP_NAME` | Public URL inference (`relay.rs`). |
| `SLACK_BOT_TOKEN`, `SLACK_TOKEN`, `SLACK_SIGNING_SECRET` | Feedback + webhooks. |
| `SKIP_FRONTEND_BUILD`, `CARGO_MANIFEST_DIR` | Build-time (`build.rs`). |

Dashboard copy may mention operational names like `ROKO_SERVE_AUTH_API_KEY` / `ROKO_DEPLOY_RAILWAY_API_TOKEN` — treat those as **human names** for secrets; values still flow through normal env/config merging.

### Agent / tools

| Variable | Role |
|----------|------|
| `ROKO_DEMO_CACHE` | Demo cache persistence (`1` / `true`). |
| `ROKO_DEBUG` | Agent debug paths. |
| `ROKO_MCP_CONFIG` | MCP config path override. |
| `ROKO_MCP_SCRIPTS_*`, `ROKO_SCRIPTS_DIR` | Scripts MCP (`roko-mcp-scripts`). |

### ACP / misc crates

| Variable | Role |
|----------|------|
| `ROKO_ACP_LEGACY` | Legacy ACP bridge (`roko-acp`). |
| `ROKO_MODEL` | Used in ACP runner for default model key. |
| `MIRAGE_*`, `ROKO_AGENT_RELAY_*`, `ETH_RPC_URL`, … | Mirage / chain sidecars (`apps/mirage-rs`, `roko-chain-watcher`) — only if you run those stacks. |

### CI / test-only

`CI`, `ROKO_TEST_OLLAMA`, `MIRAGE_RPC_URL` in tests, `ROKO_DISPATCHER`, `ROKO_MOCK_STATE_PATH`, etc. — **do not** set in production unless you know exactly why.
