# Configuration

> All configuration that controls Roko's behaviour lives in `roko.toml`. This folder is
> the complete reference for that file, plus the env-var overrides, CLI flag precedence,
> `.mcp.json` tool discovery, and secrets handling that operators need in production.

**Status**: Shipping
**Crate**: `roko-cli`, `roko-orchestrator`, `roko-runtime`
**Depends on**: [operations/README.md](../README.md)
**Last reviewed**: 2026-04-19

---

## Contents

| # | Page | What it covers | Status |
|---|------|----------------|--------|
| 00 | [Overview](00-overview.md) | `roko.toml` as the unified surface; `roko init` workflow | Shipping |
| 01 | [Schema Reference](01-roko-toml-schema.md) | Every table and key: type, default, range, example | Shipping |
| 02 | [Agent Config](02-agent-config.md) | `[agent]` table | Shipping |
| 03 | [Gate Config](03-gate-config.md) | `[gate]` table | Shipping |
| 04 | [Learn Config](04-learn-config.md) | `[learn]` table | Shipping |
| 05 | [Substrate Config](05-substrate-config.md) | `[substrate]` table | Shipping |
| 06 | [Bus Config](06-bus-config.md) | `[bus]` table (target-state) | Specified |
| 07 | [MCP Config](07-mcp-config.md) | `.mcp.json` discovery | Shipping |
| 08 | [Environment Variables](08-environment-variables.md) | Env-var overrides | Shipping |
| 09 | [CLI Flag Precedence](09-cli-flag-precedence.md) | Override chain | Shipping |
| 10 | [Validation](10-config-validation.md) | Parse-time checks; error messages | Shipping |
| 11 | [Migration](11-config-migration.md) | Old-format to `roko.toml` conversion | Shipping |
| 12 | [Examples](12-examples.md) | Ready-to-use profiles | Shipping |
| 13 | [Security](13-security-considerations.md) | Secrets, `.roko/` layout | Shipping |

## Suggested reading order

First deployment: `00` → `12` (pick a profile) → `08` (env vars for secrets) → `13` (security).
Tuning an existing deployment: `01` (full schema) → the specific table page for the subsystem you are tuning.
Upgrading from an older config: `11` (migration) → `01` (check new keys).

## See Also

- [`operations/performance/`](../performance/README.md) — performance tunables reference back to config keys
- [`operations/error-handling/04-crash-recovery.md`](../error-handling/04-crash-recovery.md) — resume flag that pairs with `roko.toml`
- [`guides/quickstart.md`](../../guides/quickstart.md) — end-to-end setup walkthrough
