# CLI Flag Precedence

> When the same configuration value can come from multiple sources, this page defines
> which source wins. The rule is simple: CLI flag > environment variable > `roko.toml` > built-in default.

**Status**: Shipping
**Crate**: `roko-cli`
**Depends on**: [08-environment-variables.md](08-environment-variables.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

Later sources override earlier sources. In order of increasing precedence:

```
built-in default  <  roko.toml  <  environment variable  <  CLI flag
```

The CLI flag always wins. The built-in default is the last resort.

---

## The Four Layers

### Layer 1: Built-in Defaults

Every configuration key has a hard-coded default embedded in the binary. These are the
values Roko uses when no other source specifies a value for a key.

Examples: `model = "claude-sonnet-4-5"`, `max_turns = 25`, `gate.pipeline = ["compile", "test", "clippy", "diff"]`.

Built-in defaults are documented in the key entries in [01-roko-toml-schema.md](01-roko-toml-schema.md).

### Layer 2: `roko.toml`

Values in `roko.toml` override the built-in defaults. Only keys that are explicitly set
in the file take effect from this layer; absent keys fall back to defaults.

The file is discovered in this order:
1. Path given by `--config <path>` CLI flag.
2. `ROKO_CONFIG` environment variable.
3. `roko.toml` in the current working directory.
4. `roko.toml` in the path given by `--project <path>` CLI flag.
5. If none found, the file layer is skipped entirely (all values come from defaults or env/flags).

### Layer 3: Environment Variables

Environment variables override values from both the file and the defaults. The naming
convention is `ROKO_<TABLE>_<KEY>` (see [08-environment-variables.md](08-environment-variables.md)
for the full table).

An environment variable for a key that is also in `roko.toml` silently wins. Use
`roko config show` to see the final merged value.

### Layer 4: CLI Flags

Flags passed on the command line override everything. CLI flags are the highest-precedence
source.

Available configuration flags (the most common):

| CLI flag | Overrides | Example |
|----------|-----------|---------|
| `--config <path>` | Config file path | `roko run --config /etc/roko.toml "task"` |
| `--model <slug>` | `agent.model` | `roko run --model claude-opus-4-6 "task"` |
| `--max-turns <n>` | `agent.max_turns` | `roko plan run --max-turns 50 plans/` |
| `--timeout <s>` | `agent.timeout_seconds` | `roko run --timeout 300 "task"` |
| `--gate-pipeline <list>` | `gate.pipeline` | `roko plan run --gate-pipeline compile,test plans/` |
| `--no-learning` | `learn.cascade_router = false` + `learn.experiments = false` | `roko run --no-learning "task"` |
| `--resume <path>` | Executor resume state path | `roko plan run --resume .roko/state/executor.json plans/` |
| `--project <path>` | Working directory | `roko plan run --project /path/to/project plans/` |
| `--concurrency <n>` | Maximum parallel agents | `roko plan run --concurrency 8 plans/` |
| `--dry-run` | Enables dry-run mode (no agent calls) | `roko plan run --dry-run plans/` |

---

## Worked Examples

### Example 1: Model override via CLI

`roko.toml` has `model = "claude-sonnet-4-5"`. You want to run one task with the opus model:

```bash
roko run --model claude-opus-4-6 "Design the new event bus interface"
```

The CLI flag `--model claude-opus-4-6` overrides the file's `claude-sonnet-4-5` for this
invocation only. The file is not changed.

### Example 2: Environment variable for CI

In CI, you set:

```bash
export ROKO_AGENT_MODEL=claude-haiku-4-5
export ROKO_GATE_MAX_RETRIES=1
export ROKO_LEARN_EXPERIMENTS=false
```

These override any values in `roko.toml` (if present) without modifying the file.

### Example 3: All three layers active

```
roko.toml:   model = "claude-sonnet-4-5"
environment: ROKO_AGENT_MODEL=claude-opus-4-6
CLI flag:    --model claude-haiku-4-5
```

Result: `claude-haiku-4-5` (CLI flag wins).

### Example 4: Debugging with `roko config show`

To see exactly what values Roko will use (after all four layers are merged):

```bash
ROKO_AGENT_MODEL=claude-opus-4-6 roko config show
```

Output shows the merged config with the source of each value annotated:

```
[agent]
model = "claude-opus-4-6"          # source: environment
max_turns = 25                      # source: default
timeout_seconds = 600               # source: roko.toml
...
```

---

## Type Coercion for CLI Flags and Env Vars

All config values are ultimately TOML types. When provided via env vars or CLI flags
(which are strings), Roko coerces them:

| TOML type | String representation | Parse rules |
|-----------|----------------------|-------------|
| `Boolean` | `"true"` or `"false"` | Case-insensitive. `"1"` / `"0"` also accepted. |
| `Integer` | `"25"` | Decimal integer string. |
| `Float` | `"10.5"` | Decimal float string. |
| `Array of String` | `"compile,test,clippy"` | Comma-separated, whitespace stripped. |
| `String` | `"any string"` | Passed through as-is. |

If coercion fails (e.g. `ROKO_AGENT_MAX_TURNS=abc`), Roko emits a validation error at
startup. See [10-config-validation.md](10-config-validation.md).

---

## See Also

- [08-environment-variables.md](08-environment-variables.md) — full env var reference
- [10-config-validation.md](10-config-validation.md) — what errors look like when values are invalid
- [00-overview.md](00-overview.md) — `roko config show` and `roko config set`

## Open Questions

- Per-command flag registry (some flags apply to all commands; some are command-specific) — full documentation is pending the CLI reference update.
- `--set <key=value>` as a generic override flag (for any `roko.toml` key) is planned but not yet implemented.
