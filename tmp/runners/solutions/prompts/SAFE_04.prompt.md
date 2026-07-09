# SAFE_04: Generate Default Contracts During `roko init`

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#safe-04`](../ISSUE-TRACKER.md#safe-04)
- Source: `tmp/solutions/roko/tasks/17-SAFETY-SECURITY.md` — Task 17.4
- Priority: **P1**
- Effort: ?
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: SAFE_04 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

: When `roko init` creates `.roko/`, also create `.roko/contracts/` with
copies of the 8 bundled contract YAML files. Add a `[safety]` section to the
generated `roko.toml`. Update `AgentContract::load_for_role()` to check the
project contract dir first, then fall back to the bundled asset.

## Exact Changes

1. In `init.rs`, after creating the `.roko/` layout, create `.roko/contracts/`
2. Embed the 8 YAML files from `crates/roko-agent/src/safety/contracts/` using
   `include_str!` or read from the crate's installed assets
3. Write each to `.roko/contracts/{role}.yaml`
4. Add to the generated `roko.toml`:
   ```toml
   [safety]
   contract_dir = ".roko/contracts"
   skip_permissions = false
   ```
5. Update `AgentContract::load_for_role()` in `contract.rs`:
   - Accept an optional `project_dir: Option<&Path>` parameter
   - Check `{project_dir}/.roko/contracts/{role}.yaml` first
   - Fall back to the bundled asset if not found
6. Or: add a new method `AgentContract::load_for_role_from_project(role, project_dir)`
   that tries the project path first

## Write Scope

- `crates/roko-cli/src/commands/init.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/17-SAFETY-SECURITY.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] `roko init` creates `.roko/contracts/implementer.yaml` (and 7 others)
- [ ] Editing `.roko/contracts/reviewer.yaml` to add `ForbiddenTools: ["bash"]` is
- [ ] If `.roko/contracts/` is missing (old projects), bundled contracts are used

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: SAFE_04 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `roko init` creates `.roko/contracts/implementer.yaml` (and 7 others)
- Editing `.roko/contracts/reviewer.yaml` to add `ForbiddenTools: ["bash"]` is
- If `.roko/contracts/` is missing (old projects), bundled contracts are used
- No files outside the Write Scope are modified.
- Commit message contains `tracker: SAFE_04 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
