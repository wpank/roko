# S-cog-4: Delete daimon crate

## Task
Delete `roko-daimon` (or the daimon modules within `roko-cognitive`). After S-cog-3, no production caller remains; the deletion is safe.

## Runner Context
Runner audit-2026-05-01, group S. Depends on S-cog-3. Wave 4.

## Source plan
`tmp/subsystem-audits/implementation-plans/31-cognitive-layer-cleanup.md` § CL-2 (deletion phase).

## Pre-deletion safety check (mandatory)

```bash
# No production callers (FatigueDetector caller list from S-cog-1 should
# have already been migrated or kept; double-check)
rg 'use roko_daimon|roko_daimon::' crates/ -g '*.rs' \
  | rg -v 'crates/roko-daimon/'

# If FatigueDetector callers remain and S-cog-1 marked them as kept,
# move FatigueDetector into roko-learn or another keeper crate first.
# This batch then proceeds with the rest of daimon.
```

If non-FatigueDetector callers remain, **stop**. The migration is incomplete.

## Exact changes

### 1. Move kept items (FatigueDetector) elsewhere

If `FatigueDetector` is in `roko-daimon` and must be kept:

```bash
# Move to roko-learn:
git mv crates/roko-daimon/src/fatigue_detector.rs crates/roko-learn/src/fatigue_detector.rs
```

Add `pub mod fatigue_detector;` to `crates/roko-learn/src/lib.rs`. Update callers:

```bash
# Replace use statements
rg 'use roko_daimon::FatigueDetector|roko_daimon::FatigueDetector' crates/ -l \
  | xargs sed -i '' 's/roko_daimon::FatigueDetector/roko_learn::fatigue_detector::FatigueDetector/g'
```

### 2. Delete the crate

```bash
git rm -r crates/roko-daimon
```

### 3. Remove from workspace `Cargo.toml`

```bash
rg 'roko-daimon' Cargo.toml
```

For each match, remove the workspace member declaration and any dep references.

### 4. Re-grep to confirm

```bash
rg 'roko_daimon|roko-daimon' crates/ Cargo.toml -g '*.rs' -g '*.toml'
# Expect: 0 hits
```

## Write Scope
- `crates/roko-daimon/` (delete entire tree)
- `Cargo.toml` (workspace member + deps)
- (FatigueDetector move target if applicable)

## Verify

```bash
ls crates/roko-daimon 2>&1
# Expect: "No such file or directory"

rg 'roko-daimon|roko_daimon' Cargo.toml crates/*/Cargo.toml
# Expect: 0 hits
```

## Acceptance Criteria

- `crates/roko-daimon/` deleted.
- Workspace `Cargo.toml` no longer references it.
- No remaining callers.
- FatigueDetector (if kept) lives in a non-daimon crate.

## Do NOT

- Do NOT delete `FatigueDetector` if it has callers.
- Do NOT bundle with S-cog-1/2/3/5.
- Do NOT add `[deprecated]` shim. Just delete.
- Do NOT skip the workspace `Cargo.toml` update.
