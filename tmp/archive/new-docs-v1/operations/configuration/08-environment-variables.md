# Environment Variables

> Every key in `roko.toml` has a corresponding environment variable. This page lists all
> of them, their precedence, and the additional env vars required for LLM backends and
> secrets.

**Status**: Shipping
**Crate**: `roko-cli`
**Depends on**: [01-roko-toml-schema.md](01-roko-toml-schema.md), [09-cli-flag-precedence.md](09-cli-flag-precedence.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

Environment variables override `roko.toml` values. API keys must be set via environment
(never in the config file). The naming convention is `ROKO_<TABLE>_<KEY>` in
`UPPER_SNAKE_CASE`.

```bash
export ANTHROPIC_API_KEY=sk-ant-...
export ROKO_AGENT_MODEL=claude-opus-4-6
roko plan run plans/
```

---

## Naming Convention

For every key in `roko.toml`, the environment variable is:

```
ROKO_<TABLE>_<KEY>
```

Examples:

| `roko.toml` key | Environment variable |
|-----------------|----------------------|
| `agent.model` | `ROKO_AGENT_MODEL` |
| `agent.max_turns` | `ROKO_AGENT_MAX_TURNS` |
| `agent.timeout_seconds` | `ROKO_AGENT_TIMEOUT_SECONDS` |
| `gate.pipeline` | `ROKO_GATE_PIPELINE` (comma-separated) |
| `gate.max_retries` | `ROKO_GATE_MAX_RETRIES` |
| `learn.cascade_router` | `ROKO_LEARN_CASCADE_ROUTER` |
| `learn.experiments` | `ROKO_LEARN_EXPERIMENTS` |
| `learn.episode_store` | `ROKO_LEARN_EPISODE_STORE` |
| `substrate.backend` | `ROKO_SUBSTRATE_BACKEND` |
| `substrate.data_dir` | `ROKO_SUBSTRATE_DATA_DIR` |
| `substrate.max_size_gb` | `ROKO_SUBSTRATE_MAX_SIZE_GB` |

Boolean values use `"true"` or `"false"` (case-insensitive).
Array values (like `gate.pipeline`) use comma-separated strings: `"compile,test,clippy"`.

---

## Special Variables

These variables are not derived from `roko.toml` keys but are required or affect Roko's
behaviour:

### `ROKO_CONFIG`

```
Type:    String (absolute file path)
Default: <not set — searches for roko.toml in CWD>
Example: ROKO_CONFIG=/etc/roko/roko.toml
Notes:   Override the config file path. Equivalent to --config <path> on the CLI.
         Takes priority over the CWD search.
```

### `ROKO_LOG`

```
Type:    String (log filter directive)
Default: "warn"
Example: ROKO_LOG=roko=debug,roko_agent=trace
Notes:   Controls the structured log output level, using the tracing-subscriber
         EnvFilter syntax. Components: roko, roko_agent, roko_gate, roko_learn,
         roko_orchestrator, roko_runtime, roko_fs.
         Values: error, warn, info, debug, trace.
         Use "roko=debug" for troubleshooting; "roko=trace" for deep debugging
         (very verbose, ~100MB/min of logs at trace level).
```

### `ROKO_LOG_FORMAT`

```
Type:    String
Default: "pretty"
Range:   "pretty" | "json" | "compact"
Example: ROKO_LOG_FORMAT=json
Notes:   Log output format. "json" is for log aggregators (Datadog, Loki, etc.).
         "pretty" is human-readable for terminal use. "compact" is single-line
         human-readable.
```

### `ROKO_NO_COLOR`

```
Type:    Boolean ("1" or "0", or present/absent)
Default: <not set>
Example: ROKO_NO_COLOR=1
Notes:   Disable ANSI color codes in terminal output. Roko also respects the
         standard NO_COLOR environment variable.
```

---

## LLM Backend API Keys

These are not `roko.toml` keys — they are passed directly to the LLM backend libraries:

| Backend | Required env var | Optional additional vars |
|---------|------------------|--------------------------|
| `anthropic` | `ANTHROPIC_API_KEY` | `ANTHROPIC_API_KEY_2` through `_10` (key rotation) |
| `openai` | `OPENAI_API_KEY` | `OPENAI_ORG_ID` |
| `openrouter` | `OPENROUTER_API_KEY` | — |
| `ollama` | none | `OLLAMA_HOST` (default: `http://localhost:11434`) |
| `bedrock` | `AWS_ACCESS_KEY_ID` + `AWS_SECRET_ACCESS_KEY` | `AWS_REGION` |
| `vertex` | `GOOGLE_APPLICATION_CREDENTIALS` (service account JSON path) | `GOOGLE_CLOUD_PROJECT` |

**Key rotation for Anthropic:** Set `ANTHROPIC_API_KEY` through `ANTHROPIC_API_KEY_10`
to enable round-robin key rotation when rate limits are hit. Roko rotates to the next
available key on a 429 response.

---

## Composing Environments

Use `.env` files (loaded via `dotenv` conventions) for local development. **Never commit
API keys** to `.env` files in version-controlled repositories.

Recommended `.env` file for local development:

```bash
# .env (add to .gitignore)
ANTHROPIC_API_KEY=sk-ant-...

# Optional: point at a local gateway for caching
ROKO_AGENT_BASE_URL=http://localhost:4000/

# Debug logging
ROKO_LOG=roko=info

# Development-friendly config overrides
ROKO_AGENT_MAX_TURNS=10
ROKO_GATE_MAX_RETRIES=1
```

For production servers, use the secrets management system appropriate to your
infrastructure (AWS Secrets Manager, Vault, Kubernetes Secrets, etc.) and inject
environment variables at process startup. Do not write secrets to disk on production
machines.

---

## Array Values

`ROKO_GATE_PIPELINE` accepts a comma-separated list of gate names. All of these are
equivalent:

```bash
# Via roko.toml
# pipeline = ["compile", "test", "clippy"]

# Via environment
export ROKO_GATE_PIPELINE="compile,test,clippy"

# Via CLI flag
roko plan run plans/ --gate-pipeline compile,test,clippy
```

Whitespace around commas is stripped. Case is preserved (gate names are lowercase).

---

## Two Full Examples

**Minimal CI environment:**

```bash
export ANTHROPIC_API_KEY=sk-ant-...
export ROKO_AGENT_MODEL=claude-haiku-4-5
export ROKO_AGENT_MAX_TURNS=15
export ROKO_GATE_PIPELINE="compile,test"
export ROKO_GATE_ADAPTIVE_THRESHOLDS=false
export ROKO_LEARN_EXPERIMENTS=false
export ROKO_LOG=roko=info
export ROKO_LOG_FORMAT=json
```

**Production server with shared learning:**

```bash
export ANTHROPIC_API_KEY=sk-ant-...
export ROKO_AGENT_MODEL=claude-opus-4-6
export ROKO_AGENT_BASE_URL=http://roko-gateway.internal:4000/
export ROKO_LEARN_EPISODE_STORE=/var/roko/episodes
export ROKO_LEARN_PLAYBOOK_PATH=/var/roko/playbook.toml
export ROKO_SUBSTRATE_DATA_DIR=/var/roko/substrate
export ROKO_SUBSTRATE_MAX_SIZE_GB=100.0
export ROKO_LOG=roko=warn
export ROKO_LOG_FORMAT=json
```

---

## See Also

- [09-cli-flag-precedence.md](09-cli-flag-precedence.md) — full override chain
- [13-security-considerations.md](13-security-considerations.md) — secrets management
- [10-config-validation.md](10-config-validation.md) — what happens if an env var has a bad value

## Open Questions

- `ROKO_LOG` filter syntax documentation is not yet in the official docs (it follows the `tracing-subscriber` `EnvFilter` syntax; an operator-friendly summary page is planned).
- Whether Roko should auto-load `.env` files or require explicit sourcing is under discussion.
