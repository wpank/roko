# PERF_05: Safety Contract Caching

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#perf-05`](../ISSUE-TRACKER.md#perf-05)
- Source: `tmp/solutions/roko/tasks/10-PERFORMANCE.md` — Task 10.5
- Priority: **??**
- Effort: ?
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: PERF_05 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Cache `AgentContract` per role using a process-scoped
`LazyLock<Mutex<HashMap<String, Arc<AgentContract>>>>` so that repeated tool
dispatches within a turn do not re-load from embedded YAML assets.

## Exact Changes

1. In `safety/mod.rs`, add a `static CONTRACT_CACHE: LazyLock<Mutex<HashMap<String, Arc<AgentContract>>>>`
2. Create `fn cached_contract_for_role(role: &str) -> Arc<AgentContract>` that
   checks the cache first, falls back to `AgentContract::load_for_role()` on miss
3. In `contract_for_role()` (line 864), replace the direct
   `AgentContract::load_for_role(role)` call with `cached_contract_for_role(role)`
4. Cache is process-scoped and never invalidated (contracts are immutable during
   a process lifetime; restarts clear the static)
5. Preserve the existing fallback logic: role overrides still checked first,
   `RestrictedFallback` mode preserved for unknown roles

## Write Scope

- `crates/roko-agent/src/safety/mod.rs`
- `crates/roko-agent/src/safety/contract.rs`

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

- [ ] Unit test: two `contract_for_role("implementer")` calls -- second is instant
- [ ] Unit test: different roles return different contracts
- [ ] No change in external safety behavior

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: PERF_05 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Unit test: two `contract_for_role("implementer")` calls -- second is instant
- Unit test: different roles return different contracts
- No change in external safety behavior
- No files outside the Write Scope are modified.
- Commit message contains `tracker: PERF_05 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
