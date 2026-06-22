# CONF_01: Make `auth_detect.rs` Respect `roko.toml` Provider Config

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#conf-01`](../ISSUE-TRACKER.md#conf-01)
- Source: `tmp/solutions/roko/tasks/16-CONFIG-AND-WIRING.md` — Task 16.1
- Priority: **P1**
- Effort: Medium
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: CONF_01 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`detect_auth()` at `crates/roko-cli/src/auth_detect.rs:66` scans env vars
(`ZAI_API_KEY`, `ANTHROPIC_API_KEY`, `OPENAI_API_KEY`) in a fixed priority order,
completely ignoring `[providers]`, `[agent].default_model`, and `[agent].default_backend`
from `roko.toml`. Setting `default_model = "cerebras-70b"` in config has zero effect
when `ANTHROPIC_API_KEY` exists in the environment.

Callers: `crates/roko-cli/src/unified.rs` (imports `detect_auth`).

## Exact Changes

1. Add `detect_auth_with_config(config: &RokoConfig) -> AuthMethod` that resolves
   from `config.agent.default_model` and `config.agent.default_backend` first,
   finding the matching provider in `config.providers`.
2. Only fall back to env var scanning when config has no explicit provider/model
   or when the configured provider's API key cannot be resolved.
3. Update all callers of `detect_auth()` to pass the loaded config when available.
4. Keep the zero-arg `detect_auth()` as the bootstrapping path for when no config exists.

## Write Scope

- `crates/roko-cli/src/auth_detect.rs`
- `crates/roko-cli/src/model_selection.rs`
- `crates/roko-cli/src/unified.rs`

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

- [ ] Setting `default_model = "cerebras-70b"` in roko.toml and running `roko run "hello"`
- [ ] With no roko.toml, `detect_auth()` still works via env var scanning.

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: CONF_01 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Setting `default_model = "cerebras-70b"` in roko.toml and running `roko run "hello"`
- With no roko.toml, `detect_auth()` still works via env var scanning.
- No files outside the Write Scope are modified.
- Commit message contains `tracker: CONF_01 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
