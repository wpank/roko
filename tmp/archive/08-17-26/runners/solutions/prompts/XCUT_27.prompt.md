# XCUT_27: Improve Dockerfile with Multi-Stage Caching

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#xcut-27`](../ISSUE-TRACKER.md#xcut-27)
- Source: `tmp/solutions/roko/tasks/19-CROSS-CUTTING.md` — Task 19.27
- Priority: **P3**
- Effort: 2 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: XCUT_27 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

The current Dockerfile is minimal: 23 lines, single builder stage that copies everything (`COPY . .`) before building, invalidating Docker layer cache on every source change. It runs as root in the runtime stage. No health check. The `.dockerignore` exists and is comprehensive (67 lines), which is good.

Current Dockerfile:
```dockerfile
FROM rust:1.91-bookworm AS builder
WORKDIR /app
COPY . .
RUN cargo build --release -p roko-cli
FROM debian:bookworm-slim AS runtime
# ... apt-get, COPY binary, VOLUME, ENTRYPOINT
```

## Exact Changes

1. Add a dependency-cache stage that copies only `Cargo.toml` and `Cargo.lock` first:
   ```dockerfile
   FROM rust:1.91-bookworm AS deps
   WORKDIR /app
   COPY Cargo.toml Cargo.lock ./
   COPY crates/*/Cargo.toml ./crates/
   RUN find crates -name Cargo.toml -exec sh -c 'mkdir -p $(dirname {})/src && echo "" > $(dirname {})/src/lib.rs' \;
   RUN cargo build --release -p roko-cli 2>/dev/null || true
   ```
2. Add the source copy stage that benefits from cached dependencies:
   ```dockerfile
   FROM deps AS builder
   COPY . .
   RUN cargo build --release -p roko-cli
   ```
3. Add non-root user to the runtime stage:
   ```dockerfile
   RUN groupadd -r roko && useradd -r -g roko roko
   USER roko
   ```
4. Add health check: `HEALTHCHECK --interval=30s CMD curl -f http://localhost:6677/api/health || exit 1`.
5. Verify `.dockerignore` covers `target/`, `.roko/`, `tmp/`, `.git/` (it does -- confirmed).

## Write Scope

_None — this is a documentation/verification-only batch._

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/19-CROSS-CUTTING.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] Source-only changes rebuild in < 3 minutes (dependency cache hit)
- [ ] Runtime image runs as non-root user `roko`
- [ ] Health check passes within 30 seconds of container start
- [ ] `docker build .` succeeds
- [ ] Image size is reasonable (< 200MB for runtime stage)

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: XCUT_27 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Source-only changes rebuild in < 3 minutes (dependency cache hit)
- Runtime image runs as non-root user `roko`
- Health check passes within 30 seconds of container start
- `docker build .` succeeds
- Image size is reasonable (< 200MB for runtime stage)
- No files outside the Write Scope are modified.
- Commit message contains `tracker: XCUT_27 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
