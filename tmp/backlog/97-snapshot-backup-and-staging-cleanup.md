# 97 — Snapshot Backup and Stale Staging Cleanup

**Priority**: P1 — data integrity gap; a corrupt snapshot is currently a hard crash with no recovery path
**Size**: S (1 day)
**Crates**: `crates/roko-cli` (runner), `crates/roko-fs`
**Depends on**: None

---

## Background

Every runner checkpoint writes a unified state snapshot to `.roko/state/state-snapshot.json` using an atomic write-tmp-rename pattern. The pattern is correct for crash safety (a partial write can never corrupt the target), but it provides no rollback: once the rename completes, the previous snapshot is gone. If the in-memory state that produced the new snapshot was itself corrupt (e.g. from a bug that cleared completed-task tracking), restarting the runner will load the bad snapshot and repeat the same mistake.

A second gap compounds this: `clean_stale_staging_files()` was implemented in `persist.rs` (line 348) to remove orphaned `.tmp.<PID>.<SEQ>` files left by crashed processes, but the function has zero callers. These files are small but accumulate permanently across every restart after a crash during an atomic write.

The third gap is the hard-crash behavior on load: if `load_state_snapshot()` returns an error (checksum mismatch, truncated JSON, schema version bump), the runner at line 1947 of `event_loop.rs` returns the error immediately via `return Err(err).context("load authoritative unified state snapshot")`. There is no attempt to fall back to a backup or to start fresh with a warning. This means any snapshot corruption blocks the runner entirely.

## Current State

1. **Atomic write has no backup step.** `crates/roko-fs/src/atomic.rs`, lines 200-227 — `atomic_write_bytes()` creates a unique sibling staging file, writes + syncs it, then renames it over the target path. The line `std::fs::rename(&tmp, path)?` at line 217 atomically replaces the old file with no copy of it preserved.

2. **`clean_stale_staging_files` is never called.** `crates/roko-cli/src/runner/persist.rs`, lines 341-388 — the function exists and correctly uses `kill -0` to detect dead PIDs, but a workspace-wide search confirms it has exactly one occurrence in the codebase (the definition itself). It is not called from `event_loop.rs`, `persist.rs`, or anywhere else.

3. **Corrupt snapshot is a hard error.** `crates/roko-cli/src/runner/event_loop.rs`, lines 1915-1948 — the `Err(err)` arm at line 1946 returns immediately:
   ```rust
   Err(err) => {
       return Err(err).context("load authoritative unified state snapshot");
   }
   ```
   The `Ok(None)` arm (lines 1929-1944) already handles a missing file gracefully (falls back to legacy `run-state.json`, then to no state). The `Err` arm has no equivalent fallback.

4. **`PersistPaths` exposes `state_snapshot_json` and `state_dir`.** `crates/roko-cli/src/runner/persist.rs`, lines 32-86 — `PersistPaths` stores `state_snapshot_json: PathBuf` (line 47) and is constructed from `workdir` by `from_workdir()`. A backup path is trivially derivable as `state_snapshot_json.with_extension("json.bak")`.

5. **`load_state_snapshot` and `save_state_snapshot` are the only I/O entry points.** `crates/roko-cli/src/runner/persist.rs`, lines 513-562 — these are the canonical functions. Adding backup logic here keeps it in one place.

6. **Runner startup is at `event_loop.rs` line 1718.** The `run()` function begins with config loading and extension initialization before reaching the snapshot load at line 1915. The staging cleanup call belongs early in `run()`, before JSONL recovery or snapshot loading, so orphaned files are cleaned before any new I/O begins.

## Implementation Plan

### Step 1: Add backup before overwrite in `save_state_snapshot`

In `crates/roko-cli/src/runner/persist.rs`, modify `save_state_snapshot` (lines 513-523) to rename the existing file to a `.bak` sibling before the atomic write:

```rust
pub fn save_state_snapshot(paths: &PersistPaths, snapshot: &StateSnapshot) -> Result<()> {
    let json = serde_json::to_vec_pretty(snapshot).context("serializing state snapshot")?;
    if json.len() as u64 > roko_runtime::MAX_DURABLE_RUNNER_PROJECTION_BYTES {
        anyhow::bail!(
            "state snapshot is {} bytes; maximum is {}",
            json.len(),
            roko_runtime::MAX_DURABLE_RUNNER_PROJECTION_BYTES
        );
    }
    // Best-effort backup: rename existing snapshot before overwriting.
    // This preserves one previous good checkpoint. Failure is non-fatal
    // because the write-tmp-rename below is crash-safe.
    let backup_path = backup_snapshot_path(paths);
    if paths.state_snapshot_json.exists() {
        let _ = std::fs::rename(&paths.state_snapshot_json, &backup_path);
    }
    atomic_write(&paths.state_snapshot_json, &json)
}

fn backup_snapshot_path(paths: &PersistPaths) -> PathBuf {
    paths.state_snapshot_json.with_extension("json.bak")
}
```

Also add a `load_state_snapshot_backup` function immediately after `load_state_snapshot`:

```rust
/// Load the backup state snapshot if it exists. Used for fallback on primary corruption.
pub fn load_state_snapshot_backup(paths: &PersistPaths) -> Result<Option<StateSnapshot>> {
    let backup = PersistPaths {
        state_snapshot_json: backup_snapshot_path(paths),
        ..paths.clone()
    };
    load_state_snapshot(&backup)
}
```

### Step 2: Call `clean_stale_staging_files` at runner startup

In `crates/roko-cli/src/runner/event_loop.rs`, inside `run()` (line 1718), add the cleanup call early in the function, after `PersistPaths::from_workdir` is called but before the JSONL recovery block (line ~1897). Find where `paths` is first constructed and add:

```rust
let cleaned = persist::clean_stale_staging_files(&paths.state_snapshot_json.parent().unwrap_or(&config.workdir));
if cleaned > 0 {
    info!(count = cleaned, "cleaned stale staging files from previous crash");
}
```

Note: `clean_stale_staging_files` takes a `&Path` to the directory to scan. Look at where `PersistPaths::from_workdir` is called in `event_loop.rs` and pass the state directory (`paths.state_snapshot_json.parent().unwrap()` or the RokoLayout state dir). Find the exact call site by searching for `PersistPaths::from_workdir` in `event_loop.rs`.

### Step 3: Fall back to backup on corrupt snapshot

In `crates/roko-cli/src/runner/event_loop.rs`, replace the `Err(err)` arm of the `load_state_snapshot` match (lines 1946-1948):

```rust
Err(err) => {
    warn!(
        error = %err,
        "unified state snapshot is corrupt; trying backup"
    );
    match persist::load_state_snapshot_backup(&paths) {
        Ok(Some(backup_snapshot)) => {
            warn!("loaded backup snapshot — most recent checkpoint may be lost");
            let run_state = serde_json::from_str(&backup_snapshot.run_state_json)
                .context("parse validated backup run_state_json")?;
            let loaded_gt = serde_json::from_str(&backup_snapshot.gate_thresholds_json)
                .context("parse validated backup gate_thresholds_json")?;
            (Some(run_state), Some(loaded_gt))
        }
        Ok(None) | Err(_) => {
            warn!("no valid backup snapshot found; starting fresh");
            (None, None)
        }
    }
}
```

### Step 4: Report stale staging files in `roko doctor disk`

The disk doctor is in `crates/roko-cli/src/commands/diagnose.rs` (the new file listed in git status) or `crates/roko-cli/src/commands/doctor.rs` — find the `disk` subcommand with `grep -rn "doctor.*disk\|disk.*doctor" crates/roko-cli/src/`. Add a check that counts files matching `*.tmp.*.*` in `.roko/state/` and reports the count.

### Step 5: Add tests

In `crates/roko-cli/src/runner/persist.rs`, add to the existing `#[cfg(test)]` block:

```rust
#[test]
fn save_state_snapshot_creates_backup() {
    // Write a snapshot, then write again; verify .bak exists with first content.
}

#[test]
fn load_state_snapshot_backup_returns_previous() {
    // Write snapshot, rename to .bak, verify backup loader returns it.
}
```

In `crates/roko-cli/src/runner/event_loop.rs` or integration tests, add:

```rust
// Write a corrupt state-snapshot.json, write a valid .bak, verify run() starts.
// Write orphaned .tmp.99999.0 files in state dir, verify cleanup count > 0 after run.
```

## Acceptance Criteria

1. After `save_state_snapshot` is called twice, a `.roko/state/state-snapshot.json.bak` file exists containing the content from the first call.
2. `clean_stale_staging_files` is called once during `run()` startup; logged at `info` level if count > 0.
3. If `state-snapshot.json` contains invalid JSON and `state-snapshot.json.bak` contains a valid snapshot, `run()` starts successfully with a `warn!` log rather than returning `Err`.
4. If both the primary and backup snapshots are corrupt, `run()` starts fresh (same behavior as no snapshot) with a `warn!` log.
5. `roko doctor disk` reports the count of stale staging files in `.roko/state/`.
6. All existing snapshot persistence tests in `persist.rs` pass.
7. New test: write corrupt primary + valid backup → runner falls back and starts.
8. New test: seed orphaned `.tmp.<dead-PID>.<seq>` files → `clean_stale_staging_files` returns count > 0 and files are removed.

## Verification Checklist

- [ ] `grep -n 'clean_stale_staging_files' crates/roko-cli/src/runner/event_loop.rs` shows a call site
- [ ] `ls .roko/state/*.bak` appears after the second checkpoint of a plan run
- [ ] `roko doctor disk` includes a "stale staging files" line
- [ ] `cargo test -p roko-cli runner::persist` passes all tests
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` passes clean

## Files to Modify

| File | Change |
|---|---|
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/persist.rs` | Add `backup_snapshot_path()`, modify `save_state_snapshot()` to rename before write, add `load_state_snapshot_backup()` |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs` | Add `clean_stale_staging_files` call early in `run()`, replace hard-crash `Err` arm with backup-then-fresh fallback |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/diagnose.rs` (or equivalent doctor file) | Add stale staging file count to disk report |
