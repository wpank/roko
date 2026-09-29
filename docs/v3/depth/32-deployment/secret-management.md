# 32-deployment/12 -- Secret Management

> Shape-aware secret resolution: CLI flags, environment, config files,
> OS keychain, external stores, Docker patterns, and the `_FILE` suffix.

**Parent:** [32-DEPLOYMENT](../../32-DEPLOYMENT.md)

**Source:** `crates/roko-core/src/config/loader.rs`,
`crates/roko-cli/src/config_helpers.rs`

---

## 1. Shape-Aware Secret Policy

Secret handling follows the deployment profile. The same binary uses
different defaults based on the selected shape:

| Shape | Default Secret Source | Notes |
|-------|---------------------|-------|
| laptop-local | OS keychain | Best interactive experience |
| single-server | OS keychain or host store | Scoped by user or role |
| container | Env vars or `_FILE` mounts | Docker, Compose, orchestrators |
| clustered | External secret store | Vault, cloud secret manager |
| edge | Provider-native injection | Minimal surface |

Profiles can override these defaults without changing resolution
semantics.

---

## 2. Secret Resolution Order

Higher-priority sources override lower ones:

```
1. CLI flags         roko run --api-key sk-ant-...
       |
2. Environment       ANTHROPIC_API_KEY=sk-ant-...
       |
3. Config files      roko.toml / ~/.config/roko/config.toml
       |
4. OS keychain       macOS Keychain / Linux Secret Service
       |
5. Secret store      Vault / AWS Secrets Manager / 1Password CLI
       |
6. Compiled default  fail with actionable error message
```

CLI flags are for one-off debugging. Environment variables are the
portable container default. Config files hold declarative references.
OS keychains are the laptop-local default. External stores are for
production.

---

## 3. Environment Variable Conventions

| Variable | Used By | Purpose |
|----------|---------|---------|
| `ANTHROPIC_API_KEY` | roko-cli, roko-serve | Anthropic provider |
| `OPENAI_API_KEY` | roko-cli, roko-serve | OpenAI provider |
| `OPENROUTER_API_KEY` | roko-cli, roko-serve | OpenRouter multi-model |
| `GEMINI_API_KEY` | roko-cli, roko-serve | Google Gemini |
| `CEREBRAS_API_KEY` | roko-cli, roko-serve | Cerebras provider |
| `PERPLEXITY_API_KEY` | roko-cli, roko-serve | Perplexity research |
| `RUST_LOG` | all | Log level filter |

Product-specific prefixes:

| Product | Prefix | Examples |
|---------|--------|---------|
| roko-cli | `ROKO_` | `ROKO_MODEL`, `ROKO_MAX_AGENTS` |
| roko-serve | `ROKO_SERVE_` | `ROKO_SERVE_PORT`, `ROKO_SERVE_BIND` |

---

## 4. .env File Loading

`dotenvy` loads `.env` files from the working directory for laptop-local
and single-server profiles:

```rust
dotenvy::dotenv().ok();
```

Rules:
1. Look for `.env` in the current working directory
2. Load if present
3. Do not overwrite existing environment variables
4. Continue silently if missing

### .env Format

```bash
# .env (gitignored)
ANTHROPIC_API_KEY=sk-ant-abc123...
OPENAI_API_KEY=sk-def456...
ROKO_WEBHOOK_SECRET=whsec_abc123
```

### Gitignore

```gitignore
.env
.env.local
.env.*.local
```

---

## 5. ${VAR} Interpolation in Config

Config files support `${VAR}` syntax for declarative secret references:

```toml
[agent.providers.anthropic]
api_key = "${ANTHROPIC_API_KEY}"

[subscription.webhook]
secret = "${ROKO_WEBHOOK_SECRET}"
```

### Rules

1. `${VAR}` resolves from the environment
2. `${VAR:-default}` falls back to a default
3. `$$` escapes a literal dollar sign
4. Unresolved variables log a warning and remain visible

---

## 6. OS Keychain Integration

For interactive use, the OS keychain is the preferred secret store:

| Platform | Backend |
|----------|---------|
| macOS | Keychain |
| Linux | Secret Service (GNOME Keyring, KWallet) |
| Windows | Credential Manager |

### Interactive Setup

`roko setup` prompts for keys and offers to store them in the keychain:

```bash
$ roko setup

Enter your Anthropic API key (or press Enter to skip):
> sk-ant-...
Store in system keychain? [Y/n]: Y
Saved to keychain as "roko/anthropic-api-key"
```

### Key Rotation

Rotation updates the backing store. When the deployment profile supports
live refresh, no restart is needed.

---

## 7. Secret CLI

```bash
roko config secrets set anthropic.api_key
roko config secrets get anthropic.api_key
roko config secrets list
roko config secrets rotate anthropic.api_key
```

The CLI routes to the active profile's backing store, whether that is
a keychain, environment, or external store.

### Role-Based Secrets

Some roles need additional keys. The secret layer supports role-scoped
injection so a reviewer, implementer, or operator only receives the
credentials it needs. See the agent contract system for role-based tool
policy.

---

## 8. Docker Secret Patterns

Containers prefer environment variables or `_FILE` indirection:

```yaml
services:
  roko:
    image: ghcr.io/nunchi/roko-cli:latest
    environment:
      ANTHROPIC_API_KEY: ${ANTHROPIC_API_KEY:?Required}
    secrets:
      - anthropic_key

secrets:
  anthropic_key:
    file: ./secrets/anthropic.txt
```

### _FILE Suffix Convention

For Docker secrets and orchestrator mounts, support reading from a file
instead of the environment:

```rust
fn resolve_env_or_file(var_name: &str) -> Option<String> {
    if let Ok(val) = std::env::var(var_name) {
        return Some(val);
    }
    let file_var = format!("{var_name}_FILE");
    if let Ok(path) = std::env::var(&file_var) {
        if let Ok(val) = std::fs::read_to_string(&path) {
            return Some(val.trim().to_string());
        }
    }
    None
}
```

This keeps secrets out of process listings and image layers.

---

## 9. Cloud Platform Secrets

### Fly.io

```bash
fly secrets set ANTHROPIC_API_KEY=sk-ant-... --app roko-serve
```

Encrypted at rest, injected as env vars at machine startup.

### Railway

Set via GraphQL API during `roko deploy railway`. Injected as env vars.

Both platforms: secrets never appear in config files, image layers, or
logs.

---

## 10. Safety Rules

1. Never log secret values
2. Never write secrets into Signal bodies or long-lived state
3. Never persist resolved secrets into `.roko/` archives
4. Keep `.env` files gitignored
5. Prefer OS keychain for laptop-local
6. Prefer `_FILE` and external stores for containers

### Audit Trail

The daemon logs which source resolved each secret without the value:

```
[INFO] Secret resolved: ANTHROPIC_API_KEY source=keychain
[INFO] Secret resolved: OPENAI_API_KEY source=env
[WARN] Secret not found: OPENROUTER_API_KEY (optional)
```

### roko doctor Check

`roko doctor` validates secret availability:

```
Credentials:
  ANTHROPIC_API_KEY: set [source: keychain]
  OPENAI_API_KEY: set [source: .env]
  OPENROUTER_API_KEY: not set (optional)
```

---

## 11. Implementation Status

> **Implementation status:** Environment variable resolution works
> (roko-agent reads env vars directly). The `roko config secrets` CLI
> commands exist with profile-aware secret management. dotenvy loading
> is wired. `${VAR}` interpolation is implemented in the config loader.
> OS keychain integration via the `keyring` crate is designed but not
> wired. The `_FILE` suffix convention is designed but not implemented.
