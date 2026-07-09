# Configuration Validation

> Roko validates the merged configuration at startup before doing anything else. This page
> documents what is checked, what errors look like, and how to fix them.

**Status**: Shipping
**Crate**: `roko-cli`
**Depends on**: [01-roko-toml-schema.md](01-roko-toml-schema.md), [09-cli-flag-precedence.md](09-cli-flag-precedence.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

If the config is invalid, Roko exits immediately with a structured error pointing at the
exact key and the problem. Fix the key and restart.

```
Error: configuration validation failed

  ✗ agent.max_turns: value 250 exceeds maximum 200
    set in: roko.toml (line 4)
    fix:    lower the value to ≤ 200, or remove the key to use the default (25)
```

---

## When Validation Runs

The config is loaded and validated in these situations:

1. **At any `roko` command startup** — the merged config (file + env + flags) is fully
   validated before any work begins. An invalid config causes an immediate non-zero exit.
2. **On `roko config show`** — shows the merged config; also reports validation errors
   if present.
3. **On `roko config set <key> <value>`** — validates the new value before writing it
   to `roko.toml`.

Roko does not silently ignore invalid values. Every bad value is a hard startup failure.

---

## Classes of Validation Error

### Type errors

A key is set to a value of the wrong type.

```
Error: configuration validation failed

  ✗ agent.max_turns: expected integer, got string "twenty-five"
    set in: environment variable ROKO_AGENT_MAX_TURNS
    fix:    set ROKO_AGENT_MAX_TURNS to an integer (e.g. "25")
```

### Range errors

A numeric value is outside its allowed range.

```
Error: configuration validation failed

  ✗ gate.max_retries: value 10 exceeds maximum 8
    set in: roko.toml (line 12)
    fix:    lower the value to ≤ 8, or remove the key to use the default (3)

  ✗ substrate.gc_interval_hours: value 0 is below minimum 1
    set in: environment variable ROKO_SUBSTRATE_GC_INTERVAL_HOURS
    fix:    set to at least 1 (hours). Use 24 for the default.
```

### Unknown key errors

A key in `roko.toml` is not recognised.

```
Error: configuration validation failed

  ✗ Unknown key: agent.mdl
    set in: roko.toml (line 3)
    fix:    did you mean "agent.model"? Check 01-roko-toml-schema.md for valid keys.
```

Unknown keys in `roko.toml` are **hard errors** (not warnings). This prevents silently
ignoring misconfigured keys due to typos. Use `roko config show` to see all valid keys.

### File-not-found errors

A file path key points at a non-existent file.

```
Error: configuration validation failed

  ✗ agent.system_prompt_path: file not found: "AGENTS.md"
    set in: roko.toml (line 7)
    fix:    create the file, or remove the key to use the built-in system prompt
```

Note: `agent.mcp_config` is **not** validated for file existence at startup — MCP server
startup failure is handled separately as a runtime warning (not a startup error). This
allows Roko to start even when MCP servers are not yet installed.

### Backend errors

An unsupported backend slug is specified.

```
Error: configuration validation failed

  ✗ agent.backend: unknown backend "anth"
    set in: roko.toml (line 5)
    fix:    valid backends are: "anthropic", "openai", "openrouter", "ollama", "bedrock", "vertex"
            did you mean "anthropic"?
```

### Target-state key warnings

Using a Specified (target-state) key in `roko.toml` emits a warning but does not fail:

```
Warning: roko.toml uses target-state key "bus.backend"
  This key is defined in the schema but not yet wired. The value will be ignored.
  See operations/configuration/06-bus-config.md for details.
```

---

## Multiple Errors in One Pass

Roko collects all validation errors before reporting. You never see one error, fix it,
and then see the next one. All errors are shown together:

```
Error: configuration validation failed (3 errors)

  ✗ agent.max_turns: value 250 exceeds maximum 200
    set in: roko.toml (line 4)

  ✗ gate.pipeline: unknown gate "lnt" (did you mean "lint"?)
    set in: roko.toml (line 9)

  ✗ substrate.max_size_gb: expected float, got string "ten"
    set in: environment variable ROKO_SUBSTRATE_MAX_SIZE_GB
```

---

## Debugging with `roko config show`

Even if you think the config is valid, `roko config show` is a useful first step when
diagnosing unexpected behaviour. It shows the fully-merged config with source annotations
and any warnings:

```bash
roko config show
```

Output example (abbreviated):

```toml
# Merged configuration — sources: roko.toml + environment
# 1 warning

[agent]
model              = "claude-opus-4-6"     # env: ROKO_AGENT_MODEL
max_turns          = 25                    # default
timeout_seconds    = 900                   # roko.toml:6
...

Warnings:
  ⚠ roko.toml key "bus.backend" is target-state and will be ignored
```

---

## See Also

- [01-roko-toml-schema.md](01-roko-toml-schema.md) — full key reference with valid ranges
- [09-cli-flag-precedence.md](09-cli-flag-precedence.md) — understanding source annotations
- [11-config-migration.md](11-config-migration.md) — if the error is "unknown key" from an old config format

## Open Questions

- JSON Schema export (`roko config schema --json`) is planned to allow editor validation (VS Code, etc.) but not yet implemented.
- `--strict` mode (treat target-state key warnings as errors) is under consideration for CI environments.
