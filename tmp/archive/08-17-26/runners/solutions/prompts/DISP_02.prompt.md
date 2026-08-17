# DISP_02: Thread CascadeRouter Through resolve_effective_model_key

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#disp-02`](../ISSUE-TRACKER.md#disp-02)
- Source: `tmp/solutions/roko/tasks/03-INFERENCE-DISPATCH.md` — Task 3.2
- Priority: **P0**
- Effort: 3 hours
- Depends on: `DISP_01` (source 3.1)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: DISP_02 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`resolve_effective_model_key()` at `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/model_selection.rs:184-196` is the convenience wrapper called by CLI command handlers. It currently hardcodes `None` for the cascade_router parameter at line 192:
```rust
resolve_effective_model(cli_model, None, role.map(str::to_string), None, &config)
```

Call sites (found via grep):
- `commands/plan.rs:559` -- `roko plan generate`
- `commands/plan.rs:608` -- `roko plan regenerate`
- `commands/prd.rs:351` -- `roko prd plan`
- `commands/prd.rs:672` -- `roko prd draft`
- `commands/config_cmd.rs:352` -- `roko config models route` (uses `resolve_effective_model` directly)
- `commands/config_cmd.rs:649` -- `roko config models list` (uses `resolve_effective_model` directly)

## Exact Changes

1. Change `resolve_effective_model_key()` signature from:
   ```rust
   pub fn resolve_effective_model_key(
       workdir: &Path, cli_model: Option<String>, role: Option<&str>, context: &str,
   ) -> anyhow::Result<String>
   ```
   to:
   ```rust
   pub fn resolve_effective_model_key(
       workdir: &Path, cli_model: Option<String>, role: Option<&str>, context: &str,
       cascade_router: Option<&CascadeRouter>,
   ) -> anyhow::Result<String>
   ```
2. Forward `cascade_router` to `resolve_effective_model()` instead of hardcoded `None`
3. Update all 4 call sites in `commands/plan.rs` and `commands/prd.rs`:
   - At each call site, the `CascadeRouter` is already loaded nearby (e.g., `plan.rs:322-323` loads the router). Pass a reference.
   - Where no router is loaded yet, call `load_cascade_router(workdir, &config)` and pass `Some(&router)`
4. For `config_cmd.rs` call sites (which call `resolve_effective_model` directly), add `Some(&router)` where the router is loaded at line 687
5. Run `cargo test -p roko-cli` to verify no regressions

## Design Guidance

Adding the parameter to the convenience wrapper means every CLI command that resolves a model can benefit from adaptive routing with zero extra wiring per command. Pass `None` only in tests or where config is not available.

## Write Scope

- `crates/roko-cli/src/model_selection.rs`
- `crates/roko-cli/src/commands/plan.rs`
- `crates/roko-cli/src/commands/prd.rs`
- `crates/roko-cli/src/commands/config_cmd.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/03-INFERENCE-DISPATCH.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] `grep -n 'resolve_effective_model_key' crates/roko-cli/src/ -r` shows `cascade_router` parameter at all call sites
- [ ] `SelectionSource::CascadeRouter` variant is now reachable (verify with a test that passes a router with observations)
- [ ] Existing tests pass unchanged (callers that don't have a router pass `None`)

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: DISP_02 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `grep -n 'resolve_effective_model_key' crates/roko-cli/src/ -r` shows `cascade_router` parameter at all call sites
- `SelectionSource::CascadeRouter` variant is now reachable (verify with a test that passes a router with observations)
- Existing tests pass unchanged (callers that don't have a router pass `None`)
- No files outside the Write Scope are modified.
- Commit message contains `tracker: DISP_02 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
