# XCUT_30: Add roko deploy docker Subcommand

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#xcut-30`](../ISSUE-TRACKER.md#xcut-30)
- Source: `tmp/solutions/roko/tasks/19-CROSS-CUTTING.md` — Task 19.30
- Priority: **P1**
- Effort: 3 hours
- Depends on: `XCUT_27` (source 19.27)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: XCUT_30 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Users who want to self-host need to manually build the Docker image, configure volumes, set environment variables, and manage the container. A `roko deploy docker` command should generate a ready-to-run configuration.

## Exact Changes

1. Add `roko deploy docker` subcommand that:
   - Builds the Docker image using the workspace Dockerfile.
   - Generates `docker-compose.prod.yml` with:
     - roko-serve container with configured ports, volumes, and env vars.
     - Auto-generated API key in `.env` file.
     - Restart policy: `unless-stopped`.
     - Log driver configuration: `json-file` with max-size and max-file.
   - Prints instructions: "Run: docker compose -f docker-compose.prod.yml up -d".
2. Add `--port` flag (default 6677).
3. Add `--data-dir` flag for `.roko/` volume mount location.
4. Add `--tls` flag that generates a self-signed cert and configures HTTPS.

## Write Scope

- `crates/roko-cli/src/commands/deploy.rs`

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

- [ ] `roko deploy docker` produces a `docker-compose.prod.yml` ready to run
- [ ] The generated compose file includes API auth, health check, restart policy, and log limits
- [ ] `docker compose -f docker-compose.prod.yml up -d` starts a working roko-serve instance
- [ ] `--tls` flag configures HTTPS with a self-signed certificate

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: XCUT_30 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `roko deploy docker` produces a `docker-compose.prod.yml` ready to run
- The generated compose file includes API auth, health check, restart policy, and log limits
- `docker compose -f docker-compose.prod.yml up -d` starts a working roko-serve instance
- `--tls` flag configures HTTPS with a self-signed certificate
- No files outside the Write Scope are modified.
- Commit message contains `tracker: XCUT_30 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
