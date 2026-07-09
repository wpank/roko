# PERF_25: WarmPoolConfig in `roko.toml` Schema

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#perf-25`](../ISSUE-TRACKER.md#perf-25)
- Source: `tmp/solutions/roko/tasks/10-PERFORMANCE.md` — Task 10.25
- Priority: **??**
- Effort: ?
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: PERF_25 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Add `[conductor.warm_pool]` config section to the TOML schema.

## Exact Changes

1. Add `WarmPoolConfig` struct to config:
   ```rust
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct WarmPoolTomlConfig {
       pub enabled: bool,
       pub max_warm_slots: usize,
       pub max_active: usize,
       pub idle_timeout_secs: u64,
       pub pre_warm_on_serve: bool,
       pub pre_warm_providers: Vec<String>,
       pub pre_warm_models: Vec<String>,
   }
   ```
2. Add `pub warm_pool: Option<WarmPoolTomlConfig>` to `[conductor]` section
3. Implement `Default`: enabled=true, max_warm_slots=4, max_active=8,
   idle_timeout_secs=300, pre_warm_on_serve=true
4. Validate: max_warm_slots <= 16, idle_timeout_secs >= 30
5. Wire so `roko config show` displays warm pool config

## Write Scope

- `crates/roko-core/src/config/mod.rs`
- `roko.toml`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/10-PERFORMANCE.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] `roko config show` includes warm pool config
- [ ] `roko config validate` accepts valid config
- [ ] Missing `[conductor.warm_pool]` uses defaults (backwards compatible)

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: PERF_25 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `roko config show` includes warm pool config
- `roko config validate` accepts valid config
- Missing `[conductor.warm_pool]` uses defaults (backwards compatible)
- No files outside the Write Scope are modified.
- Commit message contains `tracker: PERF_25 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
