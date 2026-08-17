# CONF_24: Wire Secret Resolution Through Provider Config

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#conf-24`](../ISSUE-TRACKER.md#conf-24)
- Source: `tmp/solutions/roko/tasks/16-CONFIG-AND-WIRING.md` — Task 16.24
- Priority: **P2**
- Effort: Medium
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: CONF_24 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`ProviderConfig::resolve_api_key()` at `crates/roko-core/src/config/provider.rs:76`
only checks `api_key_env` -> env var. It does not support:
- Inline `api_key` field (for testing/simple setups)
- Profile-aware secrets store (`roko config secrets`)

Some code reads env var names from config and resolves them. Other code hardcodes the
env var name directly. The `config secrets` subcommands exist but the resolution path
is inconsistent.

## Exact Changes

1. Extend `ProviderConfig::resolve_api_key()` to check in order:
   - `self.api_key` (inline key, if field is added)
   - `self.api_key_env` -> `std::env::var(name)`
   - Profile-aware secrets store (if available)
2. All provider adapter constructors use `provider_config.resolve_api_key()`.
3. `roko config check-secrets` verifies all configured providers have resolvable keys.
4. `roko config providers health` calls `resolve_api_key()` and reports status.

## Write Scope

- `crates/roko-core/src/config/provider.rs`
- `crates/roko-cli/src/commands/config_cmd.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/16-CONFIG-AND-WIRING.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] `roko config providers health` shows green/red status for each provider's key.
- [ ] A provider with `api_key_env = "CUSTOM_KEY"` resolves when `CUSTOM_KEY` is set.

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: CONF_24 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `roko config providers health` shows green/red status for each provider's key.
- A provider with `api_key_env = "CUSTOM_KEY"` resolves when `CUSTOM_KEY` is set.
- No files outside the Write Scope are modified.
- Commit message contains `tracker: CONF_24 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
