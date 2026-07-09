# Configuration Overview

> `roko.toml` is the single configuration surface for the Roko runtime. Every subsystem
> that has a configurable behaviour reads from this file, from the agent's model choice
> to the substrate's storage path.

**Status**: Shipping
**Crate**: `roko-cli`, `roko-orchestrator`
**Depends on**: [operations/configuration/README.md](README.md)
**Used by**: [02-agent-config.md](02-agent-config.md), [03-gate-config.md](03-gate-config.md), [04-learn-config.md](04-learn-config.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

Run `roko init` in any project directory. It creates `.roko/` and an initial `roko.toml`.
Edit `roko.toml` to change behaviour. Environment variables override file values; CLI flags
override environment variables. Nothing else needs to change.

---

## The Idea

Roko uses a single `roko.toml` file rather than many per-subsystem config files. The
rationale is operability: one file to audit, one file to version-control, one file to hand
off to a team member. Every knob for every subsystem is in one place.

The file is **optional** in development. Roko ships sensible defaults for every key. You
only need `roko.toml` when you want to override a default.

In production, commit `roko.toml` to your repository. Secrets (API keys, tokens) go in
environment variables, not in `roko.toml`. The config file is safe to version-control;
the environment is where secrets live. See [13-security-considerations.md](13-security-considerations.md).

---

## The `roko init` Workflow

Running `roko init` in a project directory:

1. Creates the `.roko/` directory (state store, logs, run history).
2. Writes a starter `roko.toml` with the most commonly overridden keys and their defaults.
3. Writes a starter `.mcp.json` for MCP tool server discovery.
4. Does **not** overwrite existing files — safe to re-run.

```bash
cd /path/to/your/project
roko init
# Created .roko/
# Created roko.toml
# Created .mcp.json
```

After `roko init`, the directory looks like:

```
your-project/
  roko.toml          ← configuration (commit this)
  .mcp.json          ← MCP tool servers (commit this)
  .roko/
    state/           ← executor snapshots (do not commit)
    logs/            ← structured log output (do not commit)
    episodes/        ← learning episode store (do not commit)
    .roko.lock       ← prevents concurrent roko processes
```

The `.roko/` directory is ephemeral state. Commit `roko.toml` and `.mcp.json`. Add
`.roko/` to `.gitignore`.

---

## The `roko config` Subcommand

Three subcommands help manage the config without editing the file by hand:

| Command | What it does |
|---------|-------------|
| `roko config show` | Print the fully-resolved config (file + env overrides merged) |
| `roko config edit` | Open `roko.toml` in `$EDITOR` |
| `roko config set <key> <value>` | Write a single key to `roko.toml` (e.g. `roko config set agent.model claude-sonnet-4-5`) |

`roko config show` is particularly useful for debugging: it shows the merged, validated
config exactly as the runtime sees it, after all env-var and flag overrides are applied.

---

## Config File Location

Roko searches for `roko.toml` in the following order, stopping at the first match:

1. Path given by `--config <path>` CLI flag.
2. `ROKO_CONFIG` environment variable (must be an absolute path).
3. `roko.toml` in the current working directory.
4. `roko.toml` in the directory given by `--project <path>` (if set).
5. Built-in defaults (no file required).

This means you can maintain one `roko.toml` per project by always running `roko` from
the project root, which is the recommended pattern.

---

## Table Structure at a Glance

```toml
[agent]         # LLM model, turn limits, timeouts, MCP path
[gate]          # Verification pipeline: which gates, thresholds
[learn]         # CascadeRouter, experiment A/B testing, distillation
[substrate]     # Storage backend: JSONL, in-memory, SQLite (planned)
[bus]           # Transport (target-state; not required today)
```

Every table is optional. An empty `roko.toml` (or no file at all) is valid.

---

## Minimum Working Configuration

A minimal `roko.toml` for a coding agent on a laptop:

```toml
[agent]
model = "claude-sonnet-4-5"
mcp_config = ".mcp.json"
```

A minimal config for a server deployment:

```toml
[agent]
model = "claude-opus-4-6"
max_turns = 40
timeout_seconds = 900

[gate]
pipeline = ["compile", "test", "clippy", "diff", "semantic"]

[learn]
cascade_router = true
experiments = true
```

For complete worked examples see [12-examples.md](12-examples.md).

---

## See Also

- [01-roko-toml-schema.md](01-roko-toml-schema.md) — full schema with every key
- [08-environment-variables.md](08-environment-variables.md) — env-var overrides
- [09-cli-flag-precedence.md](09-cli-flag-precedence.md) — override chain
- [12-examples.md](12-examples.md) — ready-to-use profiles
- [13-security-considerations.md](13-security-considerations.md) — secrets and `.roko/` security

## Open Questions

- Nested project config discovery (parent directory walk-up) is not yet implemented.
- Config schema versioning (`roko.toml.version` key) is discussed but not yet introduced.
