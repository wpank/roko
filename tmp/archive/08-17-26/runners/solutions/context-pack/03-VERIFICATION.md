# Verification Recipes

Every batch ends with a `Verify` block. The recipes here are the building
blocks. Use ripgrep (`rg`) — never `grep -r` — and never run cargo from
inside the batch (the merge-back pipeline does that).

## Standard recipe (most batches)

```bash
# 1. Confirm the targeted change landed
rg '<expected-string-after-change>' <file>

# 2. Confirm the bad pattern is gone
rg '<expected-string-before-change>' <file>   # should be empty (use --quiet)

# 3. Confirm no neighbours were touched
git diff --stat   # should mention only the listed write scope
```

## Pattern: replace-this-with-that

```bash
# After change: the new text appears
rg --quiet 'new pattern' path/to/file.rs && echo OK || echo MISSING

# After change: the old text is gone
! rg --quiet 'old pattern' path/to/file.rs && echo CLEAN || echo LEFTOVER
```

## Pattern: remove unreachable / panic / unwrap

```bash
# Confirm no unreachable in the touched function
rg 'unreachable!' crates/<crate>/src/<file>.rs
# Should be empty (or only in unrelated functions)

# Confirm no panic / unwrap added
rg 'panic!|unwrap\(\)' crates/<crate>/src/<file>.rs | grep -v test
```

## Pattern: wire-call-must-exist

```bash
# Confirm the new call site exists
rg 'expected_function_name\(' crates/<crate>/src/<file>.rs

# Confirm the function being called is actually defined and pub
rg 'pub(\(crate\))? fn expected_function_name' crates/
```

## Pattern: count anti-pattern occurrences shrinking

```bash
# Before fix (capture baseline)
rg 'reqwest::Client::new\(\)' crates/ | wc -l   # e.g. 6

# After fix
rg 'reqwest::Client::new\(\)' crates/ | wc -l   # should be 5 (or fewer)
```

## Pattern: route auth check

```bash
# Confirm a route is mounted inside the auth router (line 100..120 of mod.rs)
rg 'shared_runs::auth_routes' crates/roko-serve/src/routes/mod.rs

# And NOT in the public router
rg 'shared_runs::auth_routes' crates/roko-serve/src/routes/mod.rs | rg -v 'public'
```

## Pattern: config field consumed

```bash
# Field exists in struct
rg 'pub workflow_template' crates/roko-core/src/config/

# Field actually read at runtime (not just deserialized)
rg 'workflow_template' crates/roko-cli/src/runner/
```

## Pattern: tracker checklist

After landing, the commit message MUST include:

```
tracker: <BATCH_ID> done <short-sha-or-blank>
```

Confirm via:

```bash
git log -1 --format=%B | rg "^tracker: $BATCH_ID done"
```

## What NOT to verify

- Do **not** run `cargo check`, `cargo build`, `cargo test`, or `cargo clippy`.
- Do **not** run `npx`, `npm`, or `yarn` in the Rust crates (use `yarn`
  only inside `demo/demo-app` and only when explicitly required by a
  batch).
- Do **not** start `roko serve` to test routes — write a unit test instead.

## Pre-commit pipeline (runs after merge-back)

```bash
cargo +nightly fmt --all
cargo clippy --workspace --no-deps -- -D warnings
cargo test --workspace
```

If your batch breaks any of these, the merge is reverted. Read the source
twice; do not compile.
