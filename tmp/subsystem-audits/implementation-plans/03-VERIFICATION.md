# 03 — Verification Protocol

This file describes how to **prove** a task in this folder is done. Every
plan ends with a "Verify" section that should be runnable without prior
knowledge of the task.

## Standard Verification Loop

```bash
# 1. Pre-commit (must pass with no warnings)
cargo +nightly fmt --all
cargo clippy --workspace --no-deps -- -D warnings
cargo test --workspace

# 2. Per-task verification (varies; each plan specifies)
cargo test -p <crate> <test_pattern> --lib
rg '<symbol_or_string>' crates/ -g '*.rs'   # absence/presence claims

# 3. Static checks (all tasks)
git diff --check                             # no whitespace damage
git status                                   # only intended files modified
bash -n scripts/roko-fitness-checks.sh
bash scripts/roko-fitness-checks.sh           # exits 0; new findings reviewed
```

## Verification by Task Class

### "Code is gone" tasks (T2-*)

Use ripgrep to assert absence:

```bash
rg '<symbol or filename>' crates/ -g '*.rs'           # no hits
rg '<symbol>' tests/ -g '*.rs'                        # no test references
ls crates/<path>/<file>.rs                            # `ls: not found`
```

Then build clean: `cargo build --workspace --all-features`.

### "Field/sink is wired" tasks (T4-*, plan 25)

1. Construct the wiring in production code (not just a test).
2. Add an integration test that exercises the wired path end-to-end.
3. Use ripgrep to assert the new symbol appears in product code, not only
   tests:

```bash
rg 'with_ingestor' crates/roko-cli/src/commands/    # must match (product use)
rg 'with_ingestor' crates/roko-cli/src/runtime_feedback/  # also matches (definition)
```

### "Path is migrated" tasks (T5-36, T5-37, plan 22)

The migration is done iff:

- No raw provider HTTP construction remains in the migrated route/module.
  - `rg 'reqwest::Client::new' crates/<path>/` returns nothing for that route.
- The route uses `state.model_call_service` (or equivalent).
- A focused test exercises the new path with a stub provider.
- The fitness inventory script reports the migrated route as "no
  raw HTTP."

### "Validation is enforced" tasks (T1-12, plan 23)

- Construct an invalid input and assert a typed error is returned.
- Construct a valid input and assert success.
- Add a regression test for the specific bypass that was discovered.

### "Default is restrictive" tasks (T1-15, plan 28)

- Construct the default with no overrides; assert the restrictive contract
  is in effect (specific tool denied).
- Add a permissive test fixture; assert the same call now succeeds.
- Verify the production constructor uses the restrictive path (`grep`).

### "Architecture extracted" tasks (T5-35, plan 20)

Each slice produces:

- A new module file with the extracted unit.
- The original function calls into the new module.
- The original function shrinks by exactly the extracted lines.
- All tests still pass.
- A wc -l before/after diff matches the extracted unit's size.

```bash
# Before extraction
wc -l crates/roko-cli/src/orchestrate.rs

# Extract slice (one commit)

# After extraction
wc -l crates/roko-cli/src/orchestrate.rs
wc -l crates/roko-cli/src/orchestrate/<new_module>.rs
# Sum is approximately the same; tests still pass
cargo test --workspace
```

### "Configuration field is removed" tasks (T2-18, T2-19, T2-21)

- Field absent from struct → `rg '<field>' crates/roko-core/src/config/` empty.
- Field absent from `roko.toml` → `grep '<field>' roko.toml` empty.
- TUI display absent → `rg '<field>' crates/roko-cli/src/tui/` empty.
- `cargo check --workspace` clean.
- `cargo test --workspace` clean.

## Reporting Format

When you finish a task, report in this format:

```
Task: T2-16 — Delete 4 orphan learn files
Status: Done
Files deleted:
  crates/roko-learn/src/resonant_patterns.rs (152 LOC)
  crates/roko-learn/src/signal_metabolism.rs (203 LOC)
  crates/roko-learn/src/shapley.rs (88 LOC)
  crates/roko-learn/src/kalman.rs (174 LOC)
Verification:
  $ rg 'mod (resonant_patterns|signal_metabolism|shapley|kalman)' crates/roko-learn/
    (no matches)
  $ cargo check -p roko-learn
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 4.20s
  $ cargo test -p roko-learn
    test result: ok. 47 passed; 0 failed; 0 ignored
Net LOC: -617
Commit: abc1234
Anti-patterns checked:
  - No new dispatch path
  - No unrelated edits
  - One item per commit
```

If a task partially landed, report exactly what landed and what didn't, and
why. Do not mark `[x]` on partial work; use `[~]` and add a follow-up note.

## When Verification Fails

- **Test fails after change**: the change broke a contract. Find which
  invariant. If the invariant was wrong, fix it in a separate commit; do
  not bundle.
- **Clippy warns**: fix the warning. Do not silence with `#[allow]` unless
  the plan explicitly authorizes it.
- **`cargo fmt` reformats other files**: revert the unrelated formatting
  changes. Only commit format changes to files you actually edited.
- **Fitness inventory regresses**: a new violation appeared. Find the cause.
  Fitness violations are blocking unless explicitly allowlisted with owner /
  reason / expiry.

## Promoting Inventory to Blocking

Plan 27 (CI fitness checks) describes how to convert
`scripts/roko-fitness-checks.sh` from inventory mode to no-new-violations
mode. Once promoted, the `git diff` between current and allowlist findings
becomes the verification criterion: a new finding fails CI; an existing
finding passes only if allowlisted with rationale.
