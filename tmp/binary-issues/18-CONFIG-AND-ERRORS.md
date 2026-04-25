# 18 — Configuration, Error Handling, and Provider Management Audit

**Status**: open (medium)
**Scope**: `crates/roko-cli/src/config.rs`, `crates/roko-cli/src/config_cmd.rs`, `crates/roko-cli/src/auth_detect.rs`, `crates/roko-cli/src/main.rs`

## What This Document Covers

The configuration system, error messages, environment detection, and provider management
that determine whether a user can even get started with roko.

---

## 1. First-Run Experience

### What roko does well
- **Zero-config chat works.** `roko` with an API key or Claude CLI just works.
- **Auth detection is automatic** with clear messages on failure.
- **`roko config validate`** is thorough (3-phase, probes endpoints, cross-references).
- **`roko doctor`** is a proper health check with structured output.
- **`roko config show`** annotates every field with its source.
- **Secret redaction** in logs is comprehensive and on by default.
- **Layered config** with clear precedence (env > project > global > defaults).

### Issues

**CF1. Two separate init paths, never connected** (`config_cmd.rs:49`, `commands/util.rs:98`)

- `roko init` creates `.roko/` + `roko.toml` (non-interactive, writes "claude" as default)
- `roko config init` runs an interactive wizard writing to `~/.roko/config.toml` (global)

These never reference each other. The wizard doesn't know about the project-level config.
The project init doesn't offer interactive setup.

**CF2. Default agent command is `cat`** (`config.rs:312`)

`AgentConfig::default()` sets `command: "cat".into()`. Intentional for tests (safe no-op),
but means `roko run "fix this"` without config invokes `cat` as the agent — echoes the
prompt back as "output" and reports success. The CLI warns about this on the `roko run`
path but not on other paths.

**CF3. No `roko config providers add` command**

Users must manually edit TOML to add a provider. No scaffolding, no interactive wizard for
adding providers. Required fields per provider kind must be known in advance.

---

## 2. Configuration Loading

### Precedence (highest first, `config.rs:2736`)
1. `ROKO__*` env vars (per-field overrides)
2. `ROKO_CONFIG` env var (single file override)
3. Project `roko.toml` (walked up from cwd)
4. Global `~/.roko/config.toml`
5. Struct defaults

### Issues

**CL1. Missing env var silently uses empty string** (`config.rs:2197`)

```
warning: ${VAR_NAME} referenced but VAR_NAME not set; using empty string
```

Goes to stderr but config load succeeds with empty values. The provider config now has
empty `api_key` and will fail later with a different, harder-to-diagnose error. Should
fail fast or at least mark the config as degraded.

**CL2. Config migration only handles v1 → v2** (`config_cmd.rs:731-774`)

No forward-looking mechanism. If schema bumps to v3, v2 configs won't warn. The
`is_stale()` method exists (`self.schema_version < CURRENT_SCHEMA_VERSION`) but is never
called to trigger a warning.

---

## 3. Error Message Quality

### Good errors
- Config validation: 3-phase with unicode symbols and clear counts
- Auth detection failure: clear setup instructions with examples
- Doctor checks: structured ok/warn/fail/skipped per check
- Missing config: warns about `cat` default, suggests `roko init`

### Bad errors

**ER1. Top-level error handler is generic** (`main.rs:1645`)

```rust
eprintln!("error: {e:#}");
```

Just prints the anyhow chain. No suggestion about what to do, no "try `roko doctor`",
no link to docs.

**ER2. Wizard cancellation exits with ERROR code** (`config_cmd.rs:155,160`)

When the user cancels the wizard, the error is `anyhow!("cancelled")`. This propagates
through the error chain and exits with code 2 (`EXIT_SYSTEM_ERROR`) instead of clean
exit code 0.

**ER3. `providers health` vs `providers list` naming is misleading** (`commands/config_cmd.rs:247,276`)

`providers health` reads persisted historical data (circuit breaker state, latency
percentiles). `providers list` does live probes. The names suggest the opposite.

---

## 4. Silent Failures

**SF1. 18+ `.ok()` calls in orchestrate.rs silently swallow errors**

Found at lines: 1049, 1052, 2632, 4101, 4311, 4518, 6413, 6629, 7339, 8391, 9351,
12461, 13324, 14001, 14008, 15573, 15705, 16538, 16932, 17087, 17684, 18412.

These include:
- Task file parsing failures
- Git operations
- File system operations
- Substrate writes

Each `.ok()` discards the error. If any of these fail, the system continues with
potentially corrupt or incomplete state and the user never knows.

**SF2. Episode logger failure logged to stderr, run continues** (`run.rs:1063`)

```rust
eprintln!("[run] episode logger failed: {err}");
```

The user loses episode data without knowing it matters.

**SF3. Background serve failure only logged at warn level** (`unified.rs:141`)

In unified chat mode, tracing goes to a file. The user sees no indication that serve
failed to start.

---

## 5. Environment Detection

**ED1. No proxy support** (absent from entire codebase)

Searched for `HTTP_PROXY`, `HTTPS_PROXY`, `NO_PROXY`: zero results in any crate.
`reqwest` reads system proxy env vars by default, but this is undocumented and there's
no way to configure proxy settings in `roko.toml`. Users behind corporate firewalls
have no guidance.

**ED2. `ROKO_LOG_RAW=1` disables ALL secret redaction** (`main.rs:1721`)

Documented in a code comment but not in `--help` or any user-facing docs. A user
debugging a provider issue might set this and accidentally expose API keys in logs.

---

## Anti-Patterns

1. **Silent degradation**: Missing env vars, failed operations, and broken subsystems
   are silently worked around instead of surfaced to the user. The system appears to work
   but is actually running in a degraded state.

2. **`.ok()` epidemic**: 18+ error-swallowing calls in the orchestrator alone. Each is a
   potential silent failure that corrupts downstream assumptions.

3. **Misleading command names**: `providers health` is historical data, `providers list`
   is live. Users will try the wrong one.

4. **Delayed failure**: Empty env vars pass config loading, then cause cryptic errors at
   runtime. Fail-fast would catch these at startup.

---

## Root Cause Fix

1. **Fail-fast on config issues** — empty env vars, missing providers, invalid keys
   should fail at load time, not at first use.

2. **Replace `.ok()` with structured error handling** — categorize: log-and-continue vs
   abort-task vs surface-to-user. Add an `#[allow(clippy::ok)]` lint to prevent new
   silent swallows.

3. **Unify init paths** — one `roko init` command that handles both project and global
   config, with interactive prompts when run in a TTY.

4. **Error suggestions at the top level** — the main error handler should pattern-match
   on common error types and suggest `roko doctor`, `roko config validate`, or specific
   fixes.

---

## Checklist

- [ ] Fail-fast when env var interpolation produces empty values
- [ ] Audit and replace `.ok()` calls in orchestrate.rs with proper error handling
- [ ] Unify `roko init` and `roko config init` into one command
- [ ] Add `roko config providers add` interactive command
- [ ] Fix wizard cancellation to exit with code 0
- [ ] Rename `providers health`/`providers list` to match their actual behavior
- [ ] Add error suggestions to top-level error handler
- [ ] Document proxy support (or add explicit proxy config)
- [ ] Add `ROKO_LOG_RAW` warning to `--help` output
- [ ] Add forward-looking schema version warning (not just v1)
