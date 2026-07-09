# ACPM_36: Define TrackerAdapter Trait

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#acpm-36`](../ISSUE-TRACKER.md#acpm-36)
- Source: `tmp/solutions/roko/tasks/09-ACP-MCP.md` — Task 9.36
- Priority: **P2**
- Effort: 3 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: ACPM_36 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Roko needs a generalized adapter trait for bidirectional task sync with external systems (GitHub Issues, Sentry, Linear). The trait should be object-safe for dynamic dispatch.

## Exact Changes

1. Define the trait in `tracker.rs`:
   ```rust
   #[async_trait]
   pub trait TrackerAdapter: Send + Sync {
       fn kind(&self) -> &str;
       async fn fetch_active(&self) -> Result<Vec<ExternalTask>>;
       async fn update_state(&self, id: &str, state: &str, comment: Option<&str>) -> Result<()>;
       async fn create_task(&self, spec: &TaskSpec) -> Result<String>;
       fn state_mapping(&self) -> &StateMapping;
   }
   ```
2. Define `ExternalTask { id, title, description, state, labels, url, assignee, metadata: HashMap<String, String> }`.
3. Define `StateMapping { pending, in_progress, completed, failed }` mapping Roko states to tracker-specific strings.
4. Define `TaskSpec { title, description, labels: Vec<String>, assignee: Option<String> }`.
5. Add `pub mod tracker;` to `lib.rs`.
6. Add `async-trait` dependency to `Cargo.toml` if not already present.

## Write Scope

- `crates/roko-core/src/lib.rs`
- `crates/roko-core/Cargo.toml`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/09-ACP-MCP.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] Trait compiles and is object-safe (`Box<dyn TrackerAdapter>`)
- [ ] `StateMapping` covers all Roko task states
- [ ] Types derive `Serialize`, `Deserialize`, `Debug`, `Clone`

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: ACPM_36 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Trait compiles and is object-safe (`Box<dyn TrackerAdapter>`)
- `StateMapping` covers all Roko task states
- Types derive `Serialize`, `Deserialize`, `Debug`, `Clone`
- No files outside the Write Scope are modified.
- Commit message contains `tracker: ACPM_36 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
