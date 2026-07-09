# CONF_08: Replace Direct `PERPLEXITY_API_KEY` Reads

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#conf-08`](../ISSUE-TRACKER.md#conf-08)
- Source: `tmp/solutions/roko/tasks/16-CONFIG-AND-WIRING.md` — Task 16.8
- Priority: **P3**
- Effort: Small
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: CONF_08 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

8 locations read `PERPLEXITY_API_KEY` directly from the environment:

- `crates/roko-std/src/tool/builtin/web_search.rs:332,403`
- `crates/roko-cli/src/orchestrate.rs:4473,4692,4904,17368` (legacy, lower priority)
- `crates/roko-cli/src/chat_inline.rs:3101` (capability check)
- `crates/roko-cli/src/commands/research.rs:724`

## Exact Changes

1. Add a helper: `resolve_provider_api_key(config: &RokoConfig, provider_name: &str)
   -> Option<String>` that checks `config.providers[provider_name].resolve_api_key()`
   first, then falls back to the well-known env var.
2. In `web_search.rs`, accept the key via config at tool registry construction time.
   Fall back to `std::env::var("PERPLEXITY_API_KEY")` only in tests or standalone usage.
3. In `commands/research.rs:724`, resolve from config first.
4. Log a deprecation warning when using the env var fallback in non-test code.

## Write Scope

- `crates/roko-std/src/tool/builtin/web_search.rs`
- `crates/roko-cli/src/commands/research.rs`

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

- [ ] `roko research search "test"` works when the Perplexity key is configured only in
- [ ] The env var fallback still works but logs a warning.

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: CONF_08 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `roko research search "test"` works when the Perplexity key is configured only in
- The env var fallback still works but logs a warning.
- No files outside the Write Scope are modified.
- Commit message contains `tracker: CONF_08 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
