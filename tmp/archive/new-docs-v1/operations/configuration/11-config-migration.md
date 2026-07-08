# Configuration Migration

> How to move from older Roko and Bardo/Mori configuration formats to the current
> `roko.toml` schema.

**Status**: Shipping
**Crate**: `roko-cli`
**Depends on**: [10-config-validation.md](10-config-validation.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

If you are starting fresh, skip this page. It is only relevant if you have an existing
config from an older version of Roko or from the predecessor (Bardo/Mori).

Run `roko migrate` to automatically convert an old config file to the current schema.

---

## Migration Path Overview

| Old format | Source | Migration approach |
|------------|--------|-------------------|
| Bardo `playbook.toml` | Bardo agent orchestrator | `roko migrate --from bardo-playbook` |
| Mori `.mori.toml` | Mori build orchestrator | `roko migrate --from mori` |
| Roko v0.x `roko.toml` | Earlier Roko versions | `roko migrate --from roko-v0` |
| Manual config files | Ad-hoc project files | Manual — see table below |

---

## Bardo/Mori → Roko Migration

The Bardo/Mori system used different config keys, some of which map directly to
`roko.toml` and some of which have been renamed, split, or removed.

### Key Mapping Table

| Bardo/Mori key | `roko.toml` equivalent | Notes |
|----------------|------------------------|-------|
| `playbook.toml` file | `learn.playbook_path` | The playbook file is preserved as-is; only its path is now configured in `roko.toml` |
| `model` (top-level) | `agent.model` | |
| `max_turns` (top-level) | `agent.max_turns` | |
| `timeout` (seconds) | `agent.timeout_seconds` | Same semantics |
| `gateway_url` | `agent.base_url` | |
| `mcp.json` file | `agent.mcp_config` (points at `.mcp.json`) | File format is unchanged |
| `gates` (list) | `gate.pipeline` | Gate names may differ; see gate name mapping below |
| `max_retries` | `gate.max_retries` | |
| `episode_store` | `learn.episode_store` | |
| `cascade_router.enabled` | `learn.cascade_router` | Boolean key only |
| `data_dir` | `substrate.data_dir` | |

### Gate Name Mapping

| Bardo/Mori gate name | `roko.toml` gate name |
|----------------------|-----------------------|
| `cargo-check` | `compile` |
| `cargo-test` | `test` |
| `cargo-clippy` | `clippy` |
| `diff-check` | `diff` |
| `semantic-review` | `semantic` |
| `audit` | `security` |
| `fmt-check` | `format` |

### Removed/Renamed Keys

| Old key | Status | What to do |
|---------|--------|-----------|
| `mortality.enabled` | Removed | Lifecycle management is now in roko-orchestrator; not configurable via `roko.toml` |
| `daimon.affect_weight` | Removed | Daimon affect weights are not yet configurable (Built) |
| `succession.backup_path` | Renamed | Use `substrate.data_dir` for persistence |
| `gateway.provider_priority` | Removed | Provider failover is handled by the gateway, not `roko.toml` |
| `budget.max_usd` | Removed | Inference budget enforcement is planned but not yet in the schema |

---

## Roko v0.x → Current Migration

If you have a `roko.toml` from an earlier (pre-release) Roko version, some keys may have
been renamed or moved between tables:

| v0.x key | Current key | Notes |
|----------|-------------|-------|
| `[agent].turns` | `agent.max_turns` | Renamed for clarity |
| `[agent].api_key` | Removed | API keys must now be in environment variables, never in `roko.toml` |
| `[agent].gateway` | `agent.base_url` | Renamed |
| `[learning]` table | `[learn]` table | Table name shortened |
| `[learning].router` | `learn.cascade_router` | Renamed |
| `[storage]` table | `[substrate]` table | Renamed to match Substrate trait |
| `[storage].path` | `substrate.data_dir` | Renamed |

---

## Automated Migration

```bash
# Detect and migrate automatically
roko migrate

# Specify source format
roko migrate --from bardo-playbook --output roko.toml

# Dry run (show what would be written, do not write)
roko migrate --dry-run
```

The migration command:

1. Reads the old config file.
2. Maps old keys to new keys using the table above.
3. Warns about removed keys (those with no equivalent).
4. Writes a new `roko.toml` with the translated values.
5. Does **not** delete the old config file — it renames it to `<name>.bak`.

After migration, run `roko config show` to verify the result and `roko config validate`
to confirm there are no remaining errors.

---

## Manual Migration Checklist

If you are migrating by hand:

- [ ] Move all API keys out of the config file and into environment variables.
- [ ] Rename `[learning]` to `[learn]` (if present).
- [ ] Rename `[storage]` to `[substrate]` (if present).
- [ ] Rename any old gate names to their current equivalents (see table above).
- [ ] Check for removed keys and remove them from the new file.
- [ ] Run `roko config show` and verify all values are as expected.
- [ ] Run `roko config validate` and confirm zero errors.

---

## See Also

- [10-config-validation.md](10-config-validation.md) — understanding validation errors after migration
- [01-roko-toml-schema.md](01-roko-toml-schema.md) — current schema reference
- [13-security-considerations.md](13-security-considerations.md) — why API keys must not be in the config file

## Open Questions

- The automated `roko migrate` command is specified but not yet fully implemented — the table-rename transformations work; the key-rename transformations for v0.x are pending.
- Migration from third-party frameworks (LangChain config, AutoGen config) is not yet planned.
