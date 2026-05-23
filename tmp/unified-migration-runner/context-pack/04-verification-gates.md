# Verification Gates

## Per-batch (after every batch)

```bash
cargo check -p <affected-crates>
```

This is the minimum. Must pass before committing.

## Every 3 batches (review pass)

```bash
cargo clippy --workspace --no-deps -- -D warnings
cargo test --workspace --no-run
```

These run as part of the Codex review pass. Failures are fixed by the review agent.

## Full gates (before merge to source)

```bash
cargo +nightly fmt --all --check
cargo clippy --workspace --no-deps -- -D warnings
cargo test --workspace
```

All three must pass before any merge to source branch.

## Cargo artifact management

- Each batch uses a separate `CARGO_TARGET_DIR` in `/tmp/`
- Target dirs are cleaned after each batch verifies
- Full cleanup every 5 batches to prevent disk fill
- Never commit `target/` or `.cargo-target/` directories

## Failure protocol

1. If `cargo check` fails: the batch has a compilation error. Retry with failure context.
2. If `cargo clippy` fails: the batch has lint issues. Review pass fixes these.
3. If `cargo test` fails: a test is broken. Review pass fixes or the batch retries.
4. If a merge to source fails: STOP. All subsequent batches pause. Manual resolution required.

## What "verified" means

A batch is verified when:
- All `batch_verify_commands()` pass
- Changes are committed to the worktree branch
- No build artifacts are in the commit
