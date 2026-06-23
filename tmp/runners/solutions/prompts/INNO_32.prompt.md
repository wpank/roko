# INNO_32: Implement domain detection for projects

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#inno-32`](../ISSUE-TRACKER.md#inno-32)
- Source: `tmp/solutions/roko/tasks/11-INNOVATIONS.md` — Task 11.32
- Priority: **P2**
- Effort: 4 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: INNO_32 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

_(no context section in source)_

## Exact Changes

1. Define `DomainTag` enum: `Rust`, `TypeScript`, `JavaScript`, `Python`,
   `Go`, `Blockchain`, `React`, `WebApp`, etc.
2. Implement `detect_domains(workdir: &Path) -> Vec<DomainTag>`:
   - `Cargo.toml` -> Rust
   - `package.json` -> JavaScript
   - `tsconfig.json` -> TypeScript
   - `pyproject.toml` or `requirements.txt` -> Python
   - `go.mod` -> Go
   - `foundry.toml` -> Blockchain
3. Cache the result for the session.
4. Make domain tags available to dispatch path and memory layer.

## Write Scope

- `crates/roko-cli/src/commands/`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/11-INNOVATIONS.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] In the roko project (Rust), `detect_domains()` returns `[Rust]`
- [ ] In a project with both `Cargo.toml` and `package.json`, returns both tags
- [ ] Domain tags are logged in verbose output

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: INNO_32 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- In the roko project (Rust), `detect_domains()` returns `[Rust]`
- In a project with both `Cargo.toml` and `package.json`, returns both tags
- Domain tags are logged in verbose output
- No files outside the Write Scope are modified.
- Commit message contains `tracker: INNO_32 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
